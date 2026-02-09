use crate::{
    analyze::{counter_stats, standard_timing_stats},
    model::{CounterStats, StandardTimingsStats},
    query::{
        self,
        holochain_p2p_metrics::{HolochainP2pMetrics, query_holochain_p2p_metrics},
    },
};
use anyhow::Context as _;
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RemoteSignalsSummary {
    remote_signal_round_trip: StandardTimingsStats,
    remote_signal_timeout: Option<CounterStats>,
    holochain_p2p_metrics: HolochainP2pMetrics,
}

pub(crate) async fn summarize_remote_signals(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<RemoteSignalsSummary> {
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

    // timeouts might be empty if there were no timeouts
    let remote_signal_timeout = if remote_signal_timeout.is_empty() {
        None
    } else {
        Some(counter_stats(remote_signal_timeout, "value").context("Timeout stats")?)
    };

    Ok(RemoteSignalsSummary {
        remote_signal_round_trip: standard_timing_stats(
            remote_signal_round_trip_frame,
            "value",
            "10s",
            None,
        )
        .context("Send timing stats")?,
        remote_signal_timeout,
        holochain_p2p_metrics: query_holochain_p2p_metrics(&client, &summary).await?,
    })
}
