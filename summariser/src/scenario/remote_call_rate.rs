use crate::model::PartitionedTimingStats;
use crate::query::holochain_p2p_metrics::{HolochainP2pMetrics, query_holochain_p2p_metrics};
use crate::{analyze, query};
use analyze::partitioned_timing_stats;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RemoteCallRateSummary {
    dispatch_timing: PartitionedTimingStats,
    round_trip_timing: PartitionedTimingStats,
    error_count: usize,
    holochain_p2p_metrics: HolochainP2pMetrics,
}

pub(crate) async fn summarize_remote_call_rate(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<RemoteCallRateSummary> {
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

    Ok(RemoteCallRateSummary {
        dispatch_timing: partitioned_timing_stats(dispatch_frame, "value", "10s", &["agent"])
            .context("Timing stats for dispatch")?,
        round_trip_timing: partitioned_timing_stats(round_trip_frame, "value", "10s", &["agent"])
            .context("Timing stats for round trip")?,
        error_count: query::zome_call_error_count(client.clone(), &summary).await?,
        holochain_p2p_metrics: query_holochain_p2p_metrics(&client, &summary).await?,
    })
}
