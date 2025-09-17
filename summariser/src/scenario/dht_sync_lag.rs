use crate::aggregator::HostMetricsAggregator;
use crate::analyze::partitioned_rate_stats;
use crate::model::{
    CounterStats, GaugeStats, HolochainDatabaseKind, HolochainWorkflowKind, PartitionedRateStats,
    PartitionedTimingStats, StandardTimingsStats, SummaryOutput,
};
use crate::query::holochain_metrics::{
    query_cascade_duration, query_database_connection_use_time, query_database_utilization,
    query_database_utilization_by_id, query_post_commit_duration, query_wasm_usage,
    query_wasm_usage_by_fn, query_workflow_duration, query_workflow_duration_by_agent,
};
use crate::{analyze, query};
use analyze::partitioned_timing_stats;
use anyhow::Context;
use polars::prelude::{col, lit, IntoLazy};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DhtSyncLagSummary {
    // Scenario metrics
    create_rate: PartitionedRateStats,
    sync_lag_timing: PartitionedTimingStats,
    sync_lag_rate: PartitionedRateStats,
    error_count: usize,
    // Holochain metrics
    cascade_duration: Option<StandardTimingsStats>,
    wasm_usage_total: Option<CounterStats>,
    wasm_usage_by_fn: Option<BTreeMap<String, CounterStats>>,
    post_commit_duration: Option<StandardTimingsStats>,
    publish_dht_ops_workflow_duration: Option<StandardTimingsStats>,
    integrate_dht_ops_workflow_duration: Option<StandardTimingsStats>,
    countersigning_workflow_duration: Option<BTreeMap<String, StandardTimingsStats>>,
    app_validation_workflow_duration: Option<StandardTimingsStats>,
    system_validation_workflow_duration: Option<StandardTimingsStats>,
    validation_receipt_workflow_duration: Option<StandardTimingsStats>,
    authored_db_utilization: Option<BTreeMap<String, GaugeStats>>,
    authored_db_connection_use_time: Option<StandardTimingsStats>,
    conductor_db_utilization: Option<GaugeStats>,
    conductor_db_connection_use_time: Option<StandardTimingsStats>,
    dht_db_utilization: Option<GaugeStats>,
    dht_db_connection_use_time: Option<StandardTimingsStats>,
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

    let sync_lag = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.dht_sync_lag",
        &["agent"],
    )
    .await
    .context("Load lag data")?;

    let host_metrics = HostMetricsAggregator::new(&client, &summary)
        .try_aggregate()
        .await;

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
            countersigning_workflow_duration: query_workflow_duration_by_agent(
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

            authored_db_utilization: query_database_utilization_by_id(
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
        },
        host_metrics,
    )
}
