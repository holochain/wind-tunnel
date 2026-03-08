use crate::analyze::{standard_rate, standard_timing_stats};
use crate::model::{StandardRateStats, StandardTimingsStats};
use crate::query;
use crate::query::zome_call_error_count;
use anyhow::Context;
use polars::prelude::{IntoLazy, col, lit};
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WriteValidatedSummary {
    /// Duration of `create_sample_entry` zome calls (seconds)
    write_timing: StandardTimingsStats,
    /// Rate of `create_sample_entry` zome calls per 10-second window
    write_rate: StandardRateStats,
    /// Duration of `update_sample_entry` zome calls (seconds).
    ///
    /// Note: this field is named `read_timing` for historical reasons but measures entry updates,
    /// not reads.
    read_timing: StandardTimingsStats,
    /// Rate of `update_sample_entry` zome calls per 10-second window.
    ///
    /// Note: this field is named `read_rate` for historical reasons but measures entry updates,
    /// not reads.
    read_rate: StandardRateStats,
    /// Number of zome call errors observed during the run
    errors: usize,
}

pub(crate) async fn summarize_write_validated(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<WriteValidatedSummary> {
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
        .lazy()
        .filter(col("fn_name").eq(lit("update_sample_entry")))
        .collect()?;

    Ok(WriteValidatedSummary {
        write_timing: standard_timing_stats(create_zome_calls.clone(), "value", "10s", None)
            .context("Create timing stats")?,
        write_rate: standard_rate(create_zome_calls, "value", "10s").context("Create rate")?,
        read_timing: standard_timing_stats(update_zome_calls.clone(), "value", "10s", None)
            .context("Update timing stats")?,
        read_rate: standard_rate(update_zome_calls, "value", "10s").context("Update rate")?,
        errors: zome_call_error_count(client.clone(), &summary)
            .await
            .context("Load zome call error data")?,
    })
}
