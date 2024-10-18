use crate::analyze::{partitioned_rate_stats, partitioned_timing_stats};
use crate::model::{PartitionedRateStats, PartitionedTimingStats, SummaryOutput};
use crate::query;
use crate::query::zome_call_error_count;
use anyhow::Context;
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SingleWriteManyReadSummary {
    create_timing: PartitionedTimingStats,
    create_rate: PartitionedRateStats,
    update_timing: PartitionedTimingStats,
    update_rate_10s: PartitionedRateStats,
    error_count: usize,
}

pub(crate) async fn summarize_trycp_write_validated(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<SummaryOutput> {
    assert_eq!(summary.scenario_name, "trycp_write_validated");

    let zome_calls = query::query_zome_call_instrument_data(client.clone(), &summary)
        .await
        .context("Load zome call data")?;

    let create_calls = zome_calls
        .clone()
        .lazy()
        .filter(col("fn_name").eq(lit("create_sample_entry")))
        .collect()?;

    let update_calls = zome_calls
        .clone()
        .lazy()
        .filter(col("fn_name").eq(lit("update_sample_entry")))
        .collect()?;

    SummaryOutput::new(
        summary.clone(),
        SingleWriteManyReadSummary {
            create_timing: partitioned_timing_stats(create_calls.clone(), "value", "10s", &["agent"])?,
            create_rate: partitioned_rate_stats(create_calls.clone(), "value", "10s", &["agent"])?,
            update_timing: partitioned_timing_stats(update_calls.clone(), "value", "10s", &["agent"])?,
            update_rate_10s: partitioned_rate_stats(update_calls.clone(), "value", "10s", &["agent"])?,
            error_count: zome_call_error_count(client.clone(), &summary).await?,
        },
    )
}
