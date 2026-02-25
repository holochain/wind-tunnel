use crate::analyze::{
    delivery_ratio, partitioned_counter_stats_allow_empty, partitioned_rate_stats,
};
use crate::model::{
    CounterStats, GaugeStats, HolochainDatabaseKind, HolochainWorkflowKind,
    PartitionedCounterStats, PartitionedRateStats, PartitionedTimingStats, StandardTimingsStats,
};
use crate::query::holochain_metrics::{
    query_cascade_duration, query_database_connection_use_time, query_database_utilization,
    query_post_commit_duration, query_wasm_usage, query_wasm_usage_by_fn, query_workflow_duration,
};
use crate::query::holochain_p2p_metrics::{HolochainP2pMetrics, query_holochain_p2p_metrics};
use crate::{analyze, query};
use analyze::partitioned_timing_stats;
use anyhow::Context;
use polars::prelude::{IntoLazy, col, lit};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DhtSyncLagSummary {
    // Scenario metrics
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
    // Holochain metrics
    /// Duration of cascade (get) operations inside Holochain (seconds); None if no data
    cascade_duration: Option<StandardTimingsStats>,
    /// Total Wasm execution count across the run; None if no data
    wasm_usage_total: Option<CounterStats>,
    /// Wasm execution counts broken down by zome function name; None if no data
    wasm_usage_by_fn: Option<BTreeMap<String, CounterStats>>,
    /// Duration of post-commit workflow executions (seconds); None if no data
    post_commit_duration: Option<StandardTimingsStats>,
    /// Duration of PublishDhtOps workflow executions (seconds); None if no data
    publish_dht_ops_workflow_duration: Option<StandardTimingsStats>,
    /// Duration of IntegrateDhtOps workflow executions (seconds); None if no data
    integrate_dht_ops_workflow_duration: Option<StandardTimingsStats>,
    /// Duration of Countersigning workflow executions (seconds); None if no data
    countersigning_workflow_duration: Option<StandardTimingsStats>,
    /// Duration of AppValidation workflow executions (seconds); None if no data
    app_validation_workflow_duration: Option<StandardTimingsStats>,
    /// Duration of SysValidation workflow executions (seconds); None if no data
    system_validation_workflow_duration: Option<StandardTimingsStats>,
    /// Duration of ValidationReceipt workflow executions (seconds); None if no data
    validation_receipt_workflow_duration: Option<StandardTimingsStats>,
    /// Authored database utilization (fraction 0–1); None if no data
    authored_db_utilization: Option<GaugeStats>,
    /// Time spent holding authored database connections (seconds); None if no data
    authored_db_connection_use_time: Option<StandardTimingsStats>,
    /// Conductor database utilization (fraction 0–1); None if no data
    conductor_db_utilization: Option<GaugeStats>,
    /// Time spent holding conductor database connections (seconds); None if no data
    conductor_db_connection_use_time: Option<StandardTimingsStats>,
    /// DHT database utilization (fraction 0–1); None if no data
    dht_db_utilization: Option<GaugeStats>,
    /// Time spent holding DHT database connections (seconds); None if no data
    dht_db_connection_use_time: Option<StandardTimingsStats>,
    /// Holochain p2p network metrics for the run
    holochain_p2p_metrics: HolochainP2pMetrics,
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

        cascade_duration: query_cascade_duration(&client, &summary).await?,
        wasm_usage_total: query_wasm_usage(&client, &summary).await?,
        wasm_usage_by_fn: query_wasm_usage_by_fn(&client, &summary).await?,
        post_commit_duration: query_post_commit_duration(&client, &summary).await?,

        publish_dht_ops_workflow_duration: query_workflow_duration(
            &client,
            &summary,
            HolochainWorkflowKind::PublishDhtOps,
        )
        .await?,
        integrate_dht_ops_workflow_duration: query_workflow_duration(
            &client,
            &summary,
            HolochainWorkflowKind::IntegrateDhtOps,
        )
        .await?,
        countersigning_workflow_duration: query_workflow_duration(
            &client,
            &summary,
            HolochainWorkflowKind::Countersigning,
        )
        .await?,
        app_validation_workflow_duration: query_workflow_duration(
            &client,
            &summary,
            HolochainWorkflowKind::AppValidation,
        )
        .await?,
        system_validation_workflow_duration: query_workflow_duration(
            &client,
            &summary,
            HolochainWorkflowKind::SysValidation,
        )
        .await?,
        validation_receipt_workflow_duration: query_workflow_duration(
            &client,
            &summary,
            HolochainWorkflowKind::ValidationReceipt,
        )
        .await?,

        authored_db_utilization: query_database_utilization(
            &client,
            &summary,
            HolochainDatabaseKind::Authored,
        )
        .await?,
        authored_db_connection_use_time: query_database_connection_use_time(
            &client,
            &summary,
            HolochainDatabaseKind::Authored,
        )
        .await?,
        conductor_db_utilization: query_database_utilization(
            &client,
            &summary,
            HolochainDatabaseKind::Conductor,
        )
        .await?,
        conductor_db_connection_use_time: query_database_connection_use_time(
            &client,
            &summary,
            HolochainDatabaseKind::Conductor,
        )
        .await?,
        dht_db_utilization: query_database_utilization(
            &client,
            &summary,
            HolochainDatabaseKind::Dht,
        )
        .await?,
        dht_db_connection_use_time: query_database_connection_use_time(
            &client,
            &summary,
            HolochainDatabaseKind::Dht,
        )
        .await?,
        holochain_p2p_metrics: query_holochain_p2p_metrics(&client, &summary).await?,
    })
}
