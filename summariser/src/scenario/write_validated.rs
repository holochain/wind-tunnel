use crate::aggregator::HostMetricsAggregator;
use crate::analyze::{standard_rate, standard_timing_stats};
use crate::model::{StandardRateStats, StandardTimingsStats, SummaryOutput};
use crate::query;
use crate::query::zome_call_error_count;
use anyhow::Context;
use polars::prelude::{col, lit, IntoLazy};
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WriteValidatedSummary {
    write_timing: StandardTimingsStats,
    write_rate: StandardRateStats,
    read_timing: StandardTimingsStats,
    read_rate: StandardRateStats,
    errors: usize,
}

pub(crate) async fn summarize_write_validated(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<SummaryOutput> {
    assert_eq!(summary.scenario_name, "write_validated");

    let zome_calls = query::query_zome_call_instrument_data(client.clone(), &summary)
        .await
        .context("Load zome call data")?;

    let create_zome_calls = zome_calls
        .clone()
        .lazy()
        .filter(col("fn_name").eq(lit("create_sample_entry")))
        .collect()?;

    let update_zome_calls = zome_calls
        .clone()
        .lazy()
        .filter(col("fn_name").eq(lit("update_sample_entry")))
        .collect()?;

    let host_metrics = HostMetricsAggregator::new(&client, &summary)
        .try_aggregate()
        .await;

    SummaryOutput::new(
        summary.clone(),
        WriteValidatedSummary {
            write_timing: standard_timing_stats(create_zome_calls.clone(), "value", "10s", None)
                .context("Create timing stats")?,
            write_rate: standard_rate(create_zome_calls, "value", "10s").context("Create rate")?,
            read_timing: standard_timing_stats(update_zome_calls.clone(), "value", "10s", None)
                .context("Update timing stats")?,
            read_rate: standard_rate(update_zome_calls, "value", "10s").context("Update rate")?,
            errors: zome_call_error_count(client.clone(), &summary)
                .await
                .context("Load zome call error data")?,
        },
        host_metrics,
    )
}
