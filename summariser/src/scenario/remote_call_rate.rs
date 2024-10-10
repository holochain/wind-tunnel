use crate::analyze::standard_timing_stats;
use crate::frame::LoadError;
use crate::model::{StandardTimingsStats, SummaryOutput};
use crate::query;
use anyhow::Context;
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RemoteCallRateSummary {
    dispatch: StandardTimingsStats,
    round_trip: StandardTimingsStats,
    average_rate_10s: f64,
    error_count: usize,
}

pub(crate) async fn summarize_remote_call_rate(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<SummaryOutput> {
    assert_eq!(summary.scenario_name, "remote_call_rate");

    let dispatch_frame =
        query::query_custom_data(client.clone(), &summary, "wt.custom.remote_call_dispatch")
            .await
            .context("Load send data")?;

    let round_trip_frame =
        query::query_custom_data(client.clone(), &summary, "wt.custom.remote_call_round_trip")
            .await
            .context("Load recv data")?;

    let call_count = query::query_zome_call_instrument_data(client.clone(), &summary)
        .await
        .context("Load zome call data")?;

    let rate = call_count
        .clone()
        .lazy()
        .select([col("time"), col("value")])
        .filter(col("time").is_not_null())
        .sort(
            ["time"],
            SortMultipleOptions::default().with_maintain_order(true),
        )
        .group_by_dynamic(
            col("time"),
            [],
            DynamicGroupOptions {
                every: Duration::parse("10s"),
                period: Duration::parse("10s"),
                offset: Duration::parse("0"),
                ..Default::default()
            },
        )
        .agg([col("value").count()])
        .collect()?;

    // Slice to drop the first and last because they're likely to be partially filled windows.
    // What we really want is the average rate when the system is under load for the complete window.
    let average = rate
        .column("value")?
        .slice(1, rate.height() - 2)
        .mean()
        .context("Calculate average")?;

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
        RemoteCallRateSummary {
            dispatch: standard_timing_stats(dispatch_frame, "value", None)
                .context("Send timing stats")?,
            round_trip: standard_timing_stats(round_trip_frame, "value", None)
                .context("Recv timing stats")?,
            average_rate_10s: average,
            error_count,
        },
    )
}
