use crate::analyze::{standard_rate, standard_timing_stats};
use crate::model::{StandardRateStats, StandardTimingsStats};
use crate::query;
use crate::query::zome_call_error_count;
use anyhow::Context;
use polars::prelude::{IntoLazy, col, lit};
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ZomeCallSingleValueSummary {
    call_timing: StandardTimingsStats,
    call_rate: StandardRateStats,
    errors: usize,
}

pub(crate) async fn summarize_zome_call_single_value(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<ZomeCallSingleValueSummary> {
    assert_eq!(summary.scenario_name, "zome_call_single_value");

    let zome_calls = query::query_zome_call_instrument_data(client.clone(), &summary)
        .await
        .context("Load zome call data")?;

    let zome_calls = zome_calls
        .clone()
        .lazy()
        .filter(col("fn_name").eq(lit("get_value")))
        .collect()?;

    Ok(ZomeCallSingleValueSummary {
        call_timing: standard_timing_stats(zome_calls.clone(), "value", "10s", None)
            .context("Call timing stats")?,
        call_rate: standard_rate(zome_calls, "value", "10s").context("Call rate")?,
        errors: zome_call_error_count(client.clone(), &summary)
            .await
            .context("Load zome call error data")?,
    })
}
