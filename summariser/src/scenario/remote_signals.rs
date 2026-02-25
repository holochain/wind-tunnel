use crate::{
    analyze::{counter_stats, round_to_n_dp, standard_timing_stats},
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
    /// Distribution of remote signal round-trip durations (seconds): time between sending a
    /// signal request and receiving the response signal
    remote_signal_round_trip: StandardTimingsStats,
    /// Cumulative count of timed-out remote signals over the run; None if no timeouts occurred
    remote_signal_timeout: Option<CounterStats>,
    /// Fraction of signal attempts that timed out: timeouts / (round_trips + timeouts) (0–1).
    ///
    /// Zero when no timeouts occurred. Values > 0 indicate reliability issues.
    timeout_rate: f64,
    /// Holochain p2p network metrics for the run
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
        Some(counter_stats(remote_signal_timeout, "value", "10s").context("Timeout stats")?)
    };

    let round_trip_count = remote_signal_round_trip_frame.height() as u64;
    let timeout_count = remote_signal_timeout.as_ref().map(|t| t.count).unwrap_or(0);
    let total = round_trip_count + timeout_count;
    let timeout_rate = if total > 0 {
        round_to_n_dp(timeout_count as f64 / total as f64, 4)
    } else {
        0.0
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
        timeout_rate,
        holochain_p2p_metrics: query_holochain_p2p_metrics(&client, &summary).await?,
    })
}
