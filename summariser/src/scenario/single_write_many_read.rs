use crate::analyze::{standard_rate, standard_timing_stats};
use crate::frame::LoadError;
use crate::model::{StandardRateStats, StandardTimingsStats};
use crate::query;
use anyhow::Context;
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SingleWriteManyReadSummary {
    read_call: StandardTimingsStats,
    rate_10s: StandardRateStats,
    error_count: usize,
}

pub(crate) async fn summarize_single_write_many_read(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<SingleWriteManyReadSummary> {
    assert_eq!(summary.scenario_name, "single_write_many_read");

    let zome_calls = query::query_zome_call_instrument_data(client.clone(), &summary)
        .await
        .context("Load zome call data")?;

    let zome_calls = zome_calls
        .clone()
        .lazy()
        .filter(col("fn_name").eq(lit("get_sample_entry")))
        .collect()?;

    let error_count =
        match query::query_zome_call_instrument_data_errors(client.clone(), &summary).await {
            Ok(frame) => frame.height(),
            Err(e) => match e.downcast_ref::<LoadError>() {
                Some(LoadError::NoSeriesInResult { .. }) => 0,
                None => {
                    return Err(e).context("Load zome call error data");
                }
            },
        };

    Ok(SingleWriteManyReadSummary {
        read_call: standard_timing_stats(zome_calls.clone(), "value", "10s", None)?,
        rate_10s: standard_rate(zome_calls, "value", "10s")?,
        error_count,
    })
}
