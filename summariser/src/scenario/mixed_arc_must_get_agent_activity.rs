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
pub(crate) struct MixedArcMustGetAgentActivitySummary {
    /// Maximum chain length observed per write agent, aggregated across all reading agents.
    ///
    /// For each write agent, the maximum chain length seen by any reader is taken (collapsing
    /// the reader dimension). The resulting per-write-agent maxima are then summarised with
    /// `mean_max` and `max` across all write agents.
    retrieved_chain_len: ChainHeadStats,
    /// Distribution of wall-clock delay between a batch of entries being written and the
    /// must_get_agent_activity agent observing them (seconds); includes validation and DHT time
    chain_batch_delay_timing: PartitionedTimingStats,
    /// Rate of chain batch delay observations per must_get_agent_activity agent (observations per window)
    chain_batch_delay_rate: PartitionedRateStats,
    /// Duration of `create_validated_sample_entry` zome calls per agent (seconds)
    create_validated_sample_entry_zome_calls: PartitionedTimingStats,
    /// Cumulative count of retrieval errors per agent; empty if no errors occurred
    retrieval_errors: PartitionedCounterStats,
    /// Distribution of open DHT connection counts, partitioned by network arc behaviour
    open_connections: PartitionedGaugeStats,
    /// Number of zome call errors observed during the run
    error_count: usize,
    /// Holochain p2p network metrics including operation counts for the run
    holochain_p2p_metrics: HolochainP2pMetricsWithCounts,
}

pub(crate) async fn summarize_mixed_arc_must_get_agent_activity(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<MixedArcMustGetAgentActivitySummary> {
    assert_eq!(summary.scenario_name, "mixed_arc_must_get_agent_activity");

    let retrieved_chain_len = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.chain_len",
        &["write_agent", "agent"],
    )
    .await
    .context("Load chain_len data")?;

    let create_validated_sample_entry_zome_calls =
        query::query_zome_call_instrument_data(client.clone(), &summary)
            .await
            .context("Load create_validated_sample_entry zome call data")?
            .lazy()
            .filter(col("fn_name").eq(lit("create_validated_sample_entry")))
            .collect()?;

    let chain_batch_delay = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.chain_batch_delay",
        &["write_agent", "agent"],
    )
    .await
    .context("Load chain batch delay data")?;

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

    Ok(MixedArcMustGetAgentActivitySummary {
        retrieved_chain_len: chain_head_stats(retrieved_chain_len, "value", "write_agent", "10s")
            .context("Chain head stats for retrieved_chain_len")?,
        chain_batch_delay_timing: partitioned_timing_stats(
            chain_batch_delay.clone(),
            "value",
            "10s",
            &["agent"],
        )
        .context("Timing stats for chain batch delay")?,
        chain_batch_delay_rate: partitioned_rate_stats(
            chain_batch_delay,
            "value",
            "10s",
            &["agent"],
        )
        .context("Rate stats for chain batch delay")?,
        create_validated_sample_entry_zome_calls: partitioned_timing_stats(
            create_validated_sample_entry_zome_calls,
            "value",
            "10s",
            &["agent"],
        )
        .context("Timing stats for zome call create_validated_sample_entry")?,
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
