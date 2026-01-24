use crate::frame::LoadError;
use crate::model::StandardTimingsStats;
use crate::query::query_duration;
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
    error_res: Option<StandardTimingsStats>,
    call_remote_req: Option<StandardTimingsStats>,
    call_remote_res: Option<StandardTimingsStats>,
    get_req: Option<StandardTimingsStats>,
    get_res: Option<StandardTimingsStats>,
    get_links_req: Option<StandardTimingsStats>,
    get_links_res: Option<StandardTimingsStats>,
    count_links_req: Option<StandardTimingsStats>,
    count_links_res: Option<StandardTimingsStats>,
    get_agent_activity_req: Option<StandardTimingsStats>,
    get_agent_activity_res: Option<StandardTimingsStats>,
    must_get_agent_activity_req: Option<StandardTimingsStats>,
    must_get_agent_activity_res: Option<StandardTimingsStats>,
    send_validation_receipts_req: Option<StandardTimingsStats>,
    send_validation_receipts_res: Option<StandardTimingsStats>,
    remote_signal_evt: Option<StandardTimingsStats>,
    publish_countersign_evt: Option<StandardTimingsStats>,
    countersigning_session_negotiation_evt: Option<StandardTimingsStats>,
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
            error_res: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "ErrorRes")),
            )
            .await?,
            call_remote_req: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "CallRemoteReq")),
            )
            .await?,
            call_remote_res: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "CallRemoteRes")),
            )
            .await?,
            get_req: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "GetReq")),
            )
            .await?,
            get_res: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "GetRes")),
            )
            .await?,
            get_links_req: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "GetLinksReq")),
            )
            .await?,
            get_links_res: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "GetLinksRes")),
            )
            .await?,
            count_links_req: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "CountLinksReq")),
            )
            .await?,
            count_links_res: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "CountLinksRes")),
            )
            .await?,
            get_agent_activity_req: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "GetAgentActivityReq")),
            )
            .await?,
            get_agent_activity_res: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "GetAgentActivityRes")),
            )
            .await?,
            must_get_agent_activity_req: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "MustGetAgentActivityReq")),
            )
            .await?,
            must_get_agent_activity_res: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "MustGetAgentActivityRes")),
            )
            .await?,
            send_validation_receipts_req: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "SendValidationReceiptsReq")),
            )
            .await?,
            send_validation_receipts_res: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "SendValidationReceiptsRes")),
            )
            .await?,
            remote_signal_evt: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "RemoteSignalEvt")),
            )
            .await?,
            publish_countersign_evt: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "PublishCountersignEvt")),
            )
            .await?,
            countersigning_session_negotiation_evt: query_p2p_handle_request_duration(
                client,
                summary,
                Some(("message_type", "CountersigningSessionNegotiationEvt")),
            )
            .await?,
        },
    })
}
