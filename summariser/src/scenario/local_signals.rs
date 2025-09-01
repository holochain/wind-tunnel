use crate::aggregator::{try_aggregate_holochain_metrics, HostMetricsAggregator};
use crate::analyze::{round_to_n_dp, standard_timing_stats};
use crate::model::{HolochainMetricsConfig, StandardTimingsStats, SummaryOutput};
use crate::query;
use anyhow::Context;
use polars::frame::DataFrame;
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocalSignalsSummary {
    send: StandardTimingsStats,
    recv: StandardTimingsStats,
    success_ratio: RatioStats,
}

pub(crate) async fn summarize_local_signals(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<SummaryOutput> {
    assert_eq!(summary.scenario_name, "local_signals");

    let send_frame =
        query::query_custom_data(client.clone(), &summary, "wt.custom.signal_batch_send", &[])
            .await
            .context("Load send data")?;

    let recv_frame =
        query::query_custom_data(client.clone(), &summary, "wt.custom.signal_batch_recv", &[])
            .await
            .context("Load recv data")?;

    let success_ratio = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.signal_success_ratio",
        &[],
    )
    .await
    .context("Load success ratio")?;

    let host_metrics = HostMetricsAggregator::new(&client, &summary)
        .try_aggregate()
        .await;

    let holochain_metrics =
        try_aggregate_holochain_metrics(&client, &summary, HolochainMetricsConfig::none()).await;
    SummaryOutput::new(
        summary,
        LocalSignalsSummary {
            send: standard_timing_stats(send_frame, "value", "10s", None)
                .context("Send timing stats")?,
            recv: standard_timing_stats(recv_frame, "value", "10s", None)
                .context("Recv timing stats")?,
            success_ratio: ratio_stats(success_ratio, "value").context("Success ratio stats")?,
        },
        host_metrics,
        holochain_metrics,
    )
}

pub(crate) fn ratio_stats(frame: DataFrame, column: &str) -> anyhow::Result<RatioStats> {
    let value_col = frame.column(column)?.clone();
    let values_series = value_col.as_materialized_series();

    let mean = values_series.mean().context("Mean")?;
    let std = values_series.std(0).context("Std")?;
    let min = values_series
        .min::<f64>()
        .context("Min")?
        .context("Missing min")?;
    let max = values_series
        .max::<f64>()
        .context("Max")?
        .context("Missing max")?;

    Ok(RatioStats {
        mean: round_to_n_dp(mean, 4),
        std: round_to_n_dp(std, 4),
        min,
        max,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatioStats {
    pub mean: f64,
    pub std: f64,
    pub min: f64,
    pub max: f64,
}
