use crate::frame::frame_to_json;
use crate::model::{
    PartitionedRateStats, StandardRateStats, StandardRatioStats, StandardTimingsStats,
};
use anyhow::Context;
use polars::frame::DataFrame;
use polars::prelude::*;
use std::collections::HashMap;

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

pub(crate) fn standard_rate(
    frame: DataFrame,
    column: &str,
    window_duration: &str,
) -> anyhow::Result<StandardRateStats> {
    let mut rate = frame
        .clone()
        .lazy()
        .select([col("time"), col(column)])
        .filter(col("time").is_not_null())
        .sort(
            ["time"],
            SortMultipleOptions::default().with_maintain_order(true),
        )
        .group_by_dynamic(
            col("time"),
            [],
            DynamicGroupOptions {
                every: Duration::parse(window_duration),
                period: Duration::parse(window_duration),
                offset: Duration::parse("0"),
                ..Default::default()
            },
        )
        .agg([col(column).count().alias("count")])
        .sort(
            ["time"],
            SortMultipleOptions::default().with_maintain_order(true),
        )
        .collect()?;

    // Slice to drop the first and last because they're likely to be partially filled windows.
    // What we really want is the average rate when the system is under load for the complete window.
    let mean = rate
        .column("count")?
        .slice(1, rate.height() - 2)
        .mean()
        .context("Calculate average");

    Ok(StandardRateStats {
        trend: frame_to_json(&mut rate)?,
        mean_rate: mean?,
    })
}

pub(crate) fn partitioned_rate(
    frame: DataFrame,
    column: &str,
    window_duration: &str,
    partition_by: &[&str],
) -> anyhow::Result<PartitionedRateStats> {
    let mut select_cols = vec![col("time"), col(column)];
    select_cols.extend(partition_by.iter().map(|&s| col(s)));

    let frame = frame
        .clone()
        .lazy()
        .select(select_cols)
        .filter(col("time").is_not_null())
        .sort(
            ["time"],
            SortMultipleOptions::default().with_maintain_order(true),
        )
        .group_by_dynamic(
            col("time"),
            partition_by.iter().map(|&s| col(s)).collect::<Vec<_>>(),
            DynamicGroupOptions {
                every: Duration::parse(window_duration),
                period: Duration::parse(window_duration),
                offset: Duration::parse("0"),
                ..Default::default()
            },
        )
        .agg([col(column).count()])
        .collect()?;

    let mut trend = frame
        .clone()
        .lazy()
        .select([col("time"), col(column)])
        .group_by([col("time")])
        .agg([col(column).sum().alias("count")])
        .sort(
            ["time"],
            SortMultipleOptions::default().with_maintain_order(true),
        )
        .collect()?;

    let mut out = PartitionedRateStats {
        trend: frame_to_json(&mut trend)?,
        rates: HashMap::new(),
        by_partition: HashMap::new(),
    };
    for prefix_len in 1..=partition_by.len() {
        let mut group_sum_cols = vec![col("time")];
        for c in partition_by.iter().take(prefix_len) {
            group_sum_cols.push(col(*c));
        }

        let group_out_cols = partition_by[0..prefix_len]
            .iter()
            .map(|s| col(*s))
            .collect::<Vec<_>>();

        let mut grouped = frame
            .clone()
            .lazy()
            .group_by(group_sum_cols)
            .agg([col(column).sum()])
            .collect()?
            .lazy()
            .group_by(group_out_cols)
            .agg([col(column)
                .slice(1, u32::MAX)
                .reverse()
                .slice(1, u32::MAX)
                .mean()
                .alias("mean")])
            .collect()?;

        let mean = grouped
            .column("mean")?
            .mean()
            .context("Calculate average")?;

        let key = partition_by[0..prefix_len].join("-");

        out.rates
            .insert(format!("{}-mean-rate-10s", key.clone()), mean);
        out.by_partition
            .insert(key.clone(), frame_to_json(&mut grouped)?);
    }

    Ok(out)
}

#[inline]
fn bound_pct(value: f64) -> f64 {
    value
}
