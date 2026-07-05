use crate::ScenarioValues;
use crate::behaviours::common;
use crate::behaviours::ui_refresh;
use holochain_wind_tunnel_runner::prelude::{
    AgentContext, HolochainAgentContext, HolochainRunnerContext, HookResult, ReportMetric,
};
use rave_engine::types::Actionable;
use rave_engine::types::{ProposalInput, UnitMap};
use std::collections::BTreeMap;
use std::str::FromStr;
use std::thread;
use std::time::Duration;
use wind_tunnel_unyt_scenario::ArcType;
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
    arc_type: ArcType,
) -> HookResult {
    // step 1 - wait for network init
    if !common::is_network_initialized(ctx, &arc_type)? {
        // return and wait for next iteration
        thread::sleep(Duration::from_secs(1));
        return Ok(());
    }

    // step 2 - handle incoming transactions
    {
        // Refresh the action list like the UI does.
        let actionable_transactions = ui_refresh::ui_action_list_refresh(ctx);

        // Log what this agent actually received, tagged with arc type. Zero-arc agents
        // that never see any actionables here can publish proposals but never observe
        // responses, so this is the first place to look when an arc type stays idle.
        log::info!(
            "[{arc_type}-arc] Agent {} | action list: {} proposals, {} commitments, {} accepts, {} rejects",
            ctx.get().cell_id().agent_pubkey(),
            actionable_transactions.proposal_actionable.len(),
            actionable_transactions.commitment_actionable.len(),
            actionable_transactions.accept_actionable.len(),
            actionable_transactions.reject_actionable.len(),
        );

        // measure sync lag for all newly seen transactions
        common::measure_sync_lag(ctx, &actionable_transactions, &arc_type);

        // emit proposal_round_trip_time for proposals that reached a terminal state
        measure_rejected_round_trip_time(ctx, &actionable_transactions, &arc_type);

        // for each reject actionable, call `create_reclaim_balance`
        common::create_reclaim_balance(ctx, actionable_transactions.reject_actionable);
        // for each accept actionable, call `create_receipt_for_accept`
        common::create_receipt_for_accept(ctx, actionable_transactions.accept_actionable);
        // handle incoming proposals (and counter-proposals)
        common::handle_proposals(ctx, actionable_transactions.proposal_actionable, &arc_type)?;
        // handle incoming commitments
        common::handle_commitments(
            ctx,
            actionable_transactions.commitment_actionable,
            &arc_type,
        )?;
    }

    // step 3 - check ledger and calculate spendable amount
    let spendable_amount = match common::get_spendable_amount(ctx)? {
        Some(amount) => amount,
        None => return Ok(()),
    };

    // step 4 - create proposals
    if spendable_amount > ZFuel::zero() {
        create_proposals(ctx, spendable_amount, &arc_type)?;
    } else {
        log::warn!(
            "[{arc_type}-arc] No spendable amount for agent {}",
            ctx.get().cell_id().agent_pubkey(),
        );
    }

    // step 5 - refresh watched transaction details (like the UI) and poll their status
    let watched = ctx.get().scenario_values.watched_transactions().clone();
    ui_refresh::ui_transaction_detail_refresh(ctx, &watched);
    common::poll_watched_transactions(ctx);

    // step 6 - get history
    common::poll_history(ctx);

    // Give other agents time to receive and respond to proposals.
    thread::sleep(Duration::from_secs(3));

    Ok(())
}

/// Emit `proposal_round_trip_time` for proposals that were rejected.
///
/// The proposer receives the `reject_actionable` for its own proposal (it is the party that
/// reclaims), so the reject's root proposal can be matched against `pending_proposals` here.
/// The accepted round-trip is recorded in `handle_commitments` instead, because the
/// proposer never sees the accept as actionable, as it goes to the committer, who receipts.
fn measure_rejected_round_trip_time(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    actionable: &Actionable,
    arc_type: &ArcType,
) {
    let reporter = ctx.runner_context().reporter();
    let agent_key = ctx.get().cell_id().agent_pubkey().to_string();

    let mut resolved = Vec::new();
    for tx in &actionable.reject_actionable {
        if let Some(proposal_hash) = common::find_root_proposal(tx)
            && let Some(created_at) = ctx
                .get()
                .scenario_values
                .pending_proposals
                .get(&proposal_hash)
        {
            reporter.add_custom(
                ReportMetric::new("proposal_round_trip_time")
                    .with_tag("agent", agent_key.clone())
                    .with_tag("outcome", "rejected")
                    .with_tag("arc", arc_type.as_tag())
                    .with_field("value", created_at.elapsed().as_secs_f64()),
            );
            resolved.push(proposal_hash);
        }
    }

    for id in &resolved {
        ctx.get_mut().scenario_values.pending_proposals.remove(id);
    }
}

fn create_proposals(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    spendable_amount: ZFuel,
    arc_type: &ArcType,
) -> anyhow::Result<()> {
    // Spend only a small slice of the spendable amount per round. A single proposal must
    // be small enough that several can be in flight at once without any one agent's spend
    // reaching its credit limit, so keep this well below 100.
    let spend_pct = ctx.get().scenario_values.config().spend_fraction_pct;

    ctx.collect_agents()?;
    let participating_agents = ctx.get().scenario_values.participating_agents().to_vec();
    if participating_agents.is_empty() {
        log::warn!(
            "[{arc_type}-arc] Agent {} | no participating agents to propose to",
            ctx.get().cell_id().agent_pubkey(),
        );
        return Ok(());
    }

    let spendable_amount = (spendable_amount * Fraction::new(spend_pct as i64, 100)?)?;
    let fraction = Fraction::new(participating_agents.len() as i64, 1)?;
    let amount_per_agent = (spendable_amount / fraction)?;
    // Bidirectional exchange: the proposer receives `unit 0` and sends a small amount of
    // `unit 1`. Sending value to the counterparty is what makes them create a receipt after
    // accepting, so this exercises the full commit -> accept -> receipt process. A proposal
    // that only receives never produces a receipt.
    let unit_1_sent = ZFuel::from_str("-1")?;
    let amount = UnitMap::load(BTreeMap::from([
        ("0".to_string(), amount_per_agent),
        ("1".to_string(), unit_1_sent),
    ]));

    log::info!(
        "[{arc_type}-arc] Agent {} | creating proposals to {} agents",
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
                log::info!("[{arc_type}-arc] Created proposal {proposal_hash} for {peer}");
                ctx.get_mut()
                    .scenario_values
                    .pending_proposals
                    .insert(proposal_hash, std::time::Instant::now());
            }
            Err(err) => {
                log::warn!("[{arc_type}-arc] Failed to create proposal for {peer}: {err}");
            }
        }
    }

    Ok(())
}
