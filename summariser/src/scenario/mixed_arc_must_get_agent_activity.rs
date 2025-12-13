use crate::aggregator::HostMetricsAggregator;
use crate::analyze::{counter_stats, partitioned_gauge_stats, partitioned_rate_stats};
use crate::model::{
    CounterStats, PartitionedGaugeStats, PartitionedRateStats, PartitionedTimingStats,
    SummaryOutput,
};
use crate::{analyze, query};
use analyze::partitioned_timing_stats;
use anyhow::Context;
use polars::prelude::{col, lit, IntoLazy};
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MixedArcMustGetAgentActivitySummary {
    retrieved_chain_len: CounterStats,
    chain_batch_delay_timing: PartitionedTimingStats,
    chain_batch_delay_rate: PartitionedRateStats,
    create_validated_sample_entry_zome_calls: PartitionedTimingStats,
    retrieval_errors: PartitionedTimingStats,
    open_connections: PartitionedGaugeStats,
    error_count: usize,
}

pub(crate) async fn summarize_mixed_arc_must_get_agent_activity(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<SummaryOutput> {
    assert_eq!(summary.scenario_name, "mixed_arc_must_get_agent_activity");

    let retrieved_chain_len = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.mixed_arc_must_get_agent_activity_chain_len",
        &["write_agent", "get_agent_activity_agent"],
    )
    .await
    .context("Load mixed_arc_get_agent_activity_highest_observed_action_seq data")?;

    let create_validated_sample_entry_zome_calls =
        query::query_zome_call_instrument_data(client.clone(), &summary)
            .await
            .context("Load create_validated_sample_entry zome call data")?
            .clone()
            .lazy()
            .filter(col("fn_name").eq(lit("create_validated_sample_entry")))
            .collect()?;

    let chain_batch_delay = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.mixed_arc_must_get_agent_activity_chain_batch_delay",
        &["write_agent", "must_get_agent_activity_agent"],
    )
    .await
    .context("Load chain batch delay data")?;

    let retrieval_errors_stats = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.mixed_arc_must_get_agent_activity_retrieval_error_count",
        &["agent"],
    )
    .await
    .context("Load retrieval errors data")?;

    let open_connections = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.mixed_arc_must_get_agent_activity_open_connections",
        &["behaviour"],
    )
    .await
    .context("Load open connections data")?;

    let host_metrics = HostMetricsAggregator::new(&client, &summary)
        .try_aggregate()
        .await;

    SummaryOutput::new(
        summary.clone(),
        MixedArcMustGetAgentActivitySummary {
            retrieved_chain_len: counter_stats(retrieved_chain_len, "value")
                .context("Highest observed action seq stats")?,
            chain_batch_delay_timing: partitioned_timing_stats(
                chain_batch_delay.clone(),
                "value",
                "10s",
                &["must_get_agent_activity_agent"],
            )
            .context("Timing stats for chain head delay")?,
            chain_batch_delay_rate: partitioned_rate_stats(
                chain_batch_delay,
                "value",
                "10s",
                &["must_get_agent_activity_agent"],
            )
            .context("Rate stats for chain head delay")?,
            create_validated_sample_entry_zome_calls: partitioned_timing_stats(
                create_validated_sample_entry_zome_calls,
                "value",
                "10s",
                &["agent"],
            )
            .context("Timing stats for zome call get_agent_activity_full")?,
            retrieval_errors: partitioned_timing_stats(
                retrieval_errors_stats.clone(),
                "value",
                "10s",
                &["agent"],
            )
            .context("Timing stats for retrieval errors")?,
            open_connections: partitioned_gauge_stats(open_connections, "value", &["behaviour"])
                .context("Open connections")?,
            error_count: query::zome_call_error_count(client, &summary).await?,
        },
        host_metrics,
    )
}
