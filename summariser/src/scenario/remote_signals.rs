use anyhow::Context as _;
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

use crate::query::holochain_metrics::{
    query_p2p_handle_request_duration, query_p2p_handle_request_ignored_count,
    query_p2p_request_duration,
};
use crate::{
    aggregator::HostMetricsAggregator,
    analyze::{counter_stats, standard_timing_stats},
    model::{CounterStats, StandardTimingsStats, SummaryOutput},
    query,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RemoteSignalsSummary {
    remote_signal_round_trip: StandardTimingsStats,
    remote_signal_timeout: Option<CounterStats>,
    p2p_request_duration: Option<StandardTimingsStats>,
    p2p_handle_request_duration: Option<StandardTimingsStats>,
    p2p_handle_request_ignored_count: u64,
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

    // this might be empty if there were no timeouts
    let remote_signal_timeout = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.remote_signal_timeout",
        &[],
    )
    .await
    .unwrap_or_default();

    let host_metrics = HostMetricsAggregator::new(&client, &summary)
        .try_aggregate()
        .await;

    // timeouts might be empty if there were no timeouts
    let remote_signal_timeout = if remote_signal_timeout.is_empty() {
        None
    } else {
        Some(counter_stats(remote_signal_timeout, "value").context("Timeout stats")?)
    };

    SummaryOutput::new(
        summary.clone(),
        RemoteSignalsSummary {
            remote_signal_round_trip: standard_timing_stats(
                remote_signal_round_trip_frame,
                "value",
                "10s",
                None,
            )
            .context("Send timing stats")?,
            remote_signal_timeout,
            p2p_request_duration: query_p2p_request_duration(&client, &summary).await?,
            p2p_handle_request_duration: query_p2p_handle_request_duration(&client, &summary)
                .await?,
            p2p_handle_request_ignored_count: query_p2p_handle_request_ignored_count(
                &client, &summary,
            )
            .await?,
        },
        host_metrics,
    )
}
