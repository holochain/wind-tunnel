use crate::analyze::{
    chain_head_stats, partitioned_counter_stats_allow_empty, partitioned_gauge_stats,
    partitioned_rate_stats, partitioned_timing_stats,
};
use crate::model::{
    ChainHeadStats, PartitionedCounterStats, PartitionedGaugeStats, PartitionedRateStats,
    PartitionedTimingStats,
};
use crate::query;
use crate::query::holochain_p2p_metrics::{
    HolochainP2pMetricsWithCounts, query_holochain_p2p_metrics_with_counts,
};
use anyhow::Context;
use polars::prelude::{IntoLazy, col, lit};
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MixedArcGetAgentActivitySummary {
    /// Total entries created by write agents, partitioned by agent and behaviour
    entry_created_count: PartitionedCounterStats,
    /// Maximum highest-observed action sequence per write agent, aggregated across all reading agents.
    ///
    /// For each write agent, the maximum action sequence seen by any reader is taken (collapsing
    /// the reader dimension). The resulting per-write-agent maxima are then summarised with
    /// `mean_max` and `max` across all write agents. Captures how far readers tracked writers'
    /// chains during the run.
    highest_observed_action_seq: ChainHeadStats,
    /// Distribution of wall-clock delay between a new chain head being published and observed
    /// by a reading agent (seconds); includes DHT propagation time by a reading agent (seconds);
    /// includes DHT propagation time and any clock skew between writer and reading agent.
    chain_head_delay_timing: PartitionedTimingStats,
    /// Rate at which new chain head observations are recorded per agent (observations per window)
    chain_head_delay_rate: PartitionedRateStats,
    /// Duration of `get_agent_activity_full` zome calls per agent (seconds)
    get_agent_activity_full_zome_calls: PartitionedTimingStats,
    /// Cumulative count of retrieval errors per agent; empty if no errors occurred
    retrieval_errors: PartitionedCounterStats,
    /// Distribution of open DHT connection counts, partitioned by network arc behaviour
    open_connections: PartitionedGaugeStats,
    /// Number of zome call errors observed during the run
    error_count: usize,
    /// Holochain p2p network metrics including operation counts for the run
    holochain_p2p_metrics: HolochainP2pMetricsWithCounts,
}

pub(crate) async fn summarize_mixed_arc_get_agent_activity(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<MixedArcGetAgentActivitySummary> {
    assert_eq!(summary.scenario_name, "mixed_arc_get_agent_activity");

    let entry_created_count_result = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.entry_created_count",
        &["agent", "behaviour"],
    )
    .await;

    let highest_observed_action_seq_frame = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.highest_observed_action_seq",
        &["write_agent", "agent"],
    )
    .await
    .context("Load highest_observed_action_seq data")?;

    let get_agent_activity_full_zome_calls =
        query::query_zome_call_instrument_data(client.clone(), &summary)
            .await
            .context("Load get_agent_activity_full zome call data")?
            .lazy()
            .filter(col("fn_name").eq(lit("get_agent_activity_full")))
            .collect()?;

    let chain_head_delay = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.chain_head_delay",
        &["agent"],
    )
    .await
    .context("Load chain head delay data")?;

    let retrieval_errors_frame_result = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.retrieval_error_count",
        &["agent"],
    )
    .await;

    let open_connections = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.open_connections",
        &["behaviour"],
    )
    .await
    .context("Load open connections data")?;

    Ok(MixedArcGetAgentActivitySummary {
        entry_created_count: partitioned_counter_stats_allow_empty(
            entry_created_count_result,
            "value",
            "10s",
            &["agent", "behaviour"],
        )
        .context("Counter stats for entry created count")?,
        highest_observed_action_seq: chain_head_stats(
            highest_observed_action_seq_frame,
            "value",
            "write_agent",
            "10s",
        )
        .context("Chain head stats for highest_observed_action_seq")?,
        chain_head_delay_timing: partitioned_timing_stats(
            chain_head_delay.clone(),
            "value",
            "10s",
            &["agent"],
        )
        .context("Timing stats for chain head delay")?,
        chain_head_delay_rate: partitioned_rate_stats(chain_head_delay, "value", "10s", &["agent"])
            .context("Rate stats for chain head delay")?,
        get_agent_activity_full_zome_calls: partitioned_timing_stats(
            get_agent_activity_full_zome_calls,
            "value",
            "10s",
            &["agent"],
        )
        .context("Timing stats for zome call get_agent_activity_full")?,
        retrieval_errors: partitioned_counter_stats_allow_empty(
            retrieval_errors_frame_result,
            "value",
            "10s",
            &["agent"],
        )
        .context("Counter stats for retrieval errors")?,
        open_connections: partitioned_gauge_stats(open_connections, "value", &["behaviour"], "10s")
            .context("Open connections")?,
        error_count: query::zome_call_error_count(client.clone(), &summary).await?,
        holochain_p2p_metrics: query_holochain_p2p_metrics_with_counts(&client, &summary).await?,
    })
}
