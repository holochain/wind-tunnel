use crate::analyze::{
    delivery_ratio, partitioned_counter_stats_allow_empty, partitioned_rate_stats,
};
use crate::model::{PartitionedCounterStats, PartitionedRateStats, PartitionedTimingStats};
use crate::{analyze, query};
use analyze::partitioned_timing_stats;
use anyhow::Context;
use polars::prelude::{IntoLazy, col, lit};
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DhtSyncLagSummary {
    /// Rate of `created_timed_entry` zome calls per agent (calls per 10-second window)
    create_rate: PartitionedRateStats,
    /// DHT sync lag values per agent (seconds): time between an entry being created and it being
    /// observed by a reading agent.
    sync_lag_timing: PartitionedTimingStats,
    /// Rate at which sync lag observations are recorded per agent (observations per window)
    sync_lag_rate: PartitionedRateStats,
    /// Cumulative number of entries sent (written to DHT) per agent over the run
    sent_count: PartitionedCounterStats,
    /// Cumulative number of entries received (observed from DHT) per agent over the run
    recv_count: PartitionedCounterStats,
    /// Fraction of sent entries received across all readers: recv_count.total / (sent_count.total × reader_count) (0–1).
    ///
    /// Normalized by the number of receiving agents, so the ratio stays in [0, 1] regardless of
    /// how many readers there are. A value < 1 indicates data loss or incomplete propagation. Zero
    /// if nothing was sent or there were no receivers.
    ///
    /// Note that sending and receiving are not coordinated, so sending will continue until the
    /// scenario is stopped, and it's expected that some readers won't see the data before shutting
    /// down.
    delivery_ratio: f64,
    /// Number of zome call errors observed during the run
    error_count: usize,
}

pub(crate) async fn summarize_dht_sync_lag(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<DhtSyncLagSummary> {
    assert_eq!(summary.scenario_name, "dht_sync_lag");

    let create_zome_calls = query::query_zome_call_instrument_data(client.clone(), &summary)
        .await
        .context("Load send data")?
        .lazy()
        .filter(col("fn_name").eq(lit("created_timed_entry")))
        .collect()?;

    let sync_lag =
        query::query_custom_data(client.clone(), &summary, "wt.custom.sync_lag", &["agent"])
            .await
            .context("Load lag data")?;

    let sent_count_result =
        query::query_custom_data(client.clone(), &summary, "wt.custom.sent_count", &["agent"])
            .await;

    let recv_count_result =
        query::query_custom_data(client.clone(), &summary, "wt.custom.recv_count", &["agent"])
            .await;

    let sent_count =
        partitioned_counter_stats_allow_empty(sent_count_result, "value", "10s", &["agent"])
            .context("Counter stats for dht_sync_sent_count")?;
    let recv_count =
        partitioned_counter_stats_allow_empty(recv_count_result, "value", "10s", &["agent"])
            .context("Counter stats for dht_sync_recv_count")?;
    let delivery_ratio = delivery_ratio(sent_count.total_count, &recv_count);

    Ok(DhtSyncLagSummary {
        create_rate: partitioned_rate_stats(create_zome_calls, "value", "10s", &["agent"])
            .context("Rate stats for create")?,
        sync_lag_timing: partitioned_timing_stats(sync_lag.clone(), "value", "10s", &["agent"])
            .context("Timing stats for sync lag")?,
        sync_lag_rate: partitioned_rate_stats(sync_lag, "value", "10s", &["agent"])
            .context("Rate stats for sync lag")?,
        sent_count,
        recv_count,
        delivery_ratio,
        error_count: query::zome_call_error_count(client.clone(), &summary).await?,
    })
}
