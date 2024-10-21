use crate::analyze::partitioned_rate_stats;
use crate::model::{PartitionedRateStats, PartitionedTimingStats, SummaryOutput};
use crate::{analyze, query};
use analyze::partitioned_timing_stats;
use anyhow::Context;
use polars::prelude::{col, lit, IntoLazy};
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DhtSyncLagSummary {
    create_rate: PartitionedRateStats,
    sync_lag_timing: PartitionedTimingStats,
    sync_lag_rate: PartitionedRateStats,
    error_count: usize,
}

pub(crate) async fn summarize_dht_sync_lag(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<SummaryOutput> {
    assert_eq!(summary.scenario_name, "dht_sync_lag");

    let create_zome_calls = query::query_zome_call_instrument_data(client.clone(), &summary)
        .await
        .context("Load send data")?
        .clone()
        .lazy()
        .filter(col("fn_name").eq(lit("created_timed_entry")))
        .collect()?;

    let sync_lag = query::query_custom_data(client.clone(), &summary, "wt.custom.dht_sync_lag", &["agent"])
        .await
        .context("Load lag data")?;

    SummaryOutput::new(
        summary.clone(),
        DhtSyncLagSummary {
            create_rate: partitioned_rate_stats(create_zome_calls, "value", "10s", &["agent"])
            .context("Rate stats for create")?,
            sync_lag_timing: partitioned_timing_stats(sync_lag.clone(), "value", "10s", &["agent"])
                .context("Timing stats for sync lag")?,
            sync_lag_rate: partitioned_rate_stats(sync_lag, "value", "10s", &["agent"])
                .context("Rate stats for sync lag")?,
            error_count: query::zome_call_error_count(client.clone(), &summary).await?,
        },
    )
}
