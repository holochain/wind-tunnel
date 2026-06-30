mod weights;

use crate::ScenarioValues;
use holochain_wind_tunnel_runner::prelude::{
    AgentContext, HolochainAgentContext, HolochainRunnerContext, ReportMetric,
};
use rave_engine::types::Actionable;
use rave_engine::types::{
    AcceptInput, CommitmentToProposalInput, CounterProposalInput, Pagination, ReceiptInput,
    ReclaimInput, RejectInput, Transaction, UnitMap, WatchStatus,
};
use std::collections::BTreeMap;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;
use wind_tunnel_unyt_scenario::UnytScenarioValues as _;
use wind_tunnel_unyt_scenario::unyt_agent::UnytAgentExt;
use zfuel::fraction::Fraction;
use zfuel::fuel::ZFuel;

pub use self::weights::ProposalWeights;

/// For each reject actionable, call `unyt_create_reclaim_balance`.
pub fn create_reclaim_balance(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    rejected_txs: Vec<Transaction>,
) {
    for tx in rejected_txs {
        match ctx.unyt_create_reclaim_balance(ReclaimInput {
            rejection: tx.id.clone(),
            note: None,
        }) {
            Ok(_) => log::info!("Reclaimed balance for {}", tx.id),
            Err(err) => log::warn!("Failed to reclaim balance for {}: {err}", tx.id),
        }
    }
}

/// For each accepted actionable, call `unyt_create_receipt_for_accept`.
pub fn create_receipt_for_accept(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    accepted_txs: Vec<Transaction>,
) {
    for tx in accepted_txs {
        match ctx.unyt_create_receipt_for_accept(ReceiptInput {
            hash: tx.id.clone(),
            note: None,
        }) {
            Ok(_) => log::info!("Created receipt for accept {}", tx.id),
            Err(err) => log::warn!("Failed to create receipt for {}: {err}", tx.id),
        }
    }
}

/// Send counter-proposals with amounts adjusted by `UNYT_COUNTER_ADJUSTMENT_PCT`.
pub fn counter_proposals(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    transactions: Vec<Transaction>,
) -> anyhow::Result<()> {
    let pct: u8 = std::env::var("UNYT_COUNTER_ADJUSTMENT_PCT")
        .unwrap_or_else(|_| "10".to_string())
        .parse()?;

    if pct > 100 {
        anyhow::bail!("UNYT_COUNTER_ADJUSTMENT_PCT must be between 0 and 100, got {pct}");
    }

    for tx in transactions {
        let adjusted_amount = adjust_amount(&tx.amount, pct)?;
        match ctx.unyt_create_counter_proposal(CounterProposalInput {
            previous_proposal: tx.id.clone(),
            amount: adjusted_amount,
            note: None,
        }) {
            Ok(_) => log::info!(
                "Created counter-proposal for {} (adjusted by {pct}%)",
                tx.id
            ),
            Err(err) => log::warn!("Failed to create counter-proposal for {}: {err}", tx.id),
        }
    }

    Ok(())
}

/// Handle incoming proposals according to the given weights.
///
/// Splits proposals into three groups:
/// - accept: commit to the proposal → push to watched_transactions
/// - counter: send a counter-proposal back
/// - reject: reject the proposal
///
/// Proposals that have already exceeded `UNYT_MAX_NEGOTIATION_ROUNDS` are force-accepted
/// regardless of the weights to prevent infinite counter-proposal ping-pong.
pub fn handle_proposals(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    proposals: Vec<Transaction>,
    weights: &ProposalWeights,
) -> anyhow::Result<()> {
    let max_rounds: usize = std::env::var("UNYT_MAX_NEGOTIATION_ROUNDS")
        .unwrap_or_else(|_| "5".to_string())
        .parse()?;

    // Separate proposals that exceeded max rounds — these are force-accepted
    let (over_limit, within_limit): (Vec<_>, Vec<_>) = proposals
        .into_iter()
        .partition(|tx| tx.history.len() >= max_rounds);

    let total = within_limit.len();
    // Round up values, which will favor accepts over rejections. Otherwise
    // rejections will dominate when number of proposals are small.
    let accept_end = (total * weights.accept as usize).div_ceil(100);
    let counter_end = (total * (weights.accept as usize + weights.counter as usize)).div_ceil(100);
    println!("total {total} accept_end {accept_end} counter_end {counter_end}");

    let mut iter = within_limit.into_iter();
    let mut to_accept: Vec<_> = iter.by_ref().take(accept_end).collect();
    to_accept.extend(over_limit);
    let to_counter: Vec<_> = iter.by_ref().take(counter_end - accept_end).collect();
    let to_reject: Vec<_> = iter.collect();

    // How much this agent can still spend before reaching its credit limit. Committing to
    // a proposal spends its amount, so only commit while it can be afforded and leave the
    // rest for a later iteration, once earlier spends have settled and freed up credit.
    // Re-read after a commit, since nothing else here changes the balance.
    let mut available = get_spendable_amount(ctx)?.unwrap_or_else(ZFuel::zero);

    let reporter = ctx.runner_context().reporter();
    let agent_key = ctx.get().cell_id().agent_pubkey().to_string();

    // Accept: commit to the proposal
    for tx in &to_accept {
        // Committing for this agent means spending a positive amount.
        let spend = tx.amount.get_base_unyt();
        if available < spend {
            log::info!(
                "Deferring commit to proposal {}: spend {spend:?} exceeds remaining credit {available:?}",
                tx.id
            );
            continue;
        }
        match ctx.unyt_create_commit_to_proposal(CommitmentToProposalInput {
            proposal: tx.id.clone(),
            note: None,
        }) {
            Ok(commitment_hash) => {
                available = get_spendable_amount(ctx)?.unwrap_or_else(ZFuel::zero);
                log::info!(
                    "Committed to proposal {} after {} negotiation round(s)",
                    tx.id,
                    tx.history.len()
                );
                // Emit negotiation_rounds: history length = number of counter-proposal rounds
                reporter.add_custom(
                    ReportMetric::new("negotiation_rounds")
                        .with_tag("agent", agent_key.clone())
                        .with_field("value", tx.history.len() as u64),
                );
                ctx.get_mut()
                    .scenario_values
                    .watched_transactions_mut()
                    .push(commitment_hash);
            }
            Err(err) => {
                log::warn!("Failed to commit to proposal {}: {err}", tx.id);
            }
        }
    }

    // Reject: reject the proposal
    for tx in &to_reject {
        match ctx.unyt_create_reject_proposal(RejectInput {
            proposal: tx.id.clone(),
            note: None,
        }) {
            Ok(_) => log::info!("Rejected proposal {}", tx.id),
            Err(err) => log::warn!("Failed to reject proposal {}: {err}", tx.id),
        }
    }

    // Counter: send a counter-proposal back
    counter_proposals(ctx, to_counter)?;

    Ok(())
}

/// Handle incoming commitments by accepting or rejecting according to `UNYT_COMMITMENT_ACCEPT_PCT`.
///
/// The first `accept_pct` percent of commitments are accepted (via `create_accept` → push to watched),
/// the rest are rejected.
pub fn handle_commitments(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    commitments: Vec<Transaction>,
) -> anyhow::Result<()> {
    let accept_pct: u8 = std::env::var("UNYT_COMMITMENT_ACCEPT_PCT")
        .unwrap_or_else(|_| "80".to_string())
        .parse()?;

    if accept_pct > 100 {
        anyhow::bail!("UNYT_COMMITMENT_ACCEPT_PCT must be between 0 and 100, got {accept_pct}");
    }

    // Round up values, which will favor accepts over rejections. Otherwise
    // rejections will dominate when number of proposals are small.
    let total = commitments.len();
    let accept_end = (total * accept_pct as usize).div_ceil(100);

    let mut iter = commitments.into_iter();
    let to_accept: Vec<_> = iter.by_ref().take(accept_end).collect();
    let to_reject: Vec<_> = iter.collect();

    for tx in &to_accept {
        match ctx.unyt_create_accept(AcceptInput {
            commitment: tx.id.clone(),
            note: None,
        }) {
            Ok(accept_hash) => {
                log::info!("Accepted commitment {}", tx.id);
                ctx.get_mut()
                    .scenario_values
                    .watched_transactions_mut()
                    .push(accept_hash);
            }
            Err(err) => {
                log::warn!("Failed to accept commitment {}: {err}", tx.id);
            }
        }
    }

    for tx in &to_reject {
        match ctx.unyt_create_reject_proposal(RejectInput {
            proposal: tx.id.clone(),
            note: None,
        }) {
            Ok(_) => log::info!("Rejected commitment {}", tx.id),
            Err(err) => log::warn!("Failed to reject commitment {}: {err}", tx.id),
        }
    }

    Ok(())
}

/// Reduces each unit in the `UnitMap` by `pct` percent.
/// E.g. with `pct = 10`, a value of 100 becomes 90.
fn adjust_amount(amount: &UnitMap, pct: u8) -> anyhow::Result<UnitMap> {
    let factor = Fraction::new((100 - pct) as i64, 100)?;
    let mut adjusted = BTreeMap::new();
    for (key, value) in amount.into_iter() {
        adjusted.insert(key, (value * factor)?);
    }
    Ok(UnitMap::load(adjusted))
}

/// Check ledger and return the spendable amount.
///
/// Returns `Ok(None)` when the DHT is in a transient state and should be retried on the next iteration.
pub fn get_spendable_amount(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> anyhow::Result<Option<ZFuel>> {
    let ledger = match ctx.unyt_get_ledger() {
        Ok(l) => l,
        Err(err) => {
            log::warn!("Failed to get ledger (transient DHT issue): {err}");
            thread::sleep(Duration::from_secs(1));
            return Ok(None);
        }
    };
    log::info!(
        "Agent {} | ledger: {:?}",
        ctx.get().cell_id().agent_pubkey(),
        ledger
    );
    let balance = ledger.balance.get_base_unyt();
    let fees = ledger.fees_owed;
    // proposed_balance is value the agent has already promised to spend, but
    // that hasn't settled yet. It has to be counted here to prevent pushing
    // the agent over its credit limit.
    let proposed_balance = ledger.proposed_balance.get_base_unyt();
    let credit_limit = match ctx.unyt_get_my_current_applied_credit_limit() {
        Ok(cl) => cl,
        Err(err) => {
            log::warn!("Failed to get credit limit (transient DHT issue): {err}");
            thread::sleep(Duration::from_secs(1));
            return Ok(None);
        }
    };
    let spendable_amount = (balance - fees + proposed_balance + credit_limit.get_base_unyt())?;

    Ok(Some(spendable_amount))
}

/// Poll `get_status` for watched transactions and remove completed ones.
pub fn poll_watched_transactions(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) {
    let watched = ctx.get().scenario_values.watched_transactions().clone();
    if watched.is_empty() {
        return;
    }

    log::info!(
        "Agent {} | polling get_status for {} watched transactions",
        ctx.get().cell_id().agent_pubkey(),
        watched.len()
    );
    let mut completed = Vec::new();
    for tx_id in &watched {
        match ctx.unyt_get_status(tx_id.clone()) {
            Ok(state) => {
                if state.status == WatchStatus::Completed {
                    completed.push(tx_id.clone());
                }
            }
            Err(err) => {
                log::warn!("Failed to get_status for {tx_id}: {err}");
            }
        }
    }
    if !completed.is_empty() {
        log::info!(
            "Agent {} | {} watched transactions completed",
            ctx.get().cell_id().agent_pubkey(),
            completed.len()
        );
        ctx.get_mut()
            .scenario_values
            .watched_transactions_mut()
            .retain(|tx| !completed.contains(tx));
    }
}

/// Measure sync lag for all newly seen transactions across actionable lists.
///
/// For each transaction not yet in `seen_transactions`, computes `now - tx.timestamp`
/// and emits a `sync_lag` metric tagged with the transaction type.
pub fn measure_sync_lag(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    actionable: &Actionable,
) {
    let reporter = ctx.runner_context().reporter();
    let agent_key = ctx.get().cell_id().agent_pubkey().to_string();
    let now_s = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_secs_f64();

    let lists: &[(&str, &[Transaction])] = &[
        ("proposal", &actionable.proposal_actionable),
        ("commitment", &actionable.commitment_actionable),
        ("accept", &actionable.accept_actionable),
        ("reject", &actionable.reject_actionable),
    ];

    for &(tx_type, txs) in lists {
        for tx in txs {
            if ctx
                .get()
                .scenario_values
                .seen_transactions()
                .contains(&(tx.id.clone(), tx_type))
            {
                continue;
            }
            let published_at_s = tx.timestamp.as_micros() as f64 / 1e6;
            let lag_s = (now_s - published_at_s).max(0.0);
            reporter.add_custom(
                ReportMetric::new("sync_lag")
                    .with_tag("agent", agent_key.clone())
                    .with_tag("tx_type", tx_type)
                    .with_field("value", lag_s),
            );
            ctx.get_mut()
                .scenario_values
                .seen_transactions_mut()
                .insert((tx.id.clone(), tx_type));
        }
    }
}

/// Fetch recent transaction history (mirrors UI refresh).
pub fn poll_history(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) {
    match ctx.unyt_get_history(Pagination {
        high_boundary: None,
        per_page: 10,
    }) {
        Ok(history) => {
            log::info!(
                "Agent {} | get_history returned {} items",
                ctx.get().cell_id().agent_pubkey(),
                history.items.len()
            );
        }
        Err(err) => {
            log::warn!("Failed to get history: {err}");
        }
    }
}

pub fn is_network_initialized(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> anyhow::Result<bool> {
    let session_started_at = ctx
        .get()
        .scenario_values
        .session_start_time()
        .ok_or(anyhow::anyhow!("`session_started_at` not set"))?;
    let reporter = ctx.runner_context().reporter();
    let network_initialized = ctx.get().scenario_values.network_initialized();

    // short-circuit on network initialized (stored-value)
    if network_initialized {
        return Ok(true);
    }

    // check if network is initialized
    if ctx.is_network_initialized() {
        log::info!(
            "Network initialized for agent {}",
            ctx.get().cell_id().agent_pubkey()
        );
        reporter.add_custom(
            ReportMetric::new("global_definition_propagation_time")
                .with_field("value", session_started_at.elapsed().as_secs())
                .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string()),
        );
        ctx.get_mut().scenario_values.set_network_initialized(true);
        Ok(true)
    } else {
        // if the network is not initialized do not proceed with further testing without waiting for it to be initialized
        log::info!(
            "Network not initialized for agent {}, waiting for it to be initialized",
            ctx.get().cell_id().agent_pubkey()
        );
        thread::sleep(Duration::from_secs(2));
        Ok(false)
    }
}
