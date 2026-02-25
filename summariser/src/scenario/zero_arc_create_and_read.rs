use crate::analyze::{
    partitioned_counter_stats_allow_empty, partitioned_gauge_stats, partitioned_rate_stats,
    partitioned_timing_stats, round_to_n_dp,
};
use crate::model::{
    PartitionedCounterStats, PartitionedGaugeStats, PartitionedRateStats, PartitionedTimingStats,
};
use crate::query;
use crate::query::holochain_p2p_metrics::{
    HolochainP2pMetricsWithCounts, query_holochain_p2p_metrics_with_counts,
};
use anyhow::Context;
use polars::prelude::{IntoLazy, col, lit};
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ZeroArcCreateAndReadSummary {
    // Scenario metrics
    /// Rate of `created_timed_entry` zome calls per agent (calls per window)
    create_rate: PartitionedRateStats,
    /// Distribution of fetch lag per agent (seconds): time from creation to observation by a
    /// zero-arc agent actively fetching from the network
    fetch_lag_timing: PartitionedTimingStats,
    /// Rate of fetch lag observations per agent (observations per window)
    fetch_lag_rate: PartitionedRateStats,
    /// Distribution of open DHT connection counts, partitioned by arc value
    open_connections: PartitionedGaugeStats,
    /// Cumulative count of entries created (written to DHT) per agent over the run
    entry_created_count: PartitionedCounterStats,
    /// Cumulative count of entries received (observed from DHT) per agent over the run
    recv_count: PartitionedCounterStats,
    /// Fraction of created entries received across all readers: `recv_count.total / (entry_created_count.total × reader_count)` `[0–1]`.
    ///
    /// Normalized by the number of receiving agents, so the ratio stays in [0, 1] regardless of
    /// how many readers there are. A value < 1 indicates data loss or incomplete propagation.
    /// Zero if nothing was created or there were no receivers.
    delivery_ratio: f64,
    /// Number of zome call errors observed during the run
    error_count: usize,
    /// Holochain p2p network metrics including operation counts for the run
    holochain_p2p_metrics: HolochainP2pMetricsWithCounts,
}

pub(crate) async fn summarize_zero_arc_create_and_read(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<ZeroArcCreateAndReadSummary> {
    assert_eq!(summary.scenario_name, "zero_arc_create_and_read");

    let create_zome_calls = query::query_zome_call_instrument_data(client.clone(), &summary)
        .await
        .context("Load send data")?
        .lazy()
        .filter(col("fn_name").eq(lit("created_timed_entry")))
        .collect()?;

    let fetch_lag =
        query::query_custom_data(client.clone(), &summary, "wt.custom.fetch_lag", &["agent"])
            .await
            .context("Load lag data")?;

    let open_connections = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.open_connections",
        &["arc"],
    )
    .await
    .context("Load open connections data")?;

    let entry_created_count_result = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.entry_created_count",
        &["agent"],
    )
    .await;

    let recv_count_result =
        query::query_custom_data(client.clone(), &summary, "wt.custom.recv_count", &["agent"])
            .await;

    let entry_created_count = partitioned_counter_stats_allow_empty(
        entry_created_count_result,
        "value",
        "10s",
        &["agent"],
    )
    .context("Counter stats for entry created count")?;
    let recv_count =
        partitioned_counter_stats_allow_empty(recv_count_result, "value", "10s", &["agent"])
            .context("Counter stats for recv count")?;
    let delivery_ratio = if entry_created_count.total_count > 0 && recv_count.partition_count > 0 {
        round_to_n_dp(
            (recv_count.total_count as f64
                / (entry_created_count.total_count as f64 * recv_count.partition_count as f64))
                .min(1.0),
            4,
        )
    } else {
        0.0
    };

    Ok(ZeroArcCreateAndReadSummary {
        create_rate: partitioned_rate_stats(create_zome_calls, "value", "10s", &["agent"])
            .context("Rate stats for create")?,
        fetch_lag_timing: partitioned_timing_stats(fetch_lag.clone(), "value", "10s", &["agent"])
            .context("Timing stats for fetch lag")?,
        fetch_lag_rate: partitioned_rate_stats(fetch_lag, "value", "10s", &["agent"])
            .context("Rate stats for fetch lag")?,
        open_connections: partitioned_gauge_stats(open_connections, "value", &["arc"], "10s")
            .context("Open connections")?,
        entry_created_count,
        recv_count,
        delivery_ratio,
        error_count: query::zome_call_error_count(client.clone(), &summary).await?,
        holochain_p2p_metrics: query_holochain_p2p_metrics_with_counts(&client, &summary).await?,
    })
}
