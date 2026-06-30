use crate::ScenarioValues;
use crate::behaviours::common::{self, ProposalWeights};
use holochain_types::prelude::ActionHashB64;
use holochain_wind_tunnel_runner::prelude::{
    AgentContext, HolochainAgentContext, HolochainRunnerContext, HookResult, ReportMetric,
};
use rave_engine::types::Actionable;
use rave_engine::types::{ProposalInput, Transaction, TransactionType, UnitMap};
use std::collections::BTreeMap;
use std::thread;
use std::time::Duration;
use wind_tunnel_unyt_scenario::UnytScenarioValues as _;
use wind_tunnel_unyt_scenario::unyt_agent::UnytAgentExt;
use zfuel::fraction::Fraction;
use zfuel::fuel::ZFuel;

/// A participant both makes proposals to its peers and responds to the proposals and
/// commitments it receives. Because every agent plays both sides, value flows in both
/// directions and balances stay within the credit limit over a long run. With fixed
/// proposer and responder roles, one side only ever spends and runs out of credit.
pub fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    // step 1 - wait for network init
    if !common::is_network_initialized(ctx)? {
        // return and wait for next iteration
        return Ok(());
    }

    // step 2 - handle incoming transactions
    {
        let actionable_transactions = match ctx.unyt_get_actionable_transactions() {
            Ok(txs) => txs,
            Err(err) => {
                log::warn!("Failed to get actionable transactions (transient DHT issue): {err}");
                thread::sleep(Duration::from_secs(1));
                return Ok(());
            }
        };

        // measure sync lag for all newly seen transactions
        common::measure_sync_lag(ctx, &actionable_transactions);

        // emit proposal_round_trip_time for proposals that reached a terminal state
        measure_proposal_round_trip_time(ctx, &actionable_transactions);

        // for each reject actionable, call `create_reclaim_balance`
        common::create_reclaim_balance(ctx, actionable_transactions.reject_actionable);
        // for each accept actionable, call `create_receipt_for_accept`
        common::create_receipt_for_accept(ctx, actionable_transactions.accept_actionable);
        // handle incoming proposals (and counter-proposals)
        let weights = ProposalWeights::get_weights_from_env()?;
        common::handle_proposals(ctx, actionable_transactions.proposal_actionable, &weights)?;
        // handle incoming commitments
        common::handle_commitments(ctx, actionable_transactions.commitment_actionable)?;
    }

    // step 3 - check ledger and calculate spendable amount
    let spendable_amount = match common::get_spendable_amount(ctx)? {
        Some(amount) => amount,
        None => return Ok(()),
    };

    // step 4 - create proposals
    if spendable_amount > ZFuel::zero() {
        create_proposals(ctx, spendable_amount)?;
    } else {
        log::warn!(
            "No spendable amount for agent {}",
            ctx.get().cell_id().agent_pubkey(),
        );
    }

    // step 5 - poll transaction status
    common::poll_watched_transactions(ctx);

    // step 6 - get history
    common::poll_history(ctx);

    thread::sleep(Duration::from_secs(3));

    Ok(())
}

/// Emit `proposal_round_trip_time` for proposals that have reached a terminal state.
///
/// Checks `reject_actionable` and `accept_actionable` against `pending_proposals`.
/// Accept/reject transactions carry a different `id` than the original proposal, so we
/// walk each transaction's `history` to find the root proposal hash and match it against
/// `pending_proposals`. If a match is found, the elapsed time since proposal creation is
/// emitted as a metric, tagged with the outcome (`accepted` or `rejected`).
fn measure_proposal_round_trip_time(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    actionable: &Actionable,
) {
    let reporter = ctx.runner_context().reporter();
    let agent_key = ctx.get().cell_id().agent_pubkey().to_string();

    let mut resolved = Vec::new();

    for tx in &actionable.accept_actionable {
        if let Some(proposal_hash) = find_root_proposal(tx)
            && let Some(created_at) = ctx
                .get()
                .scenario_values
                .pending_proposals
                .get(&proposal_hash)
        {
            let rtt = created_at.elapsed().as_secs_f64();
            reporter.add_custom(
                ReportMetric::new("proposal_round_trip_time")
                    .with_tag("agent", agent_key.clone())
                    .with_tag("outcome", "accepted")
                    .with_field("value", rtt),
            );
            resolved.push(proposal_hash);
        }
    }

    for tx in &actionable.reject_actionable {
        if let Some(proposal_hash) = find_root_proposal(tx)
            && let Some(created_at) = ctx
                .get()
                .scenario_values
                .pending_proposals
                .get(&proposal_hash)
        {
            let rtt = created_at.elapsed().as_secs_f64();
            reporter.add_custom(
                ReportMetric::new("proposal_round_trip_time")
                    .with_tag("agent", agent_key.clone())
                    .with_tag("outcome", "rejected")
                    .with_field("value", rtt),
            );
            resolved.push(proposal_hash);
        }
    }

    for id in &resolved {
        ctx.get_mut().scenario_values.pending_proposals.remove(id);
    }
}

/// Walk a transaction's history tree to find the root proposal hash.
///
/// Accept and reject transactions carry their own action hash as `id`, not the original
/// proposal hash. The negotiation history is stored in `tx.history` as a chain of
/// `Transaction` entries. This function recursively searches for the deepest `Proposal`
/// entry, which is the original proposal that started the negotiation.
fn find_root_proposal(tx: &Transaction) -> Option<ActionHashB64> {
    // Depth-first: check children first so we find the *root* (earliest) proposal
    for child in &tx.history {
        if let Some(hash) = find_root_proposal(child) {
            return Some(hash);
        }
    }
    if tx.tx_type == TransactionType::Proposal {
        return Some(tx.id.clone());
    }
    None
}

fn create_proposals(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    spendable_amount: ZFuel,
) -> anyhow::Result<()> {
    // Spend only a small slice of the spendable amount per round. A single proposal must
    // be small enough that several can be in flight at once without any one agent's spend
    // reaching its credit limit, so keep this well below 100.
    let spend_pct: u8 = std::env::var("UNYT_SPEND_FRACTION_PCT")
        .unwrap_or_else(|_| "10".to_string())
        .parse()?;
    if spend_pct > 100 {
        anyhow::bail!("UNYT_SPEND_FRACTION_PCT must be between 0 and 100, got {spend_pct}");
    }

    ctx.collect_agents()?;
    let participating_agents = ctx.get().scenario_values.participating_agents().to_vec();
    if participating_agents.is_empty() {
        log::warn!("No participating agents to propose to");
        return Ok(());
    }

    let spendable_amount = (spendable_amount * Fraction::new(spend_pct as i64, 100)?)?;
    let fraction = Fraction::new(participating_agents.len() as i64, 1)?;
    let amount_per_agent = (spendable_amount / fraction)?;
    let amount = UnitMap::load(BTreeMap::from([("0".to_string(), amount_per_agent)]));

    log::info!(
        "Agent {} | creating proposals to {} agents",
        ctx.get().cell_id().agent_pubkey(),
        participating_agents.len()
    );

    for peer in participating_agents {
        match ctx.unyt_create_proposal(ProposalInput {
            counterparty: peer.clone(),
            amount: amount.clone(),
            lane_definitions: Vec::new(),
            note: None,
        }) {
            Ok(proposal_hash) => {
                log::info!("Created proposal {proposal_hash} for {peer}");
                ctx.get_mut()
                    .scenario_values
                    .pending_proposals
                    .insert(proposal_hash, std::time::Instant::now());
            }
            Err(err) => {
                log::warn!("Failed to create proposal for {peer}: {err}");
            }
        }
    }

    Ok(())
}
