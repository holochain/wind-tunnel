use crate::aggregator::HostMetricsAggregator;
use crate::analyze::{counter_stats, gauge_stats, partitioned_gauge_stats, partitioned_rate_stats};
use crate::frame::LoadError;
use crate::model::{
    CounterStats, GaugeStats, PartitionedGaugeStats, PartitionedRateStats, PartitionedTimingStats,
    SummaryOutput,
};
use crate::{analyze, query};
use analyze::partitioned_timing_stats;
use anyhow::Context;
use polars::prelude::{col, lit, IntoLazy};
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MixedArcGetAgentActivitySummary {
    highest_observed_action_seq: CounterStats,
    chain_head_delay_timing: PartitionedTimingStats,
    chain_head_delay_rate: PartitionedRateStats,
    get_agent_activity_full_zome_calls: PartitionedTimingStats,
    retrieval_errors: GaugeStats,
    open_connections: PartitionedGaugeStats,
    error_count: usize,
}

pub(crate) async fn summarize_mixed_arc_get_agent_activity(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<SummaryOutput> {
    assert_eq!(summary.scenario_name, "mixed_arc_get_agent_activity");

    let highest_observed_action_seq = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.mixed_arc_get_agent_activity_highest_observed_action_seq",
        &["write_agent", "get_agent_activity_agent"],
    )
    .await
    .context("Load mixed_arc_get_agent_activity_highest_observed_action_seq data")?;

    let get_agent_activity_full_zome_calls =
        query::query_zome_call_instrument_data(client.clone(), &summary)
            .await
            .context("Load get_agent_activity_full zome call data")?
            .clone()
            .lazy()
            .filter(col("fn_name").eq(lit("get_agent_activity_full")))
            .collect()?;

    let chain_head_delay = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.mixed_arc_get_agent_activity_new_chain_head_delay",
        &[],
    )
    .await
    .context("Load chain head delay data")?;

    let retrieval_errors_stats = match query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.mixed_arc_get_agent_activity_retrieval_error",
        &["agent"],
    )
    .await
    .context("Load retrieval errors data")
    {
        Ok(df) => gauge_stats(df, "value")?,
        Err(e) => {
            if let Some(LoadError::NoSeriesInResult { .. }) = e.downcast_ref::<LoadError>() {
                // It is expected that there often is no error at all so if we find no series, return an empty count
                GaugeStats {
                    count: 0,
                    max: 0.0,
                    mean: 0.0,
                    min: 0.0,
                    std: 0.0,
                }
            } else {
                return Err(e);
            }
        }
    };

    let open_connections = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.mixed_arc_get_agent_activity_open_connections",
        &["behaviour"],
    )
    .await
    .context("Load open connections data")?;

    let host_metrics = HostMetricsAggregator::new(&client, &summary)
        .try_aggregate()
        .await;

    SummaryOutput::new(
        summary.clone(),
        MixedArcGetAgentActivitySummary {
            highest_observed_action_seq: counter_stats(highest_observed_action_seq, "value")
                .context("Highest observed action seq stats")?,
            chain_head_delay_timing: partitioned_timing_stats(
                chain_head_delay.clone(),
                "value",
                "10s",
                &[],
            )
            .context("Timing stats for chain head delay")?,
            chain_head_delay_rate: partitioned_rate_stats(chain_head_delay, "value", "10s", &[])
                .context("Rate stats for chain head delay")?,
            get_agent_activity_full_zome_calls: partitioned_timing_stats(
                get_agent_activity_full_zome_calls,
                "value",
                "10s",
                &["agent"],
            )
            .context("Timing stats for zome call get_agent_activity_full")?,
            retrieval_errors: retrieval_errors_stats,
            open_connections: partitioned_gauge_stats(open_connections, "value", &["behaviour"])
                .context("Open connections")?,
            error_count: query::zome_call_error_count(client, &summary).await?,
        },
        host_metrics,
    )
}
