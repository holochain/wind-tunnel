use crate::analyze::{standard_rate, standard_timing_stats};
use crate::frame::LoadError;
use crate::model::{StandardTimingsStats, SummaryOutput};
use crate::query;
use anyhow::Context;
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SingleWriteManyReadSummary {
    create_timing: StandardTimingsStats,
    create_mean_rate_10s: f64,
    update_timing: StandardTimingsStats,
    update_mean_rate_10s: f64,
    error_count: usize,
}

pub(crate) async fn summarize_write_validated(
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

    SummaryOutput::new(
        summary,
        SingleWriteManyReadSummary {
            create_timing: standard_timing_stats(create_calls.clone(), "value", None)?,
            create_mean_rate_10s: standard_rate(create_calls.clone(), "value", "10s")?,
            update_timing: standard_timing_stats(update_calls.clone(), "value", None)?,
            update_mean_rate_10s: standard_rate(update_calls.clone(), "value", "10s")?,
            error_count,
        },
    )
}
