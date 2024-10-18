use crate::analyze::{partitioned_rate, standard_timing_stats};
use crate::model::{PartitionedRateStats, StandardTimingsStats, SummaryOutput};
use crate::query;
use crate::query::zome_call_error_count;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ValidationReceiptsSummary {
    receipts_complete_timing: StandardTimingsStats,
    receipts_complete_rate: PartitionedRateStats,
    error_count: usize,
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
        &["agent_name", "op_type"],
    )
    .await
    .context("Load receipts complete data")?;

    SummaryOutput::new(
        summary.clone(),
        ValidationReceiptsSummary {
            receipts_complete_timing: standard_timing_stats(
                receipts_complete.clone(),
                "value",
                "10s",
                None,
            )?,
            receipts_complete_rate: partitioned_rate(
                receipts_complete.clone(),
                "value",
                "10s",
                &["agent", "op_type"],
            )?,
            error_count: zome_call_error_count(client, &summary).await?,
        },
    )
}
