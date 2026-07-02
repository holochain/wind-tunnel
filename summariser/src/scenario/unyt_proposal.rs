use crate::analyze::{aggregated_single_value, partitioned_timing_stats, standard_timing_stats};
use crate::frame::LoadError;
use crate::model::{AggregatedSingleValue, PartitionedTimingStats, StandardTimingsStats};
use crate::query;
use anyhow::Context;
use polars::prelude::{IntoLazy, col, lit};
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

/// Summary of a `unyt_proposal` scenario run.
///
/// Captures negotiation performance (round-trip times, counter-proposal rounds),
/// DHT propagation delays (sync lag per transaction type), zome call latencies
/// for key proposal-lifecycle operations, and end-of-run transaction counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct UnytProposalSummary {
    /// Time (seconds since session start) at which each agent first detects network
    /// initialization. Higher values indicate slower global-definition propagation.
    global_definition_propagation_time: StandardTimingsStats,

    /// Duration of proposal round trips that ended in acceptance, per agent (seconds).
    /// Measures time from initial proposal creation to final acceptance/receipt.
    /// `None` when no proposals completed acceptance during the run.
    proposal_round_trip_accepted: Option<PartitionedTimingStats>,

    /// Duration of proposal round trips that ended in rejection, per agent (seconds).
    /// Measures time from initial proposal creation to rejection/reclaim.
    /// `None` when no proposals were rejected during the run.
    proposal_round_trip_rejected: Option<PartitionedTimingStats>,

    /// Distribution of counter-proposal rounds before a proposal reaches commitment.
    /// Lower values indicate faster negotiation convergence. A value of 0 means the
    /// first proposal was committed to directly without counter-proposals.
    negotiation_rounds: StandardTimingsStats,

    /// Delay between transaction publish and first observation for proposal-type
    /// transactions (seconds). Indicates DHT propagation speed for new proposals.
    sync_lag_proposal: Option<PartitionedTimingStats>,

    /// Delay between transaction publish and first observation for commitment-type
    /// transactions (seconds).
    sync_lag_commitment: Option<PartitionedTimingStats>,

    /// Delay between transaction publish and first observation for accept-type
    /// transactions (seconds).
    sync_lag_accept: Option<PartitionedTimingStats>,

    /// Delay between transaction publish and first observation for reject-type
    /// transactions (seconds).
    sync_lag_reject: Option<PartitionedTimingStats>,

    /// Duration of `create_proposal` zome calls per agent (seconds)
    create_proposal_zome_call: PartitionedTimingStats,

    /// Duration of `create_counter_proposal` zome calls per agent (seconds).
    /// `None` when no counter-proposals were created (e.g. `UNYT_PROPOSAL_WEIGHTS=100,0,0`).
    create_counter_proposal_zome_call: Option<PartitionedTimingStats>,

    /// Duration of `create_commit_to_proposal` zome calls per agent (seconds).
    /// `None` when no commitments were created during the run.
    create_commit_to_proposal_zome_call: Option<PartitionedTimingStats>,

    /// Duration of `create_accept` zome calls per agent (seconds).
    /// `None` when no accepts were created during the run.
    create_accept_zome_call: Option<PartitionedTimingStats>,

    /// Duration of `create_reject_proposal` zome calls per agent (seconds).
    /// `None` when no rejections were created during the run.
    create_reject_proposal_zome_call: Option<PartitionedTimingStats>,

    /// Duration of `create_receipt_for_accept` zome calls per agent (seconds).
    /// `None` when no receipts were created during the run.
    create_receipt_for_accept_zome_call: Option<PartitionedTimingStats>,

    /// Duration of `create_reclaim_balance` zome calls per agent (seconds).
    /// `None` when no reclaims were created (e.g. `UNYT_COMMITMENT_ACCEPT_PCT=100`).
    create_reclaim_balance_zome_call: Option<PartitionedTimingStats>,

    /// Duration of `get_actionable_transactions` zome calls per agent (seconds).
    /// This is called every iteration, so high latency here directly impacts loop throughput.
    get_actionable_transactions_zome_call: PartitionedTimingStats,

    /// Duration of the UI action list refresh per behaviour iteration (seconds).
    ui_action_list_refresh_duration_s: StandardTimingsStats,
    /// Total UI action list refresh calls that failed across all agents and iterations.
    ui_action_list_refresh_failed_calls: AggregatedSingleValue,
    /// Duration of refreshing a single transaction detail item per behaviour iteration (seconds).
    ui_transaction_detail_item_refresh_duration_s: StandardTimingsStats,
    /// Total primary transaction fetches made during per-item detail refresh across all agents and iterations.
    ui_transaction_detail_item_refresh_primary_transaction_total_calls: AggregatedSingleValue,
    /// Total primary transaction fetches that failed during per-item detail refresh across all agents and iterations.
    ui_transaction_detail_item_refresh_primary_transaction_failed_calls: AggregatedSingleValue,
    /// Total related transaction fetches made during per-item detail refresh across all agents and iterations.
    ui_transaction_detail_item_refresh_related_transaction_total_calls: AggregatedSingleValue,
    /// Total related transaction fetches that failed during per-item detail refresh across all agents and iterations.
    ui_transaction_detail_item_refresh_related_transaction_failed_calls: AggregatedSingleValue,
    /// Duration of refreshing all watched transaction details per behaviour iteration (seconds).
    ui_transaction_detail_refresh_duration_s: StandardTimingsStats,
    /// Total number of transactions processed during the detail refresh across all agents and iterations.
    ui_transaction_detail_refresh_transactions_processed: AggregatedSingleValue,
    /// Total primary transaction fetches made during the full detail refresh across all agents and iterations.
    ui_transaction_detail_refresh_primary_transaction_total_calls: AggregatedSingleValue,
    /// Total primary transaction fetches that failed during the full detail refresh across all agents and iterations.
    ui_transaction_detail_refresh_primary_transaction_failed_calls: AggregatedSingleValue,
    /// Total related transaction fetches made during the full detail refresh across all agents and iterations.
    ui_transaction_detail_refresh_related_transaction_total_calls: AggregatedSingleValue,
    /// Total related transaction fetches that failed during the full detail refresh across all agents and iterations.
    ui_transaction_detail_refresh_related_transaction_failed_calls: AggregatedSingleValue,

    /// Number of zome call errors observed during the run
    error_count: usize,

    /// Total completed accept transactions across all agents at teardown.
    completed_transaction_accepts: AggregatedSingleValue,

    /// Total completed spend transactions across all agents at teardown.
    completed_transaction_spends: AggregatedSingleValue,

    /// Total completed RAVE agreement executions across all agents at teardown.
    completed_transaction_raves: AggregatedSingleValue,

    /// Remaining actionable proposals across all agents at teardown.
    /// Non-zero sum indicates the scenario ended with unresolved proposals.
    actionable_transaction_proposals: AggregatedSingleValue,

    /// Remaining actionable commitments across all agents at teardown.
    /// Non-zero sum indicates the scenario ended with unresolved commitments.
    actionable_transaction_commitments: AggregatedSingleValue,

    /// Remaining actionable accepts across all agents at teardown.
    /// Non-zero sum indicates the scenario ended with unresolved accepts.
    actionable_transaction_accepts: AggregatedSingleValue,

    /// Remaining actionable rejects across all agents at teardown.
    /// Non-zero sum indicates the scenario ended with unresolved rejects.
    actionable_transaction_rejects: AggregatedSingleValue,
}

pub(crate) async fn summarize_unyt_proposal(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<UnytProposalSummary> {
    assert_eq!(summary.scenario_name, "unyt_proposal");

    // --- Custom metrics ---

    let propagation_time = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.global_definition_propagation_time",
        &["agent"],
    )
    .await
    .context("Load global_definition_propagation_time")?;

    let round_trip_data = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.proposal_round_trip_time",
        &["agent", "outcome"],
    )
    .await;

    let negotiation_data = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.negotiation_rounds",
        &["agent"],
    )
    .await
    .context("Load negotiation_rounds")?;

    let sync_lag_data = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.sync_lag",
        &["agent", "tx_type"],
    )
    .await;

    // --- Zome call timing data (single query, filtered per fn_name) ---

    let zome_calls = query::query_zome_call_instrument_data(client.clone(), &summary)
        .await
        .context("Load zome call instrument data")?;

    // --- Teardown metrics ---

    let completed_transaction_accepts = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.completed_transaction_accepts",
        &[],
    )
    .await;

    let completed_transaction_spends = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.completed_transaction_spends",
        &[],
    )
    .await;

    let completed_transaction_raves = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.completed_transaction_raves",
        &[],
    )
    .await;

    let actionable_transaction_proposals = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.actionable_transaction_proposals",
        &[],
    )
    .await;

    let actionable_transaction_commitments = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.actionable_transaction_commitments",
        &[],
    )
    .await;

    let actionable_transaction_accepts = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.actionable_transaction_accepts",
        &[],
    )
    .await;

    let actionable_transaction_rejects = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.actionable_transaction_rejects",
        &[],
    )
    .await;

    // --- UI refresh metrics ---

    let ui_action_list_refresh_duration_s = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_action_list_refresh_duration_s",
        &[],
    )
    .await
    .context("Load ui_action_list_refresh_duration_s data")?;

    let ui_action_list_refresh_failed_calls = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_action_list_refresh_failed_calls",
        &[],
    )
    .await
    .context("Load ui_action_list_refresh_failed_calls data")?;

    let ui_transaction_detail_item_refresh_duration_s = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_transaction_detail_item_refresh_duration_s",
        &[],
    )
    .await
    .context("Load ui_transaction_detail_item_refresh_duration_s data")?;

    let ui_transaction_detail_item_refresh_primary_transaction_total_calls =
        query::query_custom_data(
            client.clone(),
            &summary,
            "wt.custom.ui_transaction_detail_item_refresh_primary_transaction_total_calls",
            &[],
        )
        .await
        .context("Load ui_transaction_detail_item_refresh_primary_transaction_total_calls data")?;

    let ui_transaction_detail_item_refresh_primary_transaction_failed_calls =
        query::query_custom_data(
            client.clone(),
            &summary,
            "wt.custom.ui_transaction_detail_item_refresh_primary_transaction_failed_calls",
            &[],
        )
        .await
        .context("Load ui_transaction_detail_item_refresh_primary_transaction_failed_calls data")?;

    let ui_transaction_detail_item_refresh_related_transaction_total_calls =
        query::query_custom_data(
            client.clone(),
            &summary,
            "wt.custom.ui_transaction_detail_item_refresh_related_transaction_total_calls",
            &[],
        )
        .await
        .context("Load ui_transaction_detail_item_refresh_related_transaction_total_calls data")?;

    let ui_transaction_detail_item_refresh_related_transaction_failed_calls =
        query::query_custom_data(
            client.clone(),
            &summary,
            "wt.custom.ui_transaction_detail_item_refresh_related_transaction_failed_calls",
            &[],
        )
        .await
        .context("Load ui_transaction_detail_item_refresh_related_transaction_failed_calls data")?;

    let ui_transaction_detail_refresh_duration_s = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_transaction_detail_refresh_duration_s",
        &[],
    )
    .await
    .context("Load ui_transaction_detail_refresh_duration_s data")?;

    let ui_transaction_detail_refresh_transactions_processed = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_transaction_detail_refresh_transactions_processed",
        &[],
    )
    .await
    .context("Load ui_transaction_detail_refresh_transactions_processed data")?;

    let ui_transaction_detail_refresh_primary_transaction_total_calls = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_transaction_detail_refresh_primary_transaction_total_calls",
        &[],
    )
    .await
    .context("Load ui_transaction_detail_refresh_primary_transaction_total_calls data")?;

    let ui_transaction_detail_refresh_primary_transaction_failed_calls = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_transaction_detail_refresh_primary_transaction_failed_calls",
        &[],
    )
    .await
    .context("Load ui_transaction_detail_refresh_primary_transaction_failed_calls data")?;

    let ui_transaction_detail_refresh_related_transaction_total_calls = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_transaction_detail_refresh_related_transaction_total_calls",
        &[],
    )
    .await
    .context("Load ui_transaction_detail_refresh_related_transaction_total_calls data")?;

    let ui_transaction_detail_refresh_related_transaction_failed_calls = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_transaction_detail_refresh_related_transaction_failed_calls",
        &[],
    )
    .await
    .context("Load ui_transaction_detail_refresh_related_transaction_failed_calls data")?;

    // --- Compute round-trip stats by outcome ---

    let (round_trip_accepted, round_trip_rejected) = match round_trip_data {
        Ok(frame) => {
            let accepted = frame
                .clone()
                .lazy()
                .filter(col("outcome").eq(lit("accepted")))
                .collect()?;
            let rejected = frame
                .lazy()
                .filter(col("outcome").eq(lit("rejected")))
                .collect()?;
            (Some(accepted), Some(rejected))
        }
        Err(_) => (None, None),
    };

    // --- Compute sync lag stats by tx_type ---

    let sync_lag_proposal = filter_sync_lag(&sync_lag_data, "proposal");
    let sync_lag_commitment = filter_sync_lag(&sync_lag_data, "commitment");
    let sync_lag_accept = filter_sync_lag(&sync_lag_data, "accept");
    let sync_lag_reject = filter_sync_lag(&sync_lag_data, "reject");

    // --- Compute zome call stats per fn_name ---

    let zome_call_stats = |fn_name: &str| -> anyhow::Result<PartitionedTimingStats> {
        let filtered = zome_calls
            .clone()
            .lazy()
            .filter(col("fn_name").eq(lit(fn_name)))
            .collect()?;
        partitioned_timing_stats(filtered, "value", "10s", &["agent"])
            .context(format!("Timing stats for zome call {fn_name}"))
    };

    let optional_zome_call_stats =
        |fn_name: &str| -> anyhow::Result<Option<PartitionedTimingStats>> {
            let filtered = zome_calls
                .clone()
                .lazy()
                .filter(col("fn_name").eq(lit(fn_name)))
                .collect()?;
            if filtered.height() == 0 {
                return Ok(None);
            }
            partitioned_timing_stats(filtered, "value", "10s", &["agent"])
                .map(Some)
                .context(format!("Timing stats for zome call {fn_name}"))
        };

    Ok(UnytProposalSummary {
        global_definition_propagation_time: standard_timing_stats(
            propagation_time,
            "value",
            "10s",
            None,
        )
        .context("Stats for global_definition_propagation_time")?,
        proposal_round_trip_accepted: round_trip_accepted
            .filter(|f| f.height() > 0)
            .map(|f| partitioned_timing_stats(f, "value", "10s", &["agent"]))
            .transpose()
            .context("Round-trip timing (accepted)")?,
        proposal_round_trip_rejected: round_trip_rejected
            .filter(|f| f.height() > 0)
            .map(|f| partitioned_timing_stats(f, "value", "10s", &["agent"]))
            .transpose()
            .context("Round-trip timing (rejected)")?,
        negotiation_rounds: standard_timing_stats(negotiation_data, "value", "10s", None)
            .context("Stats for negotiation_rounds")?,
        sync_lag_proposal,
        sync_lag_commitment,
        sync_lag_accept,
        sync_lag_reject,
        create_proposal_zome_call: zome_call_stats("create_proposal")?,
        create_counter_proposal_zome_call: optional_zome_call_stats("create_counter_proposal")?,
        create_commit_to_proposal_zome_call: optional_zome_call_stats("create_commit_to_proposal")?,
        create_accept_zome_call: optional_zome_call_stats("create_accept")?,
        create_reject_proposal_zome_call: optional_zome_call_stats("create_reject_proposal")?,
        create_receipt_for_accept_zome_call: optional_zome_call_stats("create_receipt_for_accept")?,
        create_reclaim_balance_zome_call: optional_zome_call_stats("create_reclaim_balance")?,
        get_actionable_transactions_zome_call: zome_call_stats("get_actionable_transactions")?,
        ui_action_list_refresh_duration_s: standard_timing_stats(
            ui_action_list_refresh_duration_s,
            "value",
            "10s",
            None,
        )
        .context("Timing stats for ui_action_list_refresh_duration_s")?,
        ui_action_list_refresh_failed_calls: aggregated_single_value(
            ui_action_list_refresh_failed_calls,
            "value",
        )
        .context("Aggregated single value for ui_action_list_refresh_failed_calls")?,
        ui_transaction_detail_item_refresh_duration_s: standard_timing_stats(
            ui_transaction_detail_item_refresh_duration_s,
            "value",
            "10s",
            None,
        )
        .context("Timing stats for ui_transaction_detail_item_refresh_duration_s")?,
        ui_transaction_detail_item_refresh_primary_transaction_total_calls:
            aggregated_single_value(
                ui_transaction_detail_item_refresh_primary_transaction_total_calls,
                "value",
            )
            .context(
                "Aggregated single value for ui_transaction_detail_item_refresh_primary_transaction_total_calls",
            )?,
        ui_transaction_detail_item_refresh_primary_transaction_failed_calls:
            aggregated_single_value(
                ui_transaction_detail_item_refresh_primary_transaction_failed_calls,
                "value",
            )
            .context(
                "Aggregated single value for ui_transaction_detail_item_refresh_primary_transaction_failed_calls",
            )?,
        ui_transaction_detail_item_refresh_related_transaction_total_calls:
            aggregated_single_value(
                ui_transaction_detail_item_refresh_related_transaction_total_calls,
                "value",
            )
            .context(
                "Aggregated single value for ui_transaction_detail_item_refresh_related_transaction_total_calls",
            )?,
        ui_transaction_detail_item_refresh_related_transaction_failed_calls:
            aggregated_single_value(
                ui_transaction_detail_item_refresh_related_transaction_failed_calls,
                "value",
            )
            .context(
                "Aggregated single value for ui_transaction_detail_item_refresh_related_transaction_failed_calls",
            )?,
        ui_transaction_detail_refresh_duration_s: standard_timing_stats(
            ui_transaction_detail_refresh_duration_s,
            "value",
            "10s",
            None,
        )
        .context("Timing stats for ui_transaction_detail_refresh_duration_s")?,
        ui_transaction_detail_refresh_transactions_processed: aggregated_single_value(
            ui_transaction_detail_refresh_transactions_processed,
            "value",
        )
        .context(
            "Aggregated single value for ui_transaction_detail_refresh_transactions_processed",
        )?,
        ui_transaction_detail_refresh_primary_transaction_total_calls: aggregated_single_value(
            ui_transaction_detail_refresh_primary_transaction_total_calls,
            "value",
        )
        .context(
            "Aggregated single value for ui_transaction_detail_refresh_primary_transaction_total_calls",
        )?,
        ui_transaction_detail_refresh_primary_transaction_failed_calls: aggregated_single_value(
            ui_transaction_detail_refresh_primary_transaction_failed_calls,
            "value",
        )
        .context(
            "Aggregated single value for ui_transaction_detail_refresh_primary_transaction_failed_calls",
        )?,
        ui_transaction_detail_refresh_related_transaction_total_calls: aggregated_single_value(
            ui_transaction_detail_refresh_related_transaction_total_calls,
            "value",
        )
        .context(
            "Aggregated single value for ui_transaction_detail_refresh_related_transaction_total_calls",
        )?,
        ui_transaction_detail_refresh_related_transaction_failed_calls: aggregated_single_value(
            ui_transaction_detail_refresh_related_transaction_failed_calls,
            "value",
        )
        .context(
            "Aggregated single value for ui_transaction_detail_refresh_related_transaction_failed_calls",
        )?,
        error_count: query::zome_call_error_count(client.clone(), &summary).await?,
        completed_transaction_accepts: optional_aggregated_single_value(
            completed_transaction_accepts,
            "completed_transaction_accepts",
        )?,
        completed_transaction_spends: optional_aggregated_single_value(
            completed_transaction_spends,
            "completed_transaction_spends",
        )?,
        completed_transaction_raves: optional_aggregated_single_value(
            completed_transaction_raves,
            "completed_transaction_raves",
        )?,
        actionable_transaction_proposals: optional_aggregated_single_value(
            actionable_transaction_proposals,
            "actionable_transaction_proposals",
        )?,
        actionable_transaction_commitments: optional_aggregated_single_value(
            actionable_transaction_commitments,
            "actionable_transaction_commitments",
        )?,
        actionable_transaction_accepts: optional_aggregated_single_value(
            actionable_transaction_accepts,
            "actionable_transaction_accepts",
        )?,
        actionable_transaction_rejects: optional_aggregated_single_value(
            actionable_transaction_rejects,
            "actionable_transaction_rejects",
        )?,
    })
}

/// Filters sync lag data by `tx_type` and computes partitioned timing stats.
///
/// Returns `None` when the sync lag query itself failed (no data) or when no rows
/// match the given `tx_type`.
fn filter_sync_lag(
    sync_lag_result: &anyhow::Result<polars::frame::DataFrame>,
    tx_type: &str,
) -> Option<PartitionedTimingStats> {
    let frame = match sync_lag_result {
        Ok(f) => f,
        Err(_) => return None,
    };
    let filtered = frame
        .clone()
        .lazy()
        .filter(col("tx_type").eq(lit(tx_type)))
        .collect()
        .ok()?;
    if filtered.height() == 0 {
        return None;
    }
    partitioned_timing_stats(filtered, "value", "10s", &["agent"]).ok()
}

/// Returns an [`AggregatedSingleValue`] from a query result, defaulting to zero when the
/// query returned no series (metric was never recorded).
fn optional_aggregated_single_value(
    result: anyhow::Result<polars::frame::DataFrame>,
    label: &str,
) -> anyhow::Result<AggregatedSingleValue> {
    match result {
        Ok(frame) => {
            aggregated_single_value(frame, "value").context(format!("Aggregated value for {label}"))
        }
        Err(e) => {
            if e.downcast_ref::<LoadError>().is_some() {
                Ok(AggregatedSingleValue::default())
            } else {
                Err(e).context(format!("Load {label}"))
            }
        }
    }
}
