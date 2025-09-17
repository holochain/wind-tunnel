use crate::aggregator::HostMetricsAggregator;
use crate::model::{PartitionedTimingStats, SummaryOutput};
use crate::{analyze, query};
use analyze::partitioned_timing_stats;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WriteGetAgentActivitySummary {
    timing: PartitionedTimingStats,
    error_count: usize,
}

pub(crate) async fn summarize_write_get_agent_activity(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<SummaryOutput> {
    assert_eq!(summary.scenario_name, "write_get_agent_activity");

    let frame = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.write_get_agent_activity",
        &["agent"],
    )
    .await
    .context("Load write_get_agent_activity data")?;

    let host_metrics = HostMetricsAggregator::new(&client, &summary)
        .try_aggregate()
        .await;

    SummaryOutput::new(
        summary.clone(),
        WriteGetAgentActivitySummary {
            timing: partitioned_timing_stats(frame, "value", "10s", &["agent"])
                .context("Timing stats for get_agent_activity")?,
            error_count: query::zome_call_error_count(client, &summary).await?,
        },
        host_metrics,
    )
}
