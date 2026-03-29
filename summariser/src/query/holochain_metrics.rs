use std::collections::BTreeMap;

use crate::analyze::{counter_stats, histogram_timing_stats};
use crate::frame::LoadError;
use crate::model::{
    CounterStats, DbConnectionUseTimes, GaugeStats, HolochainMetrics, HolochainWorkflowKind,
    LairRequestDurations, P2pHandleRequestCounts, P2pHandleRequestDurations, P2pMetrics,
    P2pRequestCounts, P2pRequestDurations, StandardTimingsStats, WorkflowDurations,
};
use crate::query::{
    query_histogram_duration, query_histogram_total_count, query_metrics_fields,
    query_otel_counter, query_otel_gauge,
};
use anyhow::Context;
use polars::prelude::*;
use wind_tunnel_summary_model::RunSummary;

/// Query all Holochain metrics for a run and return them as a single struct.
///
/// All sub-queries run concurrently. Metrics that have no data in InfluxDB for the
/// given run are returned as `None`.
///
/// The inner future is boxed because the combined `futures::join!` across ~40+
/// concurrent queries produces a very large future that would overflow the stack.
pub async fn query_holochain_metrics(
    client: &influxdb::Client,
    summary: &RunSummary,
) -> anyhow::Result<HolochainMetrics> {
    Box::pin(query_holochain_metrics_inner(client, summary)).await
}

async fn query_holochain_metrics_inner(
    client: &influxdb::Client,
    summary: &RunSummary,
) -> anyhow::Result<HolochainMetrics> {
    let (
        cascade_duration,
        cascade_fetch_error_count,
        wasm_usage,
        zome_call_duration,
        wasm_call_duration,
        host_fn_call_duration,
        emit_signal_count,
        send_remote_signal_count,
        post_commit_duration,
        uptime,
        dropped_signal_count,
        integrated_ops_count,
        integration_delay,
        validation_attempts,
        workflow_duration,
        db_connection_use_duration,
        db_write_txn_duration,
        lair_request_duration,
        p2p,
    ) = futures::join!(
        query_optional_histogram_duration(client, summary, "hc.cascade.duration", None),
        query_optional_otel_counter(client, summary, "hc.cascade.fetch_error", None, "10s"),
        query_partitioned_otel_counter(
            client,
            summary,
            "hc.ribosome.wasm.usage",
            &["zome", "fn"],
            "10s"
        ),
        query_partitioned_histogram_duration(
            client,
            summary,
            "hc.ribosome.zome_call.duration",
            &["zome", "fn"]
        ),
        query_partitioned_histogram_duration(
            client,
            summary,
            "hc.ribosome.wasm_call.duration",
            &["zome", "fn"]
        ),
        query_partitioned_histogram_duration(
            client,
            summary,
            "hc.ribosome.host_fn_call.duration",
            &["host_fn"]
        ),
        query_optional_otel_counter(
            client,
            summary,
            "hc.ribosome.host_fn.emit_signal.count",
            None,
            "10s"
        ),
        query_optional_otel_counter(
            client,
            summary,
            "hc.ribosome.host_fn.send_remote_signal",
            None,
            "10s"
        ),
        query_optional_histogram_duration(
            client,
            summary,
            "hc.conductor.post_commit.duration",
            None
        ),
        query_optional_otel_gauge(client, summary, "hc.conductor.uptime", None, "10s"),
        query_optional_otel_counter(
            client,
            summary,
            "hc.conductor.app_ws.dropped_signal",
            None,
            "10s"
        ),
        query_optional_otel_counter(
            client,
            summary,
            "hc.conductor.workflow.integrated_ops",
            None,
            "10s"
        ),
        query_optional_histogram_duration(
            client,
            summary,
            "hc.conductor.workflow.integration_delay",
            None
        ),
        query_optional_histogram_duration(
            client,
            summary,
            "hc.conductor.workflow.validation_attempts",
            None
        ),
        query_workflow_durations(client, summary),
        query_db_connection_use_times(client, summary),
        query_optional_histogram_duration(client, summary, "hc.db.write_txn.duration", None),
        query_lair_request_durations(client, summary),
        query_p2p_metrics(client, summary),
    );

    Ok(HolochainMetrics {
        cascade_duration,
        cascade_fetch_error_count,
        wasm_usage,
        zome_call_duration,
        wasm_call_duration,
        host_fn_call_duration,
        emit_signal_count,
        send_remote_signal_count,
        post_commit_duration,
        uptime,
        dropped_signal_count,
        integrated_ops_count,
        integration_delay,
        validation_attempts,
        workflow_duration,
        db_connection_use_duration,
        db_write_txn_duration,
        lair_request_duration,
        p2p,
    })
}

// ---------------------------------------------------------------------------
// Private helpers — each returns None on any error (missing data or otherwise).
// Non-missing-data errors are logged as warnings so individual metric failures
// don't discard the entire HolochainMetrics struct.
// ---------------------------------------------------------------------------

async fn query_optional_histogram_duration(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: &str,
    filter_tag: Option<(&str, &str)>,
) -> Option<StandardTimingsStats> {
    match query_histogram_duration(client, summary, measurement, filter_tag).await {
        Ok(v) => Some(v),
        Err(e) => {
            if !matches!(
                e.downcast_ref::<LoadError>(),
                Some(LoadError::NoSeriesInResult { .. })
            ) {
                log::warn!("Failed to query Holochain metric {measurement}: {e:#}");
            }
            None
        }
    }
}

async fn query_optional_otel_counter(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: &str,
    filter_tag: Option<(&str, &str)>,
    window_duration: &str,
) -> Option<CounterStats> {
    match query_otel_counter(client, summary, measurement, filter_tag, window_duration).await {
        Ok(v) => Some(v),
        Err(e) => {
            if !matches!(
                e.downcast_ref::<LoadError>(),
                Some(LoadError::NoSeriesInResult { .. })
            ) {
                log::warn!("Failed to query Holochain metric {measurement}: {e:#}");
            }
            None
        }
    }
}

async fn query_optional_otel_gauge(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: &str,
    filter_tag: Option<(&str, &str)>,
    window_duration: &str,
) -> Option<GaugeStats> {
    match query_otel_gauge(client, summary, measurement, filter_tag, window_duration).await {
        Ok(v) => Some(v),
        Err(e) => {
            if !matches!(
                e.downcast_ref::<LoadError>(),
                Some(LoadError::NoSeriesInResult { .. })
            ) {
                log::warn!("Failed to query Holochain metric {measurement}: {e:#}");
            }
            None
        }
    }
}

async fn query_optional_histogram_total_count(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: &str,
    filter_tag: Option<(&str, &str)>,
) -> usize {
    match query_histogram_total_count(client, summary, measurement, filter_tag).await {
        Ok(v) => v,
        Err(e) => {
            if !matches!(
                e.downcast_ref::<LoadError>(),
                Some(LoadError::NoSeriesInResult { .. })
            ) {
                log::warn!("Failed to query Holochain metric {measurement}: {e:#}");
            }
            0
        }
    }
}

// ---------------------------------------------------------------------------
// Partitioned query helpers — query with tag columns, then split by tag values
// ---------------------------------------------------------------------------

/// Build a composite key from the tag columns in a row, joined by `::`.
fn composite_key(row: &[AnyValue]) -> String {
    row.iter()
        .map(|v| match v {
            AnyValue::String(s) => s.to_string(),
            AnyValue::StringOwned(s) => s.to_string(),
            other => other.to_string(),
        })
        .collect::<Vec<_>>()
        .join("::")
}

/// Query a pre-aggregated OTel histogram with tag columns, then partition by unique
/// tag combinations and compute `StandardTimingsStats` for each partition.
///
/// Returns `None` if no data exists. Keys are `"tag1::tag2"` for multi-tag
/// partitions or just `"tag_value"` for single-tag partitions.
async fn query_partitioned_histogram_duration(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: &str,
    tag_columns: &[&str],
) -> Option<BTreeMap<String, StandardTimingsStats>> {
    let frame = match query_metrics_fields(
        client,
        summary,
        measurement,
        &["count", "sum", "min", "max"],
        tag_columns,
        None,
    )
    .await
    {
        Ok(f) => f,
        Err(e) => {
            if !matches!(
                e.downcast_ref::<LoadError>(),
                Some(LoadError::NoSeriesInResult { .. })
            ) {
                log::warn!("Failed to query Holochain metric {measurement}: {e:#}");
            }
            return None;
        }
    };

    match partition_histogram_stats(frame, tag_columns) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("Failed to partition Holochain metric {measurement}: {e:#}");
            None
        }
    }
}

/// Query an OTel counter with tag columns, then partition by unique tag combinations
/// and compute `CounterStats` for each partition.
///
/// Returns `None` if no data exists.
async fn query_partitioned_otel_counter(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: &str,
    tag_columns: &[&str],
    window_duration: &str,
) -> Option<BTreeMap<String, CounterStats>> {
    let frame =
        match query_metrics_fields(client, summary, measurement, &["sum"], tag_columns, None).await
        {
            Ok(f) => f,
            Err(e) => {
                if !matches!(
                    e.downcast_ref::<LoadError>(),
                    Some(LoadError::NoSeriesInResult { .. })
                ) {
                    log::warn!("Failed to query Holochain metric {measurement}: {e:#}");
                }
                return None;
            }
        };

    match partition_otel_counter_stats(frame, tag_columns, window_duration) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("Failed to partition Holochain metric {measurement}: {e:#}");
            None
        }
    }
}

/// Partition a pre-aggregated histogram DataFrame by tag columns and compute
/// `StandardTimingsStats` per partition.
fn partition_histogram_stats(
    frame: DataFrame,
    tag_columns: &[&str],
) -> anyhow::Result<Option<BTreeMap<String, StandardTimingsStats>>> {
    let unique_keys = unique_tag_combinations(&frame, tag_columns)?;
    if unique_keys.is_empty() {
        return Ok(None);
    }

    let mut map = BTreeMap::new();
    for (key, filter_values) in &unique_keys {
        let filtered = filter_by_tags(&frame, tag_columns, filter_values)?;
        let stats = histogram_timing_stats(filtered, "10s")
            .with_context(|| format!("Histogram stats for partition {key:?}"))?;
        map.insert(key.clone(), stats);
    }
    Ok(Some(map))
}

/// Partition an OTel counter DataFrame by tag columns and compute `CounterStats`
/// per partition. The counter field is `"sum"` (not `"value"`).
fn partition_otel_counter_stats(
    frame: DataFrame,
    tag_columns: &[&str],
    window_duration: &str,
) -> anyhow::Result<Option<BTreeMap<String, CounterStats>>> {
    let unique_keys = unique_tag_combinations(&frame, tag_columns)?;
    if unique_keys.is_empty() {
        return Ok(None);
    }

    let mut map = BTreeMap::new();
    for (key, filter_values) in &unique_keys {
        let filtered = filter_by_tags(&frame, tag_columns, filter_values)?;
        let stats = counter_stats(filtered, "sum", window_duration)
            .with_context(|| format!("Counter stats for partition {key:?}"))?;
        map.insert(key.clone(), stats);
    }
    Ok(Some(map))
}

/// Extract unique combinations of tag columns from a DataFrame.
/// Returns a Vec of (composite_key, Vec<tag_values>) pairs.
fn unique_tag_combinations(
    frame: &DataFrame,
    tag_columns: &[&str],
) -> anyhow::Result<Vec<(String, Vec<String>)>> {
    let tag_cols: Vec<&str> = tag_columns.to_vec();
    let unique = frame
        .clone()
        .lazy()
        .select(tag_cols.iter().map(|&c| col(c)).collect::<Vec<_>>())
        .unique(None, UniqueKeepStrategy::First)
        .collect()
        .context("Unique tag combinations")?;

    let mut result = Vec::new();
    for row_idx in 0..unique.height() {
        let row: Vec<AnyValue> = tag_cols
            .iter()
            .map(|&c| unique.column(c).unwrap().get(row_idx).unwrap())
            .collect();
        let key = composite_key(&row);
        let values: Vec<String> = row
            .iter()
            .map(|v| match v {
                AnyValue::String(s) => s.to_string(),
                AnyValue::StringOwned(s) => s.to_string(),
                other => other.to_string(),
            })
            .collect();
        result.push((key, values));
    }
    Ok(result)
}

/// Filter a DataFrame to rows where tag columns match the given values.
fn filter_by_tags(
    frame: &DataFrame,
    tag_columns: &[&str],
    values: &[String],
) -> anyhow::Result<DataFrame> {
    let mut expr = lit(true);
    for (col_name, val) in tag_columns.iter().zip(values.iter()) {
        expr = expr.and(col(*col_name).eq(lit(val.as_str())));
    }
    frame
        .clone()
        .lazy()
        .filter(expr)
        .collect()
        .context("Filter by tags")
}

// ---------------------------------------------------------------------------
// Lair keystore request durations
// ---------------------------------------------------------------------------

async fn query_lair_request_durations(
    client: &influxdb::Client,
    summary: &RunSummary,
) -> Option<LairRequestDurations> {
    let m = "hc.keystore.lair_request.duration";
    let (sign, shared_secret_encrypt, crypto_box_xsalsa) = futures::join!(
        query_optional_histogram_duration(client, summary, m, Some(("operation", "sign"))),
        query_optional_histogram_duration(
            client,
            summary,
            m,
            Some(("operation", "shared_secret_encrypt"))
        ),
        query_optional_histogram_duration(
            client,
            summary,
            m,
            Some(("operation", "crypto_box_xsalsa"))
        ),
    );

    let d = LairRequestDurations {
        sign,
        shared_secret_encrypt,
        crypto_box_xsalsa,
    };

    if d.sign.is_none() && d.shared_secret_encrypt.is_none() && d.crypto_box_xsalsa.is_none() {
        return None;
    }
    Some(d)
}

// ---------------------------------------------------------------------------
// Workflow durations
// ---------------------------------------------------------------------------

async fn query_workflow_durations(
    client: &influxdb::Client,
    summary: &RunSummary,
) -> Option<WorkflowDurations> {
    let app_validation_tag = HolochainWorkflowKind::AppValidation.to_string();
    let countersigning_tag = HolochainWorkflowKind::Countersigning.to_string();
    let integrate_tag = HolochainWorkflowKind::IntegrateDhtOps.to_string();
    let publish_tag = HolochainWorkflowKind::PublishDhtOps.to_string();
    let sys_validation_tag = HolochainWorkflowKind::SysValidation.to_string();
    let validation_receipt_tag = HolochainWorkflowKind::ValidationReceipt.to_string();
    let witnessing_tag = HolochainWorkflowKind::Witnessing.to_string();
    let m = "hc.conductor.workflow.duration";

    let (
        app_validation,
        countersigning,
        integrate_dht_ops,
        publish_dht_ops,
        sys_validation,
        validation_receipt,
        witnessing,
    ) = futures::join!(
        query_optional_histogram_duration(
            client,
            summary,
            m,
            Some(("workflow", &app_validation_tag))
        ),
        query_optional_histogram_duration(
            client,
            summary,
            m,
            Some(("workflow", &countersigning_tag))
        ),
        query_optional_histogram_duration(client, summary, m, Some(("workflow", &integrate_tag))),
        query_optional_histogram_duration(client, summary, m, Some(("workflow", &publish_tag))),
        query_optional_histogram_duration(
            client,
            summary,
            m,
            Some(("workflow", &sys_validation_tag))
        ),
        query_optional_histogram_duration(
            client,
            summary,
            m,
            Some(("workflow", &validation_receipt_tag))
        ),
        query_optional_histogram_duration(client, summary, m, Some(("workflow", &witnessing_tag))),
    );

    let wd = WorkflowDurations {
        app_validation,
        countersigning,
        integrate_dht_ops,
        publish_dht_ops,
        sys_validation,
        validation_receipt,
        witnessing,
    };

    // Return None if every field is None (no workflow data at all).
    if wd.app_validation.is_none()
        && wd.countersigning.is_none()
        && wd.integrate_dht_ops.is_none()
        && wd.publish_dht_ops.is_none()
        && wd.sys_validation.is_none()
        && wd.validation_receipt.is_none()
        && wd.witnessing.is_none()
    {
        return None;
    }
    Some(wd)
}

// ---------------------------------------------------------------------------
// Database connection use times
// ---------------------------------------------------------------------------

async fn query_db_connection_use_times(
    client: &influxdb::Client,
    summary: &RunSummary,
) -> Option<DbConnectionUseTimes> {
    let (authored, dht, conductor, cache, wasm, peer_meta_store) = futures::join!(
        query_optional_histogram_duration(
            client,
            summary,
            "hc.db.connections.use_time",
            Some(("kind", "authored"))
        ),
        query_optional_histogram_duration(
            client,
            summary,
            "hc.db.connections.use_time",
            Some(("kind", "dht"))
        ),
        query_optional_histogram_duration(
            client,
            summary,
            "hc.db.connections.use_time",
            Some(("kind", "conductor"))
        ),
        query_optional_histogram_duration(
            client,
            summary,
            "hc.db.connections.use_time",
            Some(("kind", "cache"))
        ),
        query_optional_histogram_duration(
            client,
            summary,
            "hc.db.connections.use_time",
            Some(("kind", "wasm"))
        ),
        query_optional_histogram_duration(
            client,
            summary,
            "hc.db.connections.use_time",
            Some(("kind", "peer_meta_store"))
        ),
    );

    let db = DbConnectionUseTimes {
        authored,
        dht,
        conductor,
        cache,
        wasm,
        peer_meta_store,
    };

    if db.authored.is_none()
        && db.dht.is_none()
        && db.conductor.is_none()
        && db.cache.is_none()
        && db.wasm.is_none()
        && db.peer_meta_store.is_none()
    {
        return None;
    }
    Some(db)
}

// ---------------------------------------------------------------------------
// P2P metrics
// ---------------------------------------------------------------------------

async fn query_p2p_metrics(client: &influxdb::Client, summary: &RunSummary) -> Option<P2pMetrics> {
    let (
        request_duration,
        request_count,
        handle_request_duration,
        handle_request_count,
        ignored_request_count,
        recv_remote_signal_count,
    ) = futures::join!(
        query_p2p_request_durations(client, summary),
        query_p2p_request_counts(client, summary),
        query_p2p_handle_request_durations(client, summary),
        query_p2p_handle_request_counts(client, summary),
        query_optional_otel_counter(
            client,
            summary,
            "hc.holochain_p2p.handle_request.ignored.requests",
            None,
            "10s"
        ),
        query_optional_otel_counter(
            client,
            summary,
            "hc.holochain_p2p.recv_remote_signal",
            None,
            "10s"
        ),
    );

    let p2p = P2pMetrics {
        request_duration,
        request_count,
        handle_request_duration,
        handle_request_count,
        ignored_request_count,
        recv_remote_signal_count,
    };

    // Return None only if there's no P2P data at all.
    if p2p.request_duration.is_none()
        && p2p.request_count.is_none()
        && p2p.handle_request_duration.is_none()
        && p2p.handle_request_count.is_none()
        && p2p.ignored_request_count.is_none()
        && p2p.recv_remote_signal_count.is_none()
    {
        return None;
    }
    Some(p2p)
}

async fn query_p2p_request_durations(
    client: &influxdb::Client,
    summary: &RunSummary,
) -> Option<P2pRequestDurations> {
    let m = "hc.holochain_p2p.request.duration";
    let (
        get,
        get_links,
        count_links,
        get_agent_activity,
        must_get_agent_activity,
        send_validation_receipts,
        call_remote,
    ) = futures::join!(
        query_optional_histogram_duration(client, summary, m, Some(("tag", "get"))),
        query_optional_histogram_duration(client, summary, m, Some(("tag", "get_links"))),
        query_optional_histogram_duration(client, summary, m, Some(("tag", "count_links"))),
        query_optional_histogram_duration(client, summary, m, Some(("tag", "get_agent_activity"))),
        query_optional_histogram_duration(
            client,
            summary,
            m,
            Some(("tag", "must_get_agent_activity"))
        ),
        query_optional_histogram_duration(
            client,
            summary,
            m,
            Some(("tag", "send_validation_receipts"))
        ),
        query_optional_histogram_duration(client, summary, m, Some(("tag", "call_remote"))),
    );

    let d = P2pRequestDurations {
        get,
        get_links,
        count_links,
        get_agent_activity,
        must_get_agent_activity,
        send_validation_receipts,
        call_remote,
    };

    if d.get.is_none()
        && d.get_links.is_none()
        && d.count_links.is_none()
        && d.get_agent_activity.is_none()
        && d.must_get_agent_activity.is_none()
        && d.send_validation_receipts.is_none()
        && d.call_remote.is_none()
    {
        return None;
    }
    Some(d)
}

async fn query_p2p_request_counts(
    client: &influxdb::Client,
    summary: &RunSummary,
) -> Option<P2pRequestCounts> {
    let m = "hc.holochain_p2p.request.duration";
    let (
        get,
        get_links,
        count_links,
        get_agent_activity,
        must_get_agent_activity,
        send_validation_receipts,
        call_remote,
    ) = futures::join!(
        query_optional_histogram_total_count(client, summary, m, Some(("tag", "get"))),
        query_optional_histogram_total_count(client, summary, m, Some(("tag", "get_links"))),
        query_optional_histogram_total_count(client, summary, m, Some(("tag", "count_links"))),
        query_optional_histogram_total_count(
            client,
            summary,
            m,
            Some(("tag", "get_agent_activity"))
        ),
        query_optional_histogram_total_count(
            client,
            summary,
            m,
            Some(("tag", "must_get_agent_activity"))
        ),
        query_optional_histogram_total_count(
            client,
            summary,
            m,
            Some(("tag", "send_validation_receipts"))
        ),
        query_optional_histogram_total_count(client, summary, m, Some(("tag", "call_remote"))),
    );

    let c = P2pRequestCounts {
        get,
        get_links,
        count_links,
        get_agent_activity,
        must_get_agent_activity,
        send_validation_receipts,
        call_remote,
    };

    // Return None if all counts are zero (no outgoing P2P requests at all).
    if c.get == 0
        && c.get_links == 0
        && c.count_links == 0
        && c.get_agent_activity == 0
        && c.must_get_agent_activity == 0
        && c.send_validation_receipts == 0
        && c.call_remote == 0
    {
        return None;
    }
    Some(c)
}

async fn query_p2p_handle_request_durations(
    client: &influxdb::Client,
    summary: &RunSummary,
) -> Option<P2pHandleRequestDurations> {
    let m = "hc.holochain_p2p.handle_request.duration";
    let (
        response,
        call_remote,
        get,
        get_links,
        count_links,
        get_agent_activity,
        must_get_agent_activity,
        send_validation_receipts,
        remote_signal,
        publish_counter_sign,
        countersigning_session_negotiation,
    ) = futures::join!(
        query_optional_histogram_duration(client, summary, m, Some(("message_type", "response"))),
        query_optional_histogram_duration(
            client,
            summary,
            m,
            Some(("message_type", "call_remote"))
        ),
        query_optional_histogram_duration(client, summary, m, Some(("message_type", "get"))),
        query_optional_histogram_duration(client, summary, m, Some(("message_type", "get_links"))),
        query_optional_histogram_duration(
            client,
            summary,
            m,
            Some(("message_type", "count_links"))
        ),
        query_optional_histogram_duration(
            client,
            summary,
            m,
            Some(("message_type", "get_agent_activity"))
        ),
        query_optional_histogram_duration(
            client,
            summary,
            m,
            Some(("message_type", "must_get_agent_activity"))
        ),
        query_optional_histogram_duration(
            client,
            summary,
            m,
            Some(("message_type", "send_validation_receipts"))
        ),
        query_optional_histogram_duration(
            client,
            summary,
            m,
            Some(("message_type", "remote_signal"))
        ),
        query_optional_histogram_duration(
            client,
            summary,
            m,
            Some(("message_type", "publish_counter_sign"))
        ),
        query_optional_histogram_duration(
            client,
            summary,
            m,
            Some(("message_type", "countersigning_session_negotiation"))
        ),
    );

    let d = P2pHandleRequestDurations {
        response,
        call_remote,
        get,
        get_links,
        count_links,
        get_agent_activity,
        must_get_agent_activity,
        send_validation_receipts,
        remote_signal,
        publish_counter_sign,
        countersigning_session_negotiation,
    };

    if d.response.is_none()
        && d.call_remote.is_none()
        && d.get.is_none()
        && d.get_links.is_none()
        && d.count_links.is_none()
        && d.get_agent_activity.is_none()
        && d.must_get_agent_activity.is_none()
        && d.send_validation_receipts.is_none()
        && d.remote_signal.is_none()
        && d.publish_counter_sign.is_none()
        && d.countersigning_session_negotiation.is_none()
    {
        return None;
    }
    Some(d)
}

async fn query_p2p_handle_request_counts(
    client: &influxdb::Client,
    summary: &RunSummary,
) -> Option<P2pHandleRequestCounts> {
    let m = "hc.holochain_p2p.handle_request.duration";
    let (
        response,
        call_remote,
        get,
        get_links,
        count_links,
        get_agent_activity,
        must_get_agent_activity,
        send_validation_receipts,
        remote_signal,
        publish_counter_sign,
        countersigning_session_negotiation,
    ) = futures::join!(
        query_optional_histogram_total_count(
            client,
            summary,
            m,
            Some(("message_type", "response"))
        ),
        query_optional_histogram_total_count(
            client,
            summary,
            m,
            Some(("message_type", "call_remote"))
        ),
        query_optional_histogram_total_count(client, summary, m, Some(("message_type", "get"))),
        query_optional_histogram_total_count(
            client,
            summary,
            m,
            Some(("message_type", "get_links"))
        ),
        query_optional_histogram_total_count(
            client,
            summary,
            m,
            Some(("message_type", "count_links"))
        ),
        query_optional_histogram_total_count(
            client,
            summary,
            m,
            Some(("message_type", "get_agent_activity"))
        ),
        query_optional_histogram_total_count(
            client,
            summary,
            m,
            Some(("message_type", "must_get_agent_activity"))
        ),
        query_optional_histogram_total_count(
            client,
            summary,
            m,
            Some(("message_type", "send_validation_receipts"))
        ),
        query_optional_histogram_total_count(
            client,
            summary,
            m,
            Some(("message_type", "remote_signal"))
        ),
        query_optional_histogram_total_count(
            client,
            summary,
            m,
            Some(("message_type", "publish_counter_sign"))
        ),
        query_optional_histogram_total_count(
            client,
            summary,
            m,
            Some(("message_type", "countersigning_session_negotiation"))
        ),
    );

    let c = P2pHandleRequestCounts {
        response,
        call_remote,
        get,
        get_links,
        count_links,
        get_agent_activity,
        must_get_agent_activity,
        send_validation_receipts,
        remote_signal,
        publish_counter_sign,
        countersigning_session_negotiation,
    };

    if c.response == 0
        && c.call_remote == 0
        && c.get == 0
        && c.get_links == 0
        && c.count_links == 0
        && c.get_agent_activity == 0
        && c.must_get_agent_activity == 0
        && c.send_validation_receipts == 0
        && c.remote_signal == 0
        && c.publish_counter_sign == 0
        && c.countersigning_session_negotiation == 0
    {
        return None;
    }
    Some(c)
}
