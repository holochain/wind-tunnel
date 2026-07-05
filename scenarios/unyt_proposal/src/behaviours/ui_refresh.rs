//! UI-mirroring refresh helpers for the proposal scenario.
//!
//! These are proposal-local copies of the shared action-list / transaction-detail refresh
//! helpers. Unlike the shared versions (which emit each iteration's counts directly), these
//! emit **cumulative per-agent running totals** tagged with `agent`, so the summariser can
//! treat them as true counters and derive per-agent totals with `last - first`.

use crate::ScenarioValues;
use holochain_types::prelude::ActionHashB64;
use holochain_wind_tunnel_runner::prelude::{
    AgentContext, HolochainAgentContext, HolochainRunnerContext, ReportMetric,
};
use rave_engine::types::Actionable;
use std::str::FromStr;
use std::time::Instant;
use wind_tunnel_unyt_scenario::unyt_agent::UnytAgentExt;

/// Cumulative per-agent counts of UI-refresh sub-calls.
///
/// Each field is a monotonically increasing running total for the lifetime of the agent.
/// The refresh helpers add their per-iteration increments here and emit the running total,
/// so the summariser can difference the endpoints (`last - first`) to recover the per-agent
/// total. Emitting per-iteration values instead would leave nothing to difference.
#[derive(Debug, Default)]
pub struct UiRefreshCounters {
    /// Action-list refresh sub-calls that failed.
    pub action_list_refresh_failed_calls: u64,
    /// Transactions processed during full detail refreshes.
    pub transaction_detail_refresh_transactions_processed: u64,
    /// Primary transaction fetches made during full detail refreshes.
    pub transaction_detail_refresh_primary_transaction_total_calls: u64,
    /// Primary transaction fetches that failed during full detail refreshes.
    pub transaction_detail_refresh_primary_transaction_failed_calls: u64,
    /// Related transaction fetches made during full detail refreshes.
    pub transaction_detail_refresh_related_transaction_total_calls: u64,
    /// Related transaction fetches that failed during full detail refreshes.
    pub transaction_detail_refresh_related_transaction_failed_calls: u64,
}

/// Per-transaction call counts gathered while refreshing transaction details.
#[derive(Debug, Default)]
struct TransactionDetailRefreshOutcome {
    primary_transaction_total_calls: u64,
    primary_transaction_failed_calls: u64,
    related_transaction_total_calls: u64,
    related_transaction_failed_calls: u64,
}

fn empty_actionable() -> Actionable {
    Actionable {
        proposal_actionable: vec![],
        commitment_actionable: vec![],
        accept_actionable: vec![],
        reject_actionable: vec![],
    }
}

/// Refresh the action list the way the UI does, emitting a cumulative per-agent failure counter.
pub fn ui_action_list_refresh(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> Actionable {
    let notification_links = match ctx.unyt_get_all_notification_links() {
        Ok(links) => links,
        Err(err) => {
            log::warn!("get_all_notification_links failed: {err}");
            return empty_actionable();
        }
    };

    let reporter = ctx.runner_context().reporter();
    let agent_key = ctx.get().cell_id().agent_pubkey().to_string();
    let started = Instant::now();
    let refresh = ctx.unyt_action_list_refresh(notification_links);

    let mut failed_calls = 0_u64;
    if let Err(err) = &refresh.actionable_transactions {
        failed_calls = failed_calls.saturating_add(1);
        log::warn!("get_actionable_transactions failed during action list refresh: {err}");
    }
    if let Err(err) = &refresh.incoming_raves {
        failed_calls = failed_calls.saturating_add(1);
        log::warn!("get_incoming_raves failed during action list refresh: {err}");
    }
    if let Err(err) = &refresh.requests_to_execute_agreements {
        failed_calls = failed_calls.saturating_add(1);
        log::warn!("get_requests_to_execute_agreements failed during action list refresh: {err}");
    }
    if let Err(err) = &refresh.sorted_requests_to_spend {
        failed_calls = failed_calls.saturating_add(1);
        log::warn!("get_sorted_requests_to_spend failed during action list refresh: {err}");
    }

    let actionable = refresh.actionable_transactions.ok().flatten();

    // Accumulate into the agent's running total so the metric is a true counter.
    let cumulative_failed_calls = {
        let counters = ctx.get_mut().scenario_values.ui_refresh_counters_mut();
        counters.action_list_refresh_failed_calls = counters
            .action_list_refresh_failed_calls
            .saturating_add(failed_calls);
        counters.action_list_refresh_failed_calls
    };

    reporter.add_custom(
        ReportMetric::new("ui_action_list_refresh_failed_calls")
            .with_field("value", cumulative_failed_calls)
            .with_tag("agent", agent_key),
    );
    reporter.add_custom(
        ReportMetric::new("ui_action_list_refresh_duration_s")
            .with_field("value", started.elapsed().as_secs_f64()),
    );

    actionable.unwrap_or_else(empty_actionable)
}

/// Refresh the details of the watched transactions the way the UI does, emitting cumulative
/// per-agent counters for the sub-calls made.
pub fn ui_transaction_detail_refresh(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    watched: &[ActionHashB64],
) {
    if watched.is_empty() {
        return;
    }

    let reporter = ctx.runner_context().reporter();
    let agent_key = ctx.get().cell_id().agent_pubkey().to_string();
    let started = Instant::now();
    let mut totals = TransactionDetailRefreshOutcome::default();

    for transaction_hash in watched {
        let item_started = Instant::now();
        let outcome = run_transaction_detail_refresh(ctx, transaction_hash.clone());
        // Per-item latency is reported directly; the per-item call counts are summed into
        // `totals` and folded into the cumulative counters below.
        reporter.add_custom(
            ReportMetric::new("ui_transaction_detail_item_refresh_duration_s")
                .with_field("value", item_started.elapsed().as_secs_f64()),
        );
        totals.primary_transaction_total_calls += outcome.primary_transaction_total_calls;
        totals.primary_transaction_failed_calls += outcome.primary_transaction_failed_calls;
        totals.related_transaction_total_calls += outcome.related_transaction_total_calls;
        totals.related_transaction_failed_calls += outcome.related_transaction_failed_calls;
    }

    // Fold this refresh's totals into the agent's running totals so each metric is emitted
    // as a monotonically increasing counter.
    let counters = ctx.get_mut().scenario_values.ui_refresh_counters_mut();
    counters.transaction_detail_refresh_transactions_processed = counters
        .transaction_detail_refresh_transactions_processed
        .saturating_add(watched.len() as u64);
    counters.transaction_detail_refresh_primary_transaction_total_calls = counters
        .transaction_detail_refresh_primary_transaction_total_calls
        .saturating_add(totals.primary_transaction_total_calls);
    counters.transaction_detail_refresh_primary_transaction_failed_calls = counters
        .transaction_detail_refresh_primary_transaction_failed_calls
        .saturating_add(totals.primary_transaction_failed_calls);
    counters.transaction_detail_refresh_related_transaction_total_calls = counters
        .transaction_detail_refresh_related_transaction_total_calls
        .saturating_add(totals.related_transaction_total_calls);
    counters.transaction_detail_refresh_related_transaction_failed_calls = counters
        .transaction_detail_refresh_related_transaction_failed_calls
        .saturating_add(totals.related_transaction_failed_calls);

    reporter.add_custom(
        ReportMetric::new("ui_transaction_detail_refresh_duration_s")
            .with_field("value", started.elapsed().as_secs_f64()),
    );
    reporter.add_custom(
        ReportMetric::new("ui_transaction_detail_refresh_transactions_processed")
            .with_field(
                "value",
                counters.transaction_detail_refresh_transactions_processed,
            )
            .with_tag("agent", agent_key.clone()),
    );
    reporter.add_custom(
        ReportMetric::new("ui_transaction_detail_refresh_primary_transaction_total_calls")
            .with_field(
                "value",
                counters.transaction_detail_refresh_primary_transaction_total_calls,
            )
            .with_tag("agent", agent_key.clone()),
    );
    reporter.add_custom(
        ReportMetric::new("ui_transaction_detail_refresh_primary_transaction_failed_calls")
            .with_field(
                "value",
                counters.transaction_detail_refresh_primary_transaction_failed_calls,
            )
            .with_tag("agent", agent_key.clone()),
    );
    reporter.add_custom(
        ReportMetric::new("ui_transaction_detail_refresh_related_transaction_total_calls")
            .with_field(
                "value",
                counters.transaction_detail_refresh_related_transaction_total_calls,
            )
            .with_tag("agent", agent_key.clone()),
    );
    reporter.add_custom(
        ReportMetric::new("ui_transaction_detail_refresh_related_transaction_failed_calls")
            .with_field(
                "value",
                counters.transaction_detail_refresh_related_transaction_failed_calls,
            )
            .with_tag("agent", agent_key),
    );
}

fn run_transaction_detail_refresh(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    transaction_hash: ActionHashB64,
) -> TransactionDetailRefreshOutcome {
    let mut outcome = TransactionDetailRefreshOutcome::default();

    outcome.primary_transaction_total_calls += 1;
    let transaction = match ctx.unyt_get_transaction(transaction_hash.clone()) {
        Ok(transaction) => Some(transaction),
        Err(err) => {
            outcome.primary_transaction_failed_calls += 1;
            log::warn!(
                "get_transaction failed during detail refresh for {transaction_hash}: {err}"
            );
            None
        }
    };

    outcome.primary_transaction_total_calls += 1;
    let state = match ctx.unyt_get_status(transaction_hash.clone()) {
        Ok(state) => Some(state),
        Err(err) => {
            outcome.primary_transaction_failed_calls += 1;
            log::warn!("get_status failed during detail refresh for {transaction_hash}: {err}");
            None
        }
    };

    let related_hash = state
        .as_ref()
        .and_then(first_related_transaction_hash)
        .or_else(|| {
            transaction
                .as_ref()
                .and_then(first_related_transaction_hash)
        });

    if let Some(related_hash) = related_hash {
        outcome.related_transaction_total_calls += 1;
        if let Err(err) = ctx.unyt_get_transaction(related_hash.clone()) {
            outcome.related_transaction_failed_calls += 1;
            log::warn!(
                "related get_transaction failed during detail refresh for {related_hash}: {err}"
            );
        }
    }

    outcome
}

fn first_related_transaction_hash<T: serde::Serialize>(value: &T) -> Option<ActionHashB64> {
    let value = serde_json::to_value(value).ok()?;
    let first_related = value
        .get("related_transaction")?
        .as_array()?
        .first()?
        .clone();
    let related_hash = first_related.get("id")?.as_str()?;
    ActionHashB64::from_str(related_hash).ok()
}
