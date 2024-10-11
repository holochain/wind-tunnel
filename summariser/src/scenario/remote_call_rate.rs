use crate::analyze::{standard_rate, standard_timing_stats};
use crate::frame::LoadError;
use crate::model::{StandardRateStats, StandardTimingsStats, SummaryOutput};
use crate::query;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RemoteCallRateSummary {
    dispatch: StandardTimingsStats,
    round_trip: StandardTimingsStats,
    rate_10s: StandardRateStats,
    error_count: usize,
}

pub(crate) async fn summarize_remote_call_rate(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<SummaryOutput> {
    assert_eq!(summary.scenario_name, "remote_call_rate");

    let dispatch_frame = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.remote_call_dispatch",
        &[],
    )
    .await
    .context("Load send data")?;

    let round_trip_frame = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.remote_call_round_trip",
        &[],
    )
    .await
    .context("Load recv data")?;

    let call_count = query::query_zome_call_instrument_data(client.clone(), &summary)
        .await
        .context("Load zome call data")?;

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
            rate_10s: standard_rate(call_count.clone(), "value", "10s")?,
            error_count,
        },
    )
}
