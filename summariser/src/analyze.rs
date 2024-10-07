use crate::model::{StandardRatioStats, StandardTimingsStats};
use anyhow::Context;
use polars::frame::DataFrame;
use polars::prelude::*;

pub(crate) fn standard_timing_stats(
    frame: DataFrame,
    column: &str,
    skip: Option<i64>,
) -> anyhow::Result<StandardTimingsStats> {
    let mut value_series = frame.column(column)?.clone();
    if let Some(skip) = skip {
        value_series = value_series.slice(skip, usize::MAX);
    }

    let mean = value_series.mean().context("Mean")?;
    let std = value_series.std(0).context("Std")?;

    let out = frame
        .clone()
        .lazy()
        .select([
            col(column)
                .gt_eq(lit(mean - std))
                .and(col(column).lt_eq(lit(mean + std)))
                .alias("within_std"),
            col(column)
                .gt_eq(lit(mean - 2.0 * std))
                .and(col(column).lt_eq(lit(mean + 2.0 * std)))
                .alias("within_2std"),
            col(column)
                .gt_eq(lit(mean - 3.0 * std))
                .and(col(column).lt_eq(lit(mean + 3.0 * std)))
                .alias("within_3std"),
        ])
        .collect()?;

    let count = out
        .column("within_std")?
        .sum::<usize>()
        .context("Within std sum")?;
    let pct_within_1std = bound_pct(count as f64 / frame.column(column)?.len() as f64);

    let count = out
        .column("within_2std")?
        .sum::<usize>()
        .context("Within 2std sum")?;
    let pct_within_2std = bound_pct(count as f64 / frame.column(column)?.len() as f64);

    let count = out
        .column("within_3std")?
        .sum::<usize>()
        .context("Within 3std sum")?;
    let pct_within_3std = bound_pct(count as f64 / frame.column(column)?.len() as f64);

    Ok(StandardTimingsStats {
        mean,
        std,
        within_std: pct_within_1std,
        within_2std: pct_within_2std,
        within_3std: pct_within_3std,
    })
}

pub(crate) fn standard_ratio_stats(
    frame: DataFrame,
    column: &str,
) -> anyhow::Result<StandardRatioStats> {
    let value_series = frame.column(column)?.clone();

    let mean = value_series.mean().context("Mean")?;
    let std = value_series.std(0).context("Std")?;
    let min = value_series
        .min::<f64>()
        .context("Min")?
        .context("Missing min")?;
    let max = value_series
        .max::<f64>()
        .context("Max")?
        .context("Missing max")?;

    Ok(StandardRatioStats {
        mean,
        std,
        min,
        max,
    })
}

#[inline]
fn bound_pct(value: f64) -> f64 {
    value
}
