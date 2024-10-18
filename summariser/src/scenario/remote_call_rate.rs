use crate::model::{PartitionedTimingStats, SummaryOutput};
use crate::{analyze, query};
use analyze::partitioned_timing_stats;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RemoteCallRateSummary {
    dispatch_timing: PartitionedTimingStats,
    round_trip_timing: PartitionedTimingStats,
    error_count: usize,
}

pub(crate) async fn summarize_remote_call_rate(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<SummaryOutput> {
    assert_eq!(summary.scenario_name, "remote_call_rate");

    let dispatch_frame = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.remote_call_dispatch",
        &["agent"],
    )
    .await
    .context("Load dispatch data")?;

    let round_trip_frame = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.remote_call_round_trip",
        &["agent"],
    )
    .await
    .context("Load round trip data")?;

    SummaryOutput::new(
        summary.clone(),
        RemoteCallRateSummary {
            dispatch_timing: partitioned_timing_stats(dispatch_frame, "value", "10s", &["agent"])
                .context("Timing stats for dispatch")?,
            round_trip_timing: partitioned_timing_stats(round_trip_frame, "value", "10s", &["agent"])
                .context("Timing stats for round trip")?,
            error_count: query::zome_call_error_count(client.clone(), &summary).await?,
        },
    )
}
