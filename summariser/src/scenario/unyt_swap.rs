use crate::analyze::{
    aggregated_single_value, partitioned_counter_stats_allow_empty, partitioned_timing_stats,
};
use crate::model::{AggregatedSingleValue, PartitionedCounterStats, PartitionedTimingStats};
use crate::query;
use anyhow::Context;
use polars::prelude::{IntoLazy, col, lit};
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

/// Summary of a `unyt_swap` scenario run.
///
/// Covers both conversions the scenario exercises: the HOT -> wHOT bridging
/// flow (the bridge agent's oracle proof-of-deposit and deposit steps and the
/// bridge's collect step) and the wHOT -> HF swap flow (the bridge's commitment
/// and receipt and the swap agent's accept), together with the volume of
/// deposits, swaps, and receipts. The bridge role runs both flows each round,
/// so metrics for both are normally populated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct UnytSwapSummary {
    /// Duration of `create_parked_link` zome calls per agent (seconds).
    /// The bridge agent's oracle step that records a proof of deposit for each user.
    create_parked_link_zome_call: PartitionedTimingStats,

    /// Duration of `execute_rave` zome calls per agent (seconds).
    /// The bridging-agent step that executes the credit-limit and bridging
    /// agreements, turning a proof of deposit into a deposit RAVE.
    execute_rave_zome_call: PartitionedTimingStats,

    /// Duration of `create_parked_spend` zome calls per agent (seconds).
    /// The bridging-agent step that parks the spend on the bridging agreement
    /// before executing it.
    create_parked_spend_zome_call: PartitionedTimingStats,

    /// Duration of `create_collect_from_rave` zome calls per agent (seconds).
    /// The bridge's step that collects a deposit RAVE to receive wHOT.
    collect_from_rave_zome_call: PartitionedTimingStats,

    /// Duration of `get_incoming_raves` zome calls per agent (seconds).
    /// The bridge polls this every iteration, so latency here bounds collect throughput.
    get_incoming_raves_zome_call: PartitionedTimingStats,

    /// Proof-of-deposit parked links the bridge agent completed, as a cumulative
    /// per-agent counter. `total_count` reflects deposit volume offered across the run.
    bridge_parked_links_completed: PartitionedCounterStats,

    /// Proof-of-deposit parked links the bridge agent failed, as a cumulative
    /// per-agent counter. `total_count` reflects deposit volume offered across the run.
    bridge_parked_links_failed: PartitionedCounterStats,

    /// Deposit RAVEs the bridge collected, as a cumulative per-agent counter.
    /// `total_count` is the run-wide count of completed HOT -> wHOT conversions.
    deposit_raves_collected: PartitionedCounterStats,

    /// Total RAVE agreement executions across all agents at teardown.
    /// Reflects deposits that executed via the bridging agreement.
    completed_transaction_raves: AggregatedSingleValue,

    /// Duration of `create_commitment` zome calls per agent (seconds).
    /// The bridge's step committing to convert wHOT to HF.
    create_commitment_zome_call: PartitionedTimingStats,

    /// Duration of `create_accept` zome calls per agent (seconds).
    /// The swap agent's step accepting a swap commitment.
    create_accept_zome_call: PartitionedTimingStats,

    /// Duration of `create_receipt_for_accept` zome calls per agent (seconds).
    /// The bridge's step finalizing an accepted commitment.
    create_receipt_for_accept_zome_call: PartitionedTimingStats,

    /// Swap commitments the bridge created (wHOT -> HF offered), as a cumulative
    /// per-agent counter. `total_count` is the run-wide count of commitments created.
    swap_commitments_created: PartitionedCounterStats,

    /// Swap commitments the swap agent accepted, as a cumulative per-agent counter.
    /// `total_count` is the run-wide accepted count across all swap agents.
    commitments_accepted: PartitionedCounterStats,

    /// Swap receipts the bridge created for accepted commitments (completed wHOT -> HF),
    /// as a cumulative per-agent counter. `total_count` is the run-wide receipt count.
    swap_receipts_created: PartitionedCounterStats,

    /// End-to-end swap completion time per agent (seconds): from the bridge creating a
    /// commitment to it finalizing the matching accept with a receipt. Includes network
    /// propagation, the swap agent's accept latency, and the bridge's own poll gap, so it
    /// reflects the wall-clock time to complete a swap rather than pure protocol latency.
    swap_completion_duration_s: PartitionedTimingStats,

    /// Number of zome call errors observed during the run.
    error_count: usize,
}

pub(crate) async fn summarize_unyt_swap(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<UnytSwapSummary> {
    assert_eq!(summary.scenario_name, "unyt_swap");

    // --- Zome call timing data (single query, filtered per fn_name) ---

    let zome_calls = query::query_zome_call_instrument_data(client.clone(), &summary)
        .await
        .context("Load zome call instrument data")?;

    // --- Custom count metrics ---

    let bridge_parked_links_completed = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.bridge_parked_links_completed",
        &["agent"],
    )
    .await;

    let bridge_parked_links_failed = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.bridge_parked_links_failed",
        &["agent"],
    )
    .await;

    let deposit_raves_collected = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.deposit_raves_collected",
        &["agent"],
    )
    .await;

    let completed_transaction_raves = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.completed_transaction_raves",
        &[],
    )
    .await
    .context("Load completed_transaction_raves")?;

    let swap_commitments_created = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.swap_commitments_created",
        &["agent"],
    )
    .await;

    let commitments_accepted = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.commitments_accepted",
        &["agent"],
    )
    .await;

    let swap_receipts_created = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.swap_receipts_created",
        &["agent"],
    )
    .await;

    let swap_completion_duration_s = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.swap_completion_duration_s",
        &["agent"],
    )
    .await
    .context("Load swap completion duration")?;

    // --- Compute zome call stats per fn_name ---

    let partitioned_zome_call_stats = |fn_name: &str| -> anyhow::Result<PartitionedTimingStats> {
        let filtered = zome_calls
            .clone()
            .lazy()
            .filter(col("fn_name").eq(lit(fn_name)))
            .collect()?;
        partitioned_timing_stats(filtered, "value", "10s", &["agent"])
            .context(format!("Timing stats for zome call {fn_name}"))
    };

    Ok(UnytSwapSummary {
        create_parked_link_zome_call: partitioned_zome_call_stats("create_parked_link")?,
        execute_rave_zome_call: partitioned_zome_call_stats("execute_rave")?,
        create_parked_spend_zome_call: partitioned_zome_call_stats("create_parked_spend")?,
        collect_from_rave_zome_call: partitioned_zome_call_stats("create_collect_from_rave")?,
        get_incoming_raves_zome_call: partitioned_zome_call_stats("get_incoming_raves")?,
        bridge_parked_links_completed: partitioned_counter_stats_allow_empty(
            bridge_parked_links_completed,
            "value",
            "10s",
            &["agent"],
        )
        .context("Counter stats for bridge_parked_links_completed")?,
        bridge_parked_links_failed: partitioned_counter_stats_allow_empty(
            bridge_parked_links_failed,
            "value",
            "10s",
            &["agent"],
        )
        .context("Counter stats for bridge_parked_links_failed")?,
        deposit_raves_collected: partitioned_counter_stats_allow_empty(
            deposit_raves_collected,
            "value",
            "10s",
            &["agent"],
        )
        .context("Counter stats for deposit_raves_collected")?,
        completed_transaction_raves: aggregated_single_value(completed_transaction_raves, "value")
            .context("Aggregated value for completed_transaction_raves")?,
        create_commitment_zome_call: partitioned_zome_call_stats("create_commitment")?,
        create_accept_zome_call: partitioned_zome_call_stats("create_accept")?,
        create_receipt_for_accept_zome_call: partitioned_zome_call_stats(
            "create_receipt_for_accept",
        )?,
        swap_commitments_created: partitioned_counter_stats_allow_empty(
            swap_commitments_created,
            "value",
            "10s",
            &["agent"],
        )
        .context("Counter stats for swap_commitments_created")?,
        commitments_accepted: partitioned_counter_stats_allow_empty(
            commitments_accepted,
            "value",
            "10s",
            &["agent"],
        )
        .context("Counter stats for commitments_accepted")?,
        swap_receipts_created: partitioned_counter_stats_allow_empty(
            swap_receipts_created,
            "value",
            "10s",
            &["agent"],
        )
        .context("Counter stats for swap_receipts_created")?,
        swap_completion_duration_s: partitioned_timing_stats(
            swap_completion_duration_s,
            "value",
            "10s",
            &["agent"],
        )
        .context("Timing stats for swap_completion_duration_s")?,
        error_count: query::zome_call_error_count(client.clone(), &summary).await?,
    })
}
