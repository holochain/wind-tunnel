use crate::frame::LoadError;
use crate::model::{CounterStats, GaugeStats, HolochainWorkflowKind};
use crate::model::{HolochainDatabaseKind, StandardTimingsStats};
use crate::query::{
    query_and_partition_counter, query_and_partition_duration, query_and_partition_gauge,
    query_counter, query_duration, query_gauge,
};
use anyhow::Context;
use std::collections::BTreeMap;
use wind_tunnel_summary_model::RunSummary;

/// Query `hc.cascade.duration` metric and compute its stats.
pub async fn query_cascade_duration(
    client: &influxdb::Client,
    summary: &RunSummary,
) -> anyhow::Result<Option<StandardTimingsStats>> {
    match query_duration(client, summary, "hc.cascade.duration.s", None).await {
        Ok(duration) => Ok(Some(duration)),
        Err(e) => match e.downcast_ref::<LoadError>() {
            Some(LoadError::NoSeriesInResult { .. }) => Ok(None),
            None => Err(e).context("query cascade duration"),
        },
    }
}

/// Query `hc.ribosome.wasm.usage` metric and compute its stats.
pub async fn query_wasm_usage(
    client: &influxdb::Client,
    summary: &RunSummary,
) -> anyhow::Result<Option<CounterStats>> {
    match query_counter(client, summary, "hc.ribosome.wasm.usage", None).await {
        Ok(res) => Ok(Some(res)),
        Err(e) => match e.downcast_ref::<LoadError>() {
            Some(LoadError::NoSeriesInResult { .. }) => Ok(None),
            None => Err(e).context("query wasm usage"),
        },
    }
}

/// Query `hc.ribosome.wasm.usage` metric, partition results by fn and compute their stats.
pub async fn query_wasm_usage_by_fn(
    client: &influxdb::Client,
    summary: &RunSummary,
) -> anyhow::Result<Option<BTreeMap<String, CounterStats>>> {
    match query_and_partition_counter(
        client,
        summary,
        "hc.ribosome.wasm.usage",
        &["zome", "fn"],
        None,
    )
    .await
    {
        Ok(res) => Ok(Some(res)),
        Err(e) => match e.downcast_ref::<LoadError>() {
            Some(LoadError::NoSeriesInResult { .. }) => Ok(None),
            None => Err(e).context("query wasm usage by fn"),
        },
    }
}

/// Query `hc.conductor.post_commit.duration.s` metric and compute its stats.
pub async fn query_post_commit_duration(
    client: &influxdb::Client,
    summary: &RunSummary,
) -> anyhow::Result<Option<StandardTimingsStats>> {
    match query_duration(client, summary, "hc.conductor.post_commit.duration.s", None).await {
        Ok(res) => Ok(Some(res)),
        Err(e) => match e.downcast_ref::<LoadError>() {
            Some(LoadError::NoSeriesInResult { .. }) => Ok(None),
            None => Err(e).context("query post_commit duration"),
        },
    }
}

/// Query `hc.conductor.workflow.duration` metric for a specific workflow and compute its stats.
pub async fn query_workflow_duration(
    client: &influxdb::Client,
    summary: &RunSummary,
    workflow: HolochainWorkflowKind,
) -> anyhow::Result<Option<StandardTimingsStats>> {
    match query_duration(
        client,
        summary,
        "hc.conductor.workflow.duration.s",
        Some(("workflow", workflow.to_string().as_str())),
    )
    .await
    {
        Ok(res) => Ok(Some(res)),
        Err(e) => match e.downcast_ref::<LoadError>() {
            Some(LoadError::NoSeriesInResult { .. }) => Ok(None),
            None => Err(e).context("query workflow duration"),
        },
    }
}

/// Query `hc.conductor.workflow.duration` metric for a specific workflow,
/// partition the results by `agent` and compute stats per partition.
pub async fn query_workflow_duration_by_agent(
    client: &influxdb::Client,
    summary: &RunSummary,
    workflow: HolochainWorkflowKind,
) -> anyhow::Result<Option<BTreeMap<String, StandardTimingsStats>>> {
    match query_and_partition_duration(
        client,
        summary,
        "hc.conductor.workflow.duration.s",
        &["agent"],
        Some(("workflow", workflow.to_string().as_str())),
    )
    .await
    {
        Ok(res) => Ok(Some(res)),
        Err(e) => match e.downcast_ref::<LoadError>() {
            Some(LoadError::NoSeriesInResult { .. }) => Ok(None),
            None => Err(e).context("query workflow duration by agent"),
        },
    }
}

/// Query `hc.db.pool.utilization` metric for a specific database kind and compute its stats.
pub async fn query_database_utilization(
    client: &influxdb::Client,
    summary: &RunSummary,
    kind: HolochainDatabaseKind,
) -> anyhow::Result<Option<GaugeStats>> {
    match query_gauge(
        client,
        summary,
        "hc.db.pool.utilization",
        Some(("kind", kind.to_string().as_str())),
    )
    .await
    {
        Ok(res) => Ok(Some(res)),
        Err(e) => match e.downcast_ref::<LoadError>() {
            Some(LoadError::NoSeriesInResult { .. }) => Ok(None),
            None => Err(e).context("query database utilization"),
        },
    }
}

/// Query `hc.db.pool.utilization` metric for a specific database kind,
/// partition the results by `id` and compute stats per partition.
pub async fn query_database_utilization_by_id(
    client: &influxdb::Client,
    summary: &RunSummary,
    kind: HolochainDatabaseKind,
) -> anyhow::Result<Option<BTreeMap<String, GaugeStats>>> {
    match query_and_partition_gauge(
        client,
        summary,
        "hc.db.pool.utilization",
        &["id"],
        Some(("kind", kind.to_string().as_str())),
    )
    .await
    {
        Ok(res) => Ok(Some(res)),
        Err(e) => match e.downcast_ref::<LoadError>() {
            Some(LoadError::NoSeriesInResult { .. }) => Ok(None),
            None => Err(e).context("query database utilization by id"),
        },
    }
}

/// Query `hc.db.connections.use_time` metric for a specific database kind and compute its stats.
pub async fn query_database_connection_use_time(
    client: &influxdb::Client,
    summary: &RunSummary,
    kind: HolochainDatabaseKind,
) -> anyhow::Result<Option<StandardTimingsStats>> {
    match query_duration(
        client,
        summary,
        "hc.db.connections.use_time.s",
        Some(("kind", kind.to_string().as_str())),
    )
    .await
    {
        Ok(res) => Ok(Some(res)),
        Err(e) => match e.downcast_ref::<LoadError>() {
            Some(LoadError::NoSeriesInResult { .. }) => Ok(None),
            None => Err(e).context("query database connection use time"),
        },
    }
}
