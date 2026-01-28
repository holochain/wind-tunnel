use crate::aggregator::HostMetricsAggregator;
use crate::analyze::{partitioned_gauge_stats, partitioned_rate_stats, partitioned_timing_stats};
use crate::model::{
    PartitionedGaugeStats, PartitionedRateStats, PartitionedTimingStats, StandardTimingsStats,
    SummaryOutput,
};
use crate::query;
use crate::query::holochain_metrics::{
    query_p2p_handle_request_duration, query_p2p_request_duration,
};
use anyhow::Context;
use polars::prelude::{IntoLazy, col, lit};
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ZeroArcCreateSummary {
    // Scenario metrics
    create_rate: PartitionedRateStats,
    sync_lag_timing: PartitionedTimingStats,
    sync_lag_rate: PartitionedRateStats,
    open_connections: PartitionedGaugeStats,
    error_count: usize,
    p2p_request_duration: Option<StandardTimingsStats>,
    p2p_handle_request_duration: Option<StandardTimingsStats>,
}

pub(crate) async fn summarize_zero_arc_create_data(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<SummaryOutput> {
    assert_eq!(summary.scenario_name, "zero_arc_create_data");

    let create_zome_calls = query::query_zome_call_instrument_data(client.clone(), &summary)
        .await
        .context("Load send data")?
        .clone()
        .lazy()
        .filter(col("fn_name").eq(lit("created_timed_entry")))
        .collect()?;

    let sync_lag = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.zero_arc_create_data_sync_lag",
        &["agent"],
    )
    .await
    .context("Load lag data")?;

    let open_connections = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.zero_arc_create_data_open_connections",
        &["agent", "arc"],
    )
    .await
    .context("Load open connections data")?;

    let host_metrics = HostMetricsAggregator::new(&client, &summary)
        .try_aggregate()
        .await;

    SummaryOutput::new(
        summary.clone(),
        ZeroArcCreateSummary {
            create_rate: partitioned_rate_stats(create_zome_calls, "value", "10s", &["agent"])
                .context("Rate stats for create")?,
            sync_lag_timing: partitioned_timing_stats(sync_lag.clone(), "value", "10s", &["agent"])
                .context("Timing stats for sync lag")?,
            sync_lag_rate: partitioned_rate_stats(sync_lag, "value", "10s", &["agent"])
                .context("Rate stats for sync lag")?,
            open_connections: partitioned_gauge_stats(open_connections, "value", &["arc"])
                .context("Open connections")?,
            error_count: query::zome_call_error_count(client.clone(), &summary).await?,
            p2p_request_duration: query_p2p_request_duration(&client, &summary).await?,
            p2p_handle_request_duration: query_p2p_handle_request_duration(&client, &summary)
                .await?,
        },
        host_metrics,
    )
}
