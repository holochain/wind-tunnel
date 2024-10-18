use crate::model::{
    PartitionKey, PartitionRateStats, PartitionTimingStats, PartitionedRateStats,
    PartitionedTimingStats, StandardRateStats, StandardTimingsStats, TimingTrend,
};
use anyhow::Context;
use itertools::Itertools;
use polars::frame::DataFrame;
use polars::prelude::*;

pub(crate) fn standard_timing_stats(
    frame: DataFrame,
    column: &str,
    window_duration: &str,
    skip: Option<i64>,
) -> anyhow::Result<StandardTimingsStats> {
    let mut value_series = frame.column(column).context("Read value column")?.clone();
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
                .round(2)
                .gt_eq(lit(mean - 2.0 * std))
                .and(col(column).lt_eq(lit(mean + 2.0 * std)))
                .alias("within_2std"),
            col(column)
                .round(2)
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

    let trend = frame
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
                // every: Duration::parse(window_duration),
                every: Duration::parse(window_duration),
                period: Duration::parse(window_duration),
                offset: Duration::parse("0"),
                ..Default::default()
            },
        )
        .agg([col(column)
            .slice(1, u32::MAX)
            .reverse()
            .slice(1, u32::MAX)
            .mean()
            .alias("mean")])
        .collect()
        .context("Windowed mean")?
        .column("mean")?
        .f64()?
        .iter()
        .filter_map(|v| v.map(|v| round_to_n_dp(v, 6)))
        .collect_vec();

    Ok(StandardTimingsStats {
        mean: round_to_n_dp(mean, 6),
        std: round_to_n_dp(std, 6),
        within_std: pct_within_1std,
        within_2std: pct_within_2std,
        within_3std: pct_within_3std,
        trend: TimingTrend {
            trend,
            window_duration: window_duration.to_string(),
        },
    })
}

pub(crate) fn partitioned_timing_stats(
    frame: DataFrame,
    column: &str,
    window_duration: &str,
    partition_by: &[&str],
) -> anyhow::Result<PartitionedTimingStats> {
    let mut select_cols = vec![col("time"), col(column)];
    select_cols.extend(partition_by.iter().map(|&s| col(s)));

    let unique = frame
        .clone()
        .lazy()
        .select(partition_by.iter().map(|&s| col(s)).collect::<Vec<_>>())
        .unique_stable(None, UniqueKeepStrategy::First)
        .collect()?;

    let mut timings = Vec::new();
    for i in 0..unique.height() {
        let mut filter_expr = col("time").is_not_null();
        let mut key = Vec::new();
        for c in partition_by {
            let value = unique
                .column(c)?
                .get(i)?
                .get_str()
                .context("Get string")?
                .to_string();
            key.push(PartitionKey {
                key: c.to_string(),
                value: value.to_string(),
            });
            filter_expr = filter_expr.and(col(*c).eq(lit(value)));
        }
        key.sort();

        let filtered = frame
            .clone()
            .lazy()
            .select(select_cols.clone())
            .filter(filter_expr)
            .collect()
            .context("filter by partition")?;

        let summary_timing = standard_timing_stats(filtered, column, window_duration, None)?;

        timings.push(PartitionTimingStats {
            key,
            summary_timing,
        });
    }
    timings.sort_by_key(|v| v.key.clone());

    let mean = timings.iter().map(|t| t.summary_timing.mean).sum::<f64>() / timings.len() as f64;
    let mean_std_dev = round_to_n_dp(
        timings.iter().map(|t| t.summary_timing.std).sum::<f64>() / timings.len() as f64,
        6,
    );

    Ok(PartitionedTimingStats {
        mean,
        mean_std_dev,
        timings,
    })
}

pub(crate) fn standard_rate(
    frame: DataFrame,
    column: &str,
    window_duration: &str,
) -> anyhow::Result<StandardRateStats> {
    let rate = frame
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

    let trend = rate
        .column("count")?
        .slice(1, rate.height() - 2)
        .u32()?
        .iter()
        .map(|v| v.unwrap_or(0))
        .collect();

    // Slice to drop the first and last because they're likely to be partially filled windows.
    // What we really want is the average rate when the system is under load for the complete window.
    let mean = rate
        .column("count")?
        .slice(1, rate.height() - 2)
        .mean()
        .context("Calculate average")?;

    Ok(StandardRateStats {
        mean,
        trend,
        window_duration: window_duration.to_string(),
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

    let unique = frame
        .clone()
        .lazy()
        .select(partition_by.iter().map(|&s| col(s)).collect::<Vec<_>>())
        .unique_stable(None, UniqueKeepStrategy::First)
        .collect()?;

    let mut rates = Vec::new();
    for i in 0..unique.height() {
        let mut filter_expr = col("time").is_not_null();
        let mut key = Vec::new();
        for c in partition_by {
            let value = unique
                .column(c)?
                .get(i)?
                .get_str()
                .context("Get string")?
                .to_string();
            key.push(PartitionKey {
                key: c.to_string(),
                value: value.to_string(),
            });
            filter_expr = filter_expr.and(col(*c).eq(lit(value)));
        }
        key.sort();

        let filtered = frame
            .clone()
            .lazy()
            .select(select_cols.clone())
            .filter(filter_expr)
            .collect()
            .context("filter by partition")?;

        let summary_rate = standard_rate(filtered, column, window_duration)?;

        rates.push(PartitionRateStats { key, summary_rate });
    }
    rates.sort_by_key(|v| v.key.clone());

    let mean = rates.iter().map(|t| t.summary_rate.mean).sum::<f64>() / rates.len() as f64;

    Ok(PartitionedRateStats { mean, rates })
}

fn bound_pct(value: f64) -> f64 {
    (value.clamp(0.0, 1.0) * 10_000.0).round() / 10_000.0
}

pub fn round_to_n_dp(value: f64, n: u32) -> f64 {
    let places = 10.0_f64.powi(n as i32);
    (value * places).round() / places
}
