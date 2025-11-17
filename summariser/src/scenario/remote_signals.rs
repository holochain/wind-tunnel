use anyhow::Context as _;
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

use crate::{
    aggregator::HostMetricsAggregator,
    analyze::{counter_stats, standard_timing_stats},
    model::{CounterStats, StandardTimingsStats, SummaryOutput},
    query,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RemoteSignalsSummary {
    remote_signal_round_trip: StandardTimingsStats,
    remote_signal_timeout: CounterStats,
}

pub(crate) async fn summarize_remote_signals(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<SummaryOutput> {
    assert_eq!(summary.scenario_name, "remote_signals");

    let remote_signal_round_trip_frame = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.remote_signal_round_trip",
        &[],
    )
    .await
    .context("Load send data")?;

    let remote_signal_timeout = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.remote_signal_timeout",
        &[],
    )
    .await
    .context("Load success ratio")?;

    let host_metrics = HostMetricsAggregator::new(&client, &summary)
        .try_aggregate()
        .await;

    SummaryOutput::new(
        summary,
        RemoteSignalsSummary {
            remote_signal_round_trip: standard_timing_stats(
                remote_signal_round_trip_frame,
                "value",
                "10s",
                None,
            )
            .context("Send timing stats")?,
            remote_signal_timeout: counter_stats(remote_signal_timeout, "value")
                .context("Timeout stats")?,
        },
        host_metrics,
    )
}
