use crate::aggregator::HostMetricsAggregator;
use crate::analyze::{partitioned_rate_stats, partitioned_timing_stats};
use crate::model::{
    PartitionedRateStats, PartitionedTimingStats, StandardTimingsStats, SummaryOutput,
};
use crate::query;
use crate::query::holochain_metrics::{
    query_p2p_handle_request_duration, query_p2p_request_duration,
};
use crate::query::zome_call_error_count;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ValidationReceiptsSummary {
    receipts_complete_timing: PartitionedTimingStats,
    receipts_complete_rate: PartitionedRateStats,
    error_count: usize,
    p2p_request_duration: Option<StandardTimingsStats>,
    p2p_handle_request_duration: Option<StandardTimingsStats>,
}

pub(crate) async fn summarize_validation_receipts(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<SummaryOutput> {
    assert_eq!(summary.scenario_name, "validation_receipts");

    let receipts_complete = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.validation_receipts_complete_time",
        &["agent", "op_type"],
    )
    .await
    .context("Load receipts complete data")?;

    let host_metrics = HostMetricsAggregator::new(&client, &summary)
        .try_aggregate()
        .await;

    SummaryOutput::new(
        summary.clone(),
        ValidationReceiptsSummary {
            receipts_complete_timing: partitioned_timing_stats(
                receipts_complete.clone(),
                "value",
                "10s",
                &["agent", "op_type"],
            )
            .context("Receipts complete timing")?,
            receipts_complete_rate: partitioned_rate_stats(
                receipts_complete.clone(),
                "value",
                "10s",
                &["agent", "op_type"],
            )
            .context("Receipts complete rate")?,
            error_count: zome_call_error_count(client.clone(), &summary).await?,
            p2p_request_duration: query_p2p_request_duration(&client, &summary).await?,
            p2p_handle_request_duration: query_p2p_handle_request_duration(&client, &summary)
                .await?,
        },
        host_metrics,
    )
}
