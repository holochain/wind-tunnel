use crate::analyze::{allow_no_series, event_counts_by_tag, round_to_n_dp, standard_timing_stats};
use crate::model::StandardTimingsStats;
use crate::query;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use wind_tunnel_summary_model::RunSummary;

/// Percentage (0-100) of established connections that were upgraded to a direct one.
///
/// The caller must pass at least one connection: the connections query fails when a run
/// established none, so a zero total never reaches this function.
fn hole_punch_success_pct(direct: u64, relayed: u64) -> f64 {
    round_to_n_dp(direct as f64 / (direct + relayed) as f64 * 100.0, 2)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PeerkitHolePunchSummary {
    /// Percentage (0-100) of established connections that were upgraded to a direct connection
    /// before the upgrade timeout: `direct / (direct + relayed)`.
    ///
    /// Higher is better. Connections that were lost, never connected, or never reported a type
    /// are not part of this ratio; they are counted in `errors_by_kind` instead. A run that
    /// established no connection at all fails to summarise instead of reporting 0.0.
    hole_punch_success_pct: f64,
    /// Number of successful dials whose connection was upgraded to a direct one (hole punch
    /// succeeded).
    ///
    /// Each dial is one connect call by one agent, so this counts attempts, not unique pairs of
    /// agents. Two agents dialing each other at the same moment both record the connection they
    /// share.
    direct_dials: u64,
    /// Number of successful dials whose connection was still going through the relay when the
    /// upgrade timeout elapsed (hole punch failed or was too slow).
    relayed_dials: u64,
    /// Time in seconds taken by each successful connect call.
    ///
    /// This covers establishing the connection, not the later upgrade to a direct one.
    connect_timing: StandardTimingsStats,
    /// Time in seconds taken by each successful disconnect call.
    disconnect_timing: StandardTimingsStats,
    /// Time in seconds between a node connecting to the relay and it discovering each peer.
    ///
    /// The measurement is relative to the discovering node's own relay connection, so peers
    /// that start later inflate it: it includes startup skew, not just discovery latency.
    peer_discovery_time: StandardTimingsStats,
    /// Total number of errors recorded by all agents during the run.
    error_count: u64,
    /// Number of errors per kind. Kinds with no errors are absent.
    ///
    /// `connection_lost` is expected in small networks, where agents often dial each other at
    /// the same time and one hangs up first. `never_connected` points at dials failing
    /// silently, and `behaviour` counts terminal errors that stopped an agent.
    errors_by_kind: BTreeMap<String, u64>,
}

pub(crate) async fn summarize_peerkit_hole_punch(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<PeerkitHolePunchSummary> {
    assert_eq!(summary.scenario_name, "peerkit_hole_punch");

    let connections = query::query_metrics_fields(
        &client,
        &summary,
        "wt.custom.peerkit_connection_established",
        &["count"],
        &["type"],
        None,
    )
    .await
    .context("Load connection established data")?;
    let connections_by_type = event_counts_by_tag(&connections, "type")?;
    let direct_dials = connections_by_type.get("direct").copied().unwrap_or(0);
    let relayed_dials = connections_by_type.get("relayed").copied().unwrap_or(0);

    let errors = allow_no_series(
        query::query_metrics_fields(
            &client,
            &summary,
            "wt.custom.peerkit_error_count",
            &["count"],
            &["kind"],
            None,
        )
        .await,
    )
    .context("Load error count data")?;
    let errors_by_kind = match errors {
        Some(errors) => event_counts_by_tag(&errors, "kind")?,
        None => BTreeMap::new(),
    };

    let connect = query::query_instrument_data(client.clone(), &summary, "connect")
        .await
        .context("Load connect instrument data")?;
    let disconnect = query::query_instrument_data(client.clone(), &summary, "disconnect")
        .await
        .context("Load disconnect instrument data")?;

    let discovery = query::query_metrics_fields(
        &client,
        &summary,
        "wt.custom.peerkit_peer_discovery_time",
        &["value_s"],
        &[],
        None,
    )
    .await
    .context("Load peer discovery time data")?;

    Ok(PeerkitHolePunchSummary {
        hole_punch_success_pct: hole_punch_success_pct(direct_dials, relayed_dials),
        direct_dials,
        relayed_dials,
        connect_timing: standard_timing_stats(connect, "value", "10s", None)
            .context("Timing stats for connect")?,
        disconnect_timing: standard_timing_stats(disconnect, "value", "10s", None)
            .context("Timing stats for disconnect")?,
        peer_discovery_time: standard_timing_stats(discovery, "value_s", "10s", None)
            .context("Timing stats for peer discovery")?,
        error_count: errors_by_kind.values().sum(),
        errors_by_kind,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_pct_is_share_of_direct_dials() {
        assert_eq!(hole_punch_success_pct(3, 1), 75.0);
        assert_eq!(hole_punch_success_pct(0, 4), 0.0);
        assert_eq!(hole_punch_success_pct(4, 0), 100.0);
        assert_eq!(hole_punch_success_pct(1, 2), 33.33);
    }
}
