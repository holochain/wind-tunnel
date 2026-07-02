use crate::unyt_agent::UnytAgentExt;
use crate::{ArcType, UnytScenarioValues};
use holochain_types::prelude::ActionHashB64;
use holochain_wind_tunnel_runner::prelude::{
    AgentContext, HolochainAgentContext, HolochainRunnerContext, ReportMetric,
};
use rave_engine::types::{Actionable, Transaction};
use std::str::FromStr;
use std::time::{Instant, SystemTime};

pub fn record_sync_lag<SV: UnytScenarioValues>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    arc_type: &ArcType,
    actionables: &[Transaction],
    tx_type: &'static str,
) {
    let reporter = ctx.runner_context().reporter();
    let now_us = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_micros();
    let agent_key = ctx.get().cell_id().agent_pubkey().to_string();
    let unseen_txs: Vec<_> = actionables
        .iter()
        .filter(|tx| {
            !ctx.get()
                .scenario_values
                .seen_transactions()
                .contains(&(tx.id.clone(), tx_type))
        })
        .collect();
    for tx in unseen_txs {
        let published_at_us = tx.timestamp.as_micros() as u128;
        let lag_s = now_us.saturating_sub(published_at_us) as f64 / 1e6;
        reporter.add_custom(
            ReportMetric::new("sync_lag")
                .with_tag("tx_type", tx_type.to_owned())
                .with_tag("agent", agent_key.clone())
                .with_tag("arc", arc_type.as_tag())
                .with_field("value", lag_s),
        );
        ctx.get_mut()
            .scenario_values
            .seen_transactions_mut()
            .insert((tx.id.clone(), tx_type));
    }
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

/// Refresh the action list the way the UI does.
pub fn ui_action_list_refresh<SV: UnytScenarioValues>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
) -> Actionable {
    let notification_links = match ctx.unyt_get_all_notification_links() {
        Ok(links) => links,
        Err(err) => {
            log::warn!("get_all_notification_links failed: {err}");
            return empty_actionable();
        }
    };

    let reporter = ctx.runner_context().reporter();
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

    reporter.add_custom(
        ReportMetric::new("ui_action_list_refresh_failed_calls").with_field("value", failed_calls),
    );
    reporter.add_custom(
        ReportMetric::new("ui_action_list_refresh_duration_s")
            .with_field("value", started.elapsed().as_secs_f64()),
    );

    actionable.unwrap_or_else(empty_actionable)
}

/// Refresh the details of the watched transactions the way the UI does.
pub fn ui_transaction_detail_refresh<SV: UnytScenarioValues>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    watched: &[ActionHashB64],
) {
    if watched.is_empty() {
        return;
    }

    let reporter = ctx.runner_context().reporter();
    let started = Instant::now();
    let mut totals = TransactionDetailRefreshOutcome::default();

    for transaction_hash in watched {
        let item_started = Instant::now();
        let outcome = run_transaction_detail_refresh(ctx, transaction_hash.clone());
        reporter.add_custom(
            ReportMetric::new("ui_transaction_detail_item_refresh_duration_s")
                .with_field("value", item_started.elapsed().as_secs_f64()),
        );
        reporter.add_custom(
            ReportMetric::new("ui_transaction_detail_item_refresh_primary_transaction_total_calls")
                .with_field("value", outcome.primary_transaction_total_calls),
        );
        reporter.add_custom(
            ReportMetric::new(
                "ui_transaction_detail_item_refresh_primary_transaction_failed_calls",
            )
            .with_field("value", outcome.primary_transaction_failed_calls),
        );
        reporter.add_custom(
            ReportMetric::new("ui_transaction_detail_item_refresh_related_transaction_total_calls")
                .with_field("value", outcome.related_transaction_total_calls),
        );
        reporter.add_custom(
            ReportMetric::new(
                "ui_transaction_detail_item_refresh_related_transaction_failed_calls",
            )
            .with_field("value", outcome.related_transaction_failed_calls),
        );
        totals.primary_transaction_total_calls += outcome.primary_transaction_total_calls;
        totals.primary_transaction_failed_calls += outcome.primary_transaction_failed_calls;
        totals.related_transaction_total_calls += outcome.related_transaction_total_calls;
        totals.related_transaction_failed_calls += outcome.related_transaction_failed_calls;
    }

    reporter.add_custom(
        ReportMetric::new("ui_transaction_detail_refresh_duration_s")
            .with_field("value", started.elapsed().as_secs_f64()),
    );
    reporter.add_custom(
        ReportMetric::new("ui_transaction_detail_refresh_transactions_processed")
            .with_field("value", watched.len() as u64),
    );
    reporter.add_custom(
        ReportMetric::new("ui_transaction_detail_refresh_primary_transaction_total_calls")
            .with_field("value", totals.primary_transaction_total_calls),
    );
    reporter.add_custom(
        ReportMetric::new("ui_transaction_detail_refresh_primary_transaction_failed_calls")
            .with_field("value", totals.primary_transaction_failed_calls),
    );
    reporter.add_custom(
        ReportMetric::new("ui_transaction_detail_refresh_related_transaction_total_calls")
            .with_field("value", totals.related_transaction_total_calls),
    );
    reporter.add_custom(
        ReportMetric::new("ui_transaction_detail_refresh_related_transaction_failed_calls")
            .with_field("value", totals.related_transaction_failed_calls),
    );
}

fn run_transaction_detail_refresh<SV: UnytScenarioValues>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
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
