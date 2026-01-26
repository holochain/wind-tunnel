use crate::frame::LoadError;
use crate::model::StandardTimingsStats;
use crate::query::{query_count, query_duration};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use wind_tunnel_summary_model::RunSummary;

/// Query `hc.holochain_p2p.request.duration.s` metric and compute its stats.
pub async fn query_p2p_request_duration(
    client: &influxdb::Client,
    summary: &RunSummary,
    filter_tag: Option<(&str, &str)>,
) -> anyhow::Result<Option<StandardTimingsStats>> {
    match query_duration(
        client,
        summary,
        "hc.holochain_p2p.request.duration.s",
        filter_tag,
    )
    .await
    {
        Ok(duration) => Ok(Some(duration)),
        Err(e) => match e.downcast_ref::<LoadError>() {
            Some(LoadError::NoSeriesInResult { .. }) => Ok(None),
            None => Err(e).context("query p2p request duration"),
        },
    }
}

/// Query `hc.holochain_p2p.handle_request.duration.s` metric and compute its stats.
pub async fn query_p2p_handle_request_duration(
    client: &influxdb::Client,
    summary: &RunSummary,
    filter_tag: Option<(&str, &str)>,
) -> anyhow::Result<Option<StandardTimingsStats>> {
    match query_duration(
        client,
        summary,
        "hc.holochain_p2p.handle_request.duration.s",
        filter_tag,
    )
    .await
    {
        Ok(duration) => Ok(Some(duration)),
        Err(e) => match e.downcast_ref::<LoadError>() {
            Some(LoadError::NoSeriesInResult { .. }) => Ok(None),
            None => Err(e).context("query p2p handle request duration"),
        },
    }
}

/// Query `hc.holochain_p2p.request.duration.s` metric and return the number of data points.
pub async fn query_p2p_request_count(
    client: &influxdb::Client,
    summary: &RunSummary,
    filter_tag: Option<(&str, &str)>,
) -> anyhow::Result<usize> {
    match query_count(
        client,
        summary,
        "hc.holochain_p2p.request.duration.s",
        filter_tag,
    )
    .await
    {
        Ok(count) => Ok(count),
        Err(e) => match e.downcast_ref::<LoadError>() {
            Some(LoadError::NoSeriesInResult { .. }) => Ok(0),
            None => Err(e).context("query p2p request count"),
        },
    }
}

/// Query `hc.holochain_p2p.handle_request.duration.s` metric and return the number of data points.
pub async fn query_p2p_handle_request_count(
    client: &influxdb::Client,
    summary: &RunSummary,
    filter_tag: Option<(&str, &str)>,
) -> anyhow::Result<usize> {
    match query_count(
        client,
        summary,
        "hc.holochain_p2p.handle_request.duration.s",
        filter_tag,
    )
    .await
    {
        Ok(count) => Ok(count),
        Err(e) => match e.downcast_ref::<LoadError>() {
            Some(LoadError::NoSeriesInResult { .. }) => Ok(0),
            None => Err(e).context("query p2p handle request count"),
        },
    }
}

/// StandardTimingStats from `hc.holochain_p2p.request.duration.s` filtered for each `tag`
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolochainP2pRequestDurationByTag {
    get: Option<StandardTimingsStats>,
    get_links: Option<StandardTimingsStats>,
    count_links: Option<StandardTimingsStats>,
    get_agent_activity: Option<StandardTimingsStats>,
    must_get_agent_activity: Option<StandardTimingsStats>,
    send_validation_receipts: Option<StandardTimingsStats>,
    call_remote: Option<StandardTimingsStats>,
}

/// StandardTimingStats from `hc.holochain_p2p.handle_request.duration.s` filtered for each `message_type`
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolochainP2pHandleRequestDurationByMessageType {
    response: Option<StandardTimingsStats>,
    call_remote: Option<StandardTimingsStats>,
    get: Option<StandardTimingsStats>,
    get_links: Option<StandardTimingsStats>,
    count_links: Option<StandardTimingsStats>,
    get_agent_activity: Option<StandardTimingsStats>,
    must_get_agent_activity: Option<StandardTimingsStats>,
    send_validation_receipts: Option<StandardTimingsStats>,
    remote_signal: Option<StandardTimingsStats>,
    publish_counter_sign: Option<StandardTimingsStats>,
    countersigning_session_negotiation: Option<StandardTimingsStats>,
}

/// All holochain_p2p metrics, for each request type.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolochainP2pMetrics {
    request_roundtrip_duration: HolochainP2pRequestDurationByTag,
    handle_incoming_request_duration: HolochainP2pHandleRequestDurationByMessageType,
}

/// Query `hc.holochain_p2p.request.duration.s` metric for each `tag`,
/// and `hc.holochain_p2p.handle_request.duration.s` metric for each `message_type`,
/// returning the results in a single struct.
pub async fn query_holochain_p2p_metrics(
    client: &influxdb::Client,
    summary: &RunSummary,
) -> anyhow::Result<HolochainP2pMetrics> {
    Ok(HolochainP2pMetrics {
        request_roundtrip_duration: HolochainP2pRequestDurationByTag {
            get: query_p2p_request_duration(client, summary, Some(("tag", "get"))).await?,
            get_links: query_p2p_request_duration(client, summary, Some(("tag", "get_links")))
                .await?,
            count_links: query_p2p_request_duration(client, summary, Some(("tag", "count_links")))
                .await?,
            get_agent_activity: query_p2p_request_duration(
                client,
                summary,
                Some(("tag", "get_agent_activity")),
            )
            .await?,
            must_get_agent_activity: query_p2p_request_duration(
                client,
                summary,
                Some(("tag", "must_get_agent_activity")),
            )
            .await?,
            send_validation_receipts: query_p2p_request_duration(
                client,
                summary,
                Some(("tag", "send_validation_receipts")),
            )
            .await?,
            call_remote: query_p2p_request_duration(client, summary, Some(("tag", "call_remote")))
                .await?,
        },
        handle_incoming_request_duration: HolochainP2pHandleRequestDurationByMessageType {
            response: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "response")),
            )
            .await?,
            call_remote: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "call_remote")),
            )
            .await?,
            get: query_p2p_handle_request_duration(client, summary, Some(("message_type", "get")))
                .await?,
            get_links: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "get_links")),
            )
            .await?,
            count_links: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "count_links")),
            )
            .await?,
            get_agent_activity: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "get_agent_activity")),
            )
            .await?,
            must_get_agent_activity: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "must_get_agent_activity")),
            )
            .await?,
            send_validation_receipts: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "send_validation_receipts")),
            )
            .await?,
            remote_signal: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "remote_signal")),
            )
            .await?,
            publish_counter_sign: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "publish_counter_sign")),
            )
            .await?,
            countersigning_session_negotiation: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "countersigning_session_negotiation")),
            )
            .await?,
        },
    })
}

/// Count of `hc.holochain_p2p.request.duration.s` data points filtered for each `tag`.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolochainP2pRequestCountByTag {
    get: usize,
    get_links: usize,
    count_links: usize,
    get_agent_activity: usize,
    must_get_agent_activity: usize,
    send_validation_receipts: usize,
    call_remote: usize,
}

/// Count of `hc.holochain_p2p.handle_request.duration.s` data points filtered for each `message_type`.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolochainP2pHandleRequestCountByMessageType {
    response: usize,
    call_remote: usize,
    get: usize,
    get_links: usize,
    count_links: usize,
    get_agent_activity: usize,
    must_get_agent_activity: usize,
    send_validation_receipts: usize,
    remote_signal: usize,
    publish_counter_sign: usize,
    countersigning_session_negotiation: usize,
}

/// All holochain_p2p metrics including duration stats and request counts, for each request type.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolochainP2pMetricsWithCounts {
    request_roundtrip_duration: HolochainP2pRequestDurationByTag,
    handle_incoming_request_duration: HolochainP2pHandleRequestDurationByMessageType,
    request_count: HolochainP2pRequestCountByTag,
    handle_incoming_request_count: HolochainP2pHandleRequestCountByMessageType,
}

/// Query `hc.holochain_p2p.request.duration.s` and `hc.holochain_p2p.handle_request.duration.s`
/// metrics for each tag/message_type, returning both duration stats and request counts.
pub async fn query_holochain_p2p_metrics_with_counts(
    client: &influxdb::Client,
    summary: &RunSummary,
) -> anyhow::Result<HolochainP2pMetricsWithCounts> {
    let base = query_holochain_p2p_metrics(client, summary).await?;

    Ok(HolochainP2pMetricsWithCounts {
        request_roundtrip_duration: base.request_roundtrip_duration,
        handle_incoming_request_duration: base.handle_incoming_request_duration,
        request_count: HolochainP2pRequestCountByTag {
            get: query_p2p_request_count(client, summary, Some(("tag", "get"))).await?,
            get_links: query_p2p_request_count(client, summary, Some(("tag", "get_links"))).await?,
            count_links: query_p2p_request_count(client, summary, Some(("tag", "count_links")))
                .await?,
            get_agent_activity: query_p2p_request_count(
                client,
                summary,
                Some(("tag", "get_agent_activity")),
            )
            .await?,
            must_get_agent_activity: query_p2p_request_count(
                client,
                summary,
                Some(("tag", "must_get_agent_activity")),
            )
            .await?,
            send_validation_receipts: query_p2p_request_count(
                client,
                summary,
                Some(("tag", "send_validation_receipts")),
            )
            .await?,
            call_remote: query_p2p_request_count(client, summary, Some(("tag", "call_remote")))
                .await?,
        },
        handle_incoming_request_count: HolochainP2pHandleRequestCountByMessageType {
            response: query_p2p_handle_request_count(
                client,
                summary,
                Some(("message_type", "response")),
            )
            .await?,
            call_remote: query_p2p_handle_request_count(
                client,
                summary,
                Some(("message_type", "call_remote")),
            )
            .await?,
            get: query_p2p_handle_request_count(client, summary, Some(("message_type", "get")))
                .await?,
            get_links: query_p2p_handle_request_count(
                client,
                summary,
                Some(("message_type", "get_links")),
            )
            .await?,
            count_links: query_p2p_handle_request_count(
                client,
                summary,
                Some(("message_type", "count_links")),
            )
            .await?,
            get_agent_activity: query_p2p_handle_request_count(
                client,
                summary,
                Some(("message_type", "get_agent_activity")),
            )
            .await?,
            must_get_agent_activity: query_p2p_handle_request_count(
                client,
                summary,
                Some(("message_type", "must_get_agent_activity")),
            )
            .await?,
            send_validation_receipts: query_p2p_handle_request_count(
                client,
                summary,
                Some(("message_type", "send_validation_receipts")),
            )
            .await?,
            remote_signal: query_p2p_handle_request_count(
                client,
                summary,
                Some(("message_type", "remote_signal")),
            )
            .await?,
            publish_counter_sign: query_p2p_handle_request_count(
                client,
                summary,
                Some(("message_type", "publish_counter_sign")),
            )
            .await?,
            countersigning_session_negotiation: query_p2p_handle_request_count(
                client,
                summary,
                Some(("message_type", "countersigning_session_negotiation")),
            )
            .await?,
        },
    })
}
