use crate::model::SummaryOutput;
use crate::query;
use anyhow::Context;
use polars::prelude::*;
use serde::Serialize;
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize)]
struct AppInstallSummary {
    first_install: f64,
    mean: f64,
    std: f64,
    within_std: f64,
    within_2std: f64,
    within_3std: f64,
}

pub(crate) async fn summarize_app_install(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<SummaryOutput> {
    assert_eq!(summary.scenario_name, "app_install");

    let frame = query::query_instrument_data(client.clone(), &summary, "admin_install_app")
        .await
        .context("App install max")?;

    let first = frame
        .column("value")?
        .get(0)
        .context("First")?
        .try_extract::<f64>()?;
    let value_series = frame.column("value")?.slice(1, usize::MAX);
    let mean = value_series.mean().context("Mean")?;
    let std = value_series.std(0).context("Std")?;

    let out = frame
        .clone()
        .lazy()
        .select([
            col("value")
                .gt_eq(lit(mean - std))
                .and(col("value").lt_eq(lit(mean + std)))
                .alias("within_std"),
            col("value")
                .gt_eq(lit(mean - 2.0 * std))
                .and(col("value").lt_eq(lit(mean + 2.0 * std)))
                .alias("within_2std"),
            col("value")
                .gt_eq(lit(mean - 3.0 * std))
                .and(col("value").lt_eq(lit(mean + 3.0 * std)))
                .alias("within_3std"),
        ])
        .collect()?;

    let count = out
        .column("within_std")?
        .sum::<usize>()
        .context("Within std sum")?;
    let pct_within_1std = bound_pct(count as f64 / frame.column("value")?.len() as f64);

    let count = out
        .column("within_2std")?
        .sum::<usize>()
        .context("Within 2std sum")?;
    let pct_within_2std = bound_pct(count as f64 / frame.column("value")?.len() as f64);

    let count = out
        .column("within_3std")?
        .sum::<usize>()
        .context("Within 3std sum")?;
    let pct_within_3std = bound_pct(count as f64 / frame.column("value")?.len() as f64);

    SummaryOutput::new(
        summary,
        AppInstallSummary {
            first_install: first,
            mean,
            std,
            within_std: pct_within_1std,
            within_2std: pct_within_2std,
            within_3std: pct_within_3std,
        },
    )
}

#[inline]
fn bound_pct(value: f64) -> f64 {
    value
}
