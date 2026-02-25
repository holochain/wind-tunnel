use crate::analyze::{
    chain_head_stats, partitioned_counter_stats_allow_empty, partitioned_rate_stats,
    partitioned_timing_stats,
};
use crate::model::{
    ChainHeadStats, PartitionedCounterStats, PartitionedRateStats, PartitionedTimingStats,
};
use crate::query;
use crate::query::holochain_p2p_metrics::{HolochainP2pMetrics, query_holochain_p2p_metrics};
use anyhow::Context;
use polars::prelude::{IntoLazy, col, lit};
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WriteValidatedMustGetAgentActivitySummary {
    /// Maximum chain length observed per write agent, aggregated across all reading agents.
    ///
    /// For each write agent, the maximum chain length seen by any reader is taken (collapsing
    /// the reader dimension). The resulting per-write-agent maxima are then summarised with
    /// `mean_max` and `max` across all write agents.
    chain_len: ChainHeadStats,
    /// Distribution of wall-clock delay between a batch of entries being written and the
    /// must_get_agent_activity agent observing them (seconds); includes validation and DHT time
    chain_batch_delay_timing: PartitionedTimingStats,
    /// Rate of chain batch delay observations per agent (observations per window)
    chain_batch_delay_rate: PartitionedRateStats,
    /// Duration of `create_validated_sample_entry` zome calls per agent (seconds)
    create_validated_sample_entry_zome_calls: PartitionedTimingStats,
    /// Cumulative count of retrieval errors per agent; empty if no errors occurred
    retrieval_errors: PartitionedCounterStats,
    /// Number of zome call errors observed during the run
    error_count: usize,
    /// Holochain p2p network metrics for the run
    holochain_p2p_metrics: HolochainP2pMetrics,
}

pub(crate) async fn summarize_write_validated_must_get_agent_activity(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<WriteValidatedMustGetAgentActivitySummary> {
    assert_eq!(
        summary.scenario_name,
        "write_validated_must_get_agent_activity"
    );

    let zome_calls = query::query_zome_call_instrument_data(client.clone(), &summary)
        .await
        .context("Load zome call data")?;

    let chain_len_frame = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.chain_len",
        &["write_agent", "agent"],
    )
    .await
    .context("Load chain_len data")?;

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

    let create_validated_sample_entry_zome_calls = zome_calls
        .clone()
        .lazy()
        .filter(col("fn_name").eq(lit("create_validated_sample_entry")))
        .collect()?;

    Ok(WriteValidatedMustGetAgentActivitySummary {
        chain_len: chain_head_stats(chain_len_frame, "value", "write_agent", "10s")
            .context("Chain head stats for chain_len")?,
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
        .context("Rate stats for chain head delay")?,
        create_validated_sample_entry_zome_calls: partitioned_timing_stats(
            create_validated_sample_entry_zome_calls,
            "value",
            "10s",
            &["agent"],
        )
        .context("Write create_validated_sample_entry_zome_calls stats")?,
        retrieval_errors: partitioned_counter_stats_allow_empty(
            retrieval_errors_frame_result,
            "value",
            "10s",
            &["agent"],
        )
        .context("Counter stats for retrieval errors")?,
        error_count: query::zome_call_error_count(client.clone(), &summary).await?,
        holochain_p2p_metrics: query_holochain_p2p_metrics(&client, &summary).await?,
    })
}
