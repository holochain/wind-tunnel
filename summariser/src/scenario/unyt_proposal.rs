use crate::analyze::{
    aggregated_single_value, partitioned_counter_stats, partitioned_counter_stats_allow_empty,
    partitioned_timing_stats, standard_timing_stats,
};
use crate::frame::LoadError;
use crate::model::{
    AggregatedSingleValue, PartitionedCounterStats, PartitionedTimingStats, StandardTimingsStats,
};
use crate::query;
use anyhow::Context;
use polars::frame::DataFrame;
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
    /// initialization, partitioned by agent and arc type. Higher values indicate slower
    /// global-definition propagation; comparing the `arc` partitions shows whether zero-arc
    /// agents take longer to see the network initialized.
    global_definition_propagation_time: PartitionedTimingStats,

    /// Duration of proposal round trips that ended in acceptance (seconds), partitioned by
    /// arc type and agent. Measures time from initial proposal creation to final
    /// acceptance/receipt. Zeroed when no proposals completed acceptance during the run.
    proposal_round_trip_accepted: PartitionedTimingStats,

    /// Duration of proposal round trips that ended in rejection (seconds), partitioned by
    /// arc type and agent. Measures time from initial proposal creation to rejection/reclaim.
    /// Zeroed when no proposals were rejected during the run.
    proposal_round_trip_rejected: PartitionedTimingStats,

    /// Distribution of counter-proposal rounds before a proposal reaches commitment,
    /// partitioned by agent and arc type. Lower values indicate faster negotiation
    /// convergence. A value of 0 means the first proposal was committed to directly
    /// without counter-proposals.
    negotiation_rounds: PartitionedTimingStats,

    /// Delay between transaction publish and first observation for proposal-type
    /// transactions (seconds), partitioned by arc type and agent. Indicates DHT propagation
    /// speed for new proposals; zero-arc agents may see higher lag.
    sync_lag_proposal: PartitionedTimingStats,

    /// Delay between transaction publish and first observation for commitment-type
    /// transactions (seconds), partitioned by arc type and agent.
    sync_lag_commitment: PartitionedTimingStats,

    /// Delay between transaction publish and first observation for accept-type
    /// transactions (seconds), partitioned by arc type and agent.
    sync_lag_accept: PartitionedTimingStats,

    /// Delay between transaction publish and first observation for reject-type
    /// transactions (seconds), partitioned by arc type and agent.
    sync_lag_reject: PartitionedTimingStats,

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
    /// UI action list refresh sub-calls that failed, as a per-agent counter. Recorded as a
    /// cumulative running total per agent, so `total_count` is the run total across all agents
    /// and `max_per_partition` identifies the agent that saw the most failures.
    ui_action_list_refresh_failed_calls: PartitionedCounterStats,
    /// Duration of refreshing a single transaction detail item per behaviour iteration (seconds).
    ui_transaction_detail_item_refresh_duration_s: StandardTimingsStats,
    /// Duration of refreshing all watched transaction details per behaviour iteration (seconds).
    ui_transaction_detail_refresh_duration_s: StandardTimingsStats,
    /// Transactions processed during the full detail refresh, as a per-agent cumulative counter.
    ui_transaction_detail_refresh_transactions_processed: PartitionedCounterStats,
    /// Primary transaction fetches made during the full detail refresh, as a per-agent cumulative counter.
    ui_transaction_detail_refresh_primary_transaction_total_calls: PartitionedCounterStats,
    /// Primary transaction fetches that failed during the full detail refresh, as a per-agent cumulative counter.
    ui_transaction_detail_refresh_primary_transaction_failed_calls: PartitionedCounterStats,
    /// Related transaction fetches made during the full detail refresh, as a per-agent cumulative counter.
    ui_transaction_detail_refresh_related_transaction_total_calls: PartitionedCounterStats,
    /// Related transaction fetches that failed during the full detail refresh, as a per-agent cumulative counter.
    ui_transaction_detail_refresh_related_transaction_failed_calls: PartitionedCounterStats,

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
        &["agent", "arc"],
    )
    .await
    .context("Load global_definition_propagation_time")?;

    let round_trip_data = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.proposal_round_trip_time",
        &["agent", "outcome", "arc"],
    )
    .await
    .context("Load proposal_round_trip_time")?;

    let negotiation_data = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.negotiation_rounds",
        &["agent", "arc"],
    )
    .await
    .context("Load negotiation_rounds")?;

    let sync_lag_data = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.sync_lag",
        &["agent", "tx_type", "arc"],
    )
    .await
    .context("Load sync_lag")?;

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

    // Cumulative per-agent counter; queried with the `agent` tag so it can be partitioned.
    // Passed as a `Result` to the allow-empty helper so a run with no series degrades to zero.
    let ui_action_list_refresh_failed_calls = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_action_list_refresh_failed_calls",
        &["agent"],
    )
    .await;

    let ui_transaction_detail_item_refresh_duration_s = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_transaction_detail_item_refresh_duration_s",
        &[],
    )
    .await
    .context("Load ui_transaction_detail_item_refresh_duration_s data")?;

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
        &["agent"],
    )
    .await
    .context("Load ui_transaction_detail_refresh_transactions_processed data")?;

    let ui_transaction_detail_refresh_primary_transaction_total_calls = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_transaction_detail_refresh_primary_transaction_total_calls",
        &["agent"],
    )
    .await
    .context("Load ui_transaction_detail_refresh_primary_transaction_total_calls data")?;

    let ui_transaction_detail_refresh_primary_transaction_failed_calls = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_transaction_detail_refresh_primary_transaction_failed_calls",
        &["agent"],
    )
    .await
    .context("Load ui_transaction_detail_refresh_primary_transaction_failed_calls data")?;

    let ui_transaction_detail_refresh_related_transaction_total_calls = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_transaction_detail_refresh_related_transaction_total_calls",
        &["agent"],
    )
    .await
    .context("Load ui_transaction_detail_refresh_related_transaction_total_calls data")?;

    let ui_transaction_detail_refresh_related_transaction_failed_calls = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_transaction_detail_refresh_related_transaction_failed_calls",
        &["agent"],
    )
    .await
    .context("Load ui_transaction_detail_refresh_related_transaction_failed_calls data")?;

    // --- Split round-trip data by outcome and sync-lag data by tx_type ---
    //
    // Each frame keeps its `arc` and `agent` tags, which become partition keys in the
    // `partitioned_timing_stats` calls below. A category with no matching rows yields a
    // zeroed `PartitionedTimingStats` rather than being dropped.

    let filter_rows = |frame: &DataFrame, column: &str, value: &str| -> anyhow::Result<DataFrame> {
        Ok(frame
            .clone()
            .lazy()
            .filter(col(column).eq(lit(value)))
            .collect()?)
    };

    let round_trip_accepted = filter_rows(&round_trip_data, "outcome", "accepted")?;
    let round_trip_rejected = filter_rows(&round_trip_data, "outcome", "rejected")?;
    let sync_lag_proposal = filter_rows(&sync_lag_data, "tx_type", "proposal")?;
    let sync_lag_commitment = filter_rows(&sync_lag_data, "tx_type", "commitment")?;
    let sync_lag_accept = filter_rows(&sync_lag_data, "tx_type", "accept")?;
    let sync_lag_reject = filter_rows(&sync_lag_data, "tx_type", "reject")?;

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
        global_definition_propagation_time: partitioned_timing_stats(
            propagation_time,
            "value",
            "10s",
            &["agent", "arc"],
        )
        .context("Stats for global_definition_propagation_time")?,
        proposal_round_trip_accepted: partitioned_timing_stats(
            round_trip_accepted,
            "value",
            "10s",
            &["arc", "agent"],
        )
        .context("Round-trip timing (accepted)")?,
        proposal_round_trip_rejected: partitioned_timing_stats(
            round_trip_rejected,
            "value",
            "10s",
            &["arc", "agent"],
        )
        .context("Round-trip timing (rejected)")?,
        negotiation_rounds: partitioned_timing_stats(
            negotiation_data,
            "value",
            "10s",
            &["agent", "arc"],
        )
        .context("Stats for negotiation_rounds")?,
        sync_lag_proposal: partitioned_timing_stats(
            sync_lag_proposal,
            "value",
            "10s",
            &["arc", "agent"],
        )
        .context("Stats for sync_lag_proposal")?,
        sync_lag_commitment: partitioned_timing_stats(
            sync_lag_commitment,
            "value",
            "10s",
            &["arc", "agent"],
        )
        .context("Stats for sync_lag_commitment")?,
        sync_lag_accept: partitioned_timing_stats(
            sync_lag_accept,
            "value",
            "10s",
            &["arc", "agent"],
        )
        .context("Stats for sync_lag_accept")?,
        sync_lag_reject: partitioned_timing_stats(
            sync_lag_reject,
            "value",
            "10s",
            &["arc", "agent"],
        )
        .context("Stats for sync_lag_reject")?,
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
        ui_action_list_refresh_failed_calls: partitioned_counter_stats_allow_empty(
            ui_action_list_refresh_failed_calls,
            "value",
            "10s",
            &["agent"],
        )
        .context("Counter stats for ui_action_list_refresh_failed_calls")?,
        ui_transaction_detail_item_refresh_duration_s: standard_timing_stats(
            ui_transaction_detail_item_refresh_duration_s,
            "value",
            "10s",
            None,
        )
        .context("Timing stats for ui_transaction_detail_item_refresh_duration_s")?,
        ui_transaction_detail_refresh_duration_s: standard_timing_stats(
            ui_transaction_detail_refresh_duration_s,
            "value",
            "10s",
            None,
        )
        .context("Timing stats for ui_transaction_detail_refresh_duration_s")?,
        ui_transaction_detail_refresh_transactions_processed: partitioned_counter_stats(
            ui_transaction_detail_refresh_transactions_processed,
            "value",
            "10s",
            &["agent"],
        )
        .context("Counter stats for ui_transaction_detail_refresh_transactions_processed")?,
        ui_transaction_detail_refresh_primary_transaction_total_calls: partitioned_counter_stats(
            ui_transaction_detail_refresh_primary_transaction_total_calls,
            "value",
            "10s",
            &["agent"],
        )
        .context(
            "Counter stats for ui_transaction_detail_refresh_primary_transaction_total_calls",
        )?,
        ui_transaction_detail_refresh_primary_transaction_failed_calls: partitioned_counter_stats(
            ui_transaction_detail_refresh_primary_transaction_failed_calls,
            "value",
            "10s",
            &["agent"],
        )
        .context(
            "Counter stats for ui_transaction_detail_refresh_primary_transaction_failed_calls",
        )?,
        ui_transaction_detail_refresh_related_transaction_total_calls: partitioned_counter_stats(
            ui_transaction_detail_refresh_related_transaction_total_calls,
            "value",
            "10s",
            &["agent"],
        )
        .context(
            "Counter stats for ui_transaction_detail_refresh_related_transaction_total_calls",
        )?,
        ui_transaction_detail_refresh_related_transaction_failed_calls: partitioned_counter_stats(
            ui_transaction_detail_refresh_related_transaction_failed_calls,
            "value",
            "10s",
            &["agent"],
        )
        .context(
            "Counter stats for ui_transaction_detail_refresh_related_transaction_failed_calls",
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
