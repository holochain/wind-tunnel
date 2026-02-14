use crate::frame::LoadError;
use crate::model::{
    CounterStats, GaugeStats, GaugeTrend, PartitionGaugeStats, PartitionKey, PartitionRateStats,
    PartitionTimingStats, PartitionedGaugeStats, PartitionedRateStats, PartitionedTimingStats,
    StandardRateStats, StandardTimingsStats, TimingTrend,
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
    let mut value_col = frame.column(column).context("Read value column")?.clone();
    if let Some(skip) = skip {
        value_col = value_col.slice(skip, usize::MAX);
    }
    let value_series = value_col.as_materialized_series();

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
        .as_materialized_series()
        .sum::<usize>()
        .context("Within std sum")?;
    let pct_within_1std = bound_pct(count as f64 / frame.column(column)?.len() as f64);

    let count = out
        .column("within_2std")?
        .as_materialized_series()
        .sum::<usize>()
        .context("Within 2std sum")?;
    let pct_within_2std = bound_pct(count as f64 / frame.column(column)?.len() as f64);

    let count = out
        .column("within_3std")?
        .as_materialized_series()
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
                every: Duration::parse(window_duration),
                period: Duration::parse(window_duration),
                offset: Duration::parse("0s"),
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
        mean: round_to_n_dp(mean, 6),
        mean_std_dev,
        timings,
    })
}

/// Calculates [`crate::model::PartitionTimingStats`] for the given DataFrame and
/// if no series were found in influx, defaults to:
///
/// PartitionedTimingStats {
///     mean: 0.0,
///     mean_std_dev: 0.0,
///     timings: vec![],
/// }
///
/// This is useful in cases where no series found is an expected case, for example
/// for metrics that track errors of which none may occur during a scenario run.
pub(crate) fn partitioned_timing_stats_allow_empty(
    frame: anyhow::Result<DataFrame>,
    column: &str,
    window_duration: &str,
    partition_by: &[&str],
) -> anyhow::Result<PartitionedTimingStats> {
    match frame {
        Ok(df) => partitioned_timing_stats(df, column, window_duration, partition_by),
        Err(e) => {
            if let Some(LoadError::NoSeriesInResult { .. }) = e.downcast_ref::<LoadError>() {
                // It is expected that there often is no error at all so if we find no series, return an empty count
                Ok(PartitionedTimingStats {
                    mean: 0.0,
                    mean_std_dev: 0.0,
                    timings: vec![],
                })
            } else {
                Err(e)
            }
        }
    }
}

pub(crate) fn standard_rate(
    frame: DataFrame,
    column: &str,
    window_duration: &str,
) -> anyhow::Result<StandardRateStats> {
    let rate = frame
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
                offset: Duration::parse("0s"),
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

    let trend: Vec<u32> = if rate.height() <= 2 {
        Vec::with_capacity(0)
    } else {
        rate.column("count")?
            .slice(1, rate.height() - 2)
            .u32()?
            .iter()
            .map(|v| v.unwrap_or(0))
            .collect()
    };

    let mean = if rate.height() <= 2 {
        0.0
    } else {
        rate.column("count")?
            .slice(1, rate.height() - 2)
            .as_materialized_series()
            .mean()
            .context("Calculate average")?
    };

    Ok(StandardRateStats {
        mean: round_to_n_dp(mean, 2),
        trend,
        window_duration: window_duration.to_string(),
    })
}

/// Get the [`GaugeStats`] for a frame and the given column calculating:
///
/// - Arithmetic mean
/// - Std deviation
/// - p5 / p95 (90% operating range)
/// - Windowed mean trend
pub(crate) fn gauge_stats(
    frame: DataFrame,
    column: &str,
    window_duration: &str,
) -> anyhow::Result<GaugeStats> {
    let select = frame
        .lazy()
        .select([col("time"), col(column).cast(DataType::Float64)])
        .filter(col("time").is_not_null())
        .filter(col(column).is_not_null())
        .collect()?;

    let series = select.column(column)?.as_materialized_series();

    let mean = series.mean().context("Mean")?;
    let std = series.std(0).context("Std")?;

    // Percentile range instead of raw min/max — robust to single-sample spikes
    let sorted = series.f64()?.sort(false);
    let len = sorted.len();
    let (p5, p95) = if len == 0 {
        (0.0, 0.0)
    } else {
        let p5_idx = ((len as f64 * 0.05) as usize).min(len - 1);
        let p95_idx = ((len as f64 * 0.95) as usize).min(len - 1);
        (
            sorted.get(p5_idx).unwrap_or(0.0),
            sorted.get(p95_idx).unwrap_or(0.0),
        )
    };

    // Windowed mean trend — shows how the gauge evolves over time
    let trend = select
        .lazy()
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
                offset: Duration::parse("0s"),
                ..Default::default()
            },
        )
        .agg([col(column).mean().alias("mean")])
        .collect()
        .context("Windowed mean")?
        .column("mean")?
        .f64()?
        .iter()
        .filter_map(|v| v.map(|v| round_to_n_dp(v, 2)))
        .collect_vec();

    Ok(GaugeStats {
        mean: round_to_n_dp(mean, 2),
        std: round_to_n_dp(std, 2),
        p5: round_to_n_dp(p5, 2),
        p95: round_to_n_dp(p95, 2),
        trend: GaugeTrend {
            trend,
            window_duration: window_duration.to_string(),
        },
    })
}

/// Get the [`GaugeStats`] for a frame and the given column, partitioned
/// by tags.
pub(crate) fn partitioned_gauge_stats(
    frame: DataFrame,
    column: &str,
    partition_by: &[&str],
    window_duration: &str,
) -> anyhow::Result<PartitionedGaugeStats> {
    let mut select_cols = vec![col("time"), col(column)];
    select_cols.extend(partition_by.iter().map(|&s| col(s)));

    let unique = frame
        .clone()
        .lazy()
        .select(partition_by.iter().map(|&s| col(s)).collect::<Vec<_>>())
        .unique_stable(None, UniqueKeepStrategy::First)
        .collect()?;

    let mut partitions = Vec::new();
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

        let gauge_stats = gauge_stats(filtered, column, window_duration)?;

        partitions.push(PartitionGaugeStats { key, gauge_stats });
    }
    partitions.sort_by_key(|v| v.key.clone());

    Ok(PartitionedGaugeStats { partitions })
}

/// Get the [`CounterStats`] for a frame and the given column calculating:
///
/// - start value
/// - end value
/// - delta (end - start)
/// - rate
pub(crate) fn counter_stats(frame: DataFrame, column: &str) -> anyhow::Result<CounterStats> {
    let select = frame
        .lazy()
        .select([col("time"), col(column)])
        .filter(col("time").is_not_null())
        .filter(col(column).is_not_null())
        .sort(
            ["time"],
            SortMultipleOptions::default().with_maintain_order(true),
        )
        .collect()?;

    let times = select.column("time")?.as_materialized_series();
    let series = select
        .column(column)?
        .as_materialized_series()
        .i64()
        .context("Get series as i64")?;

    // get the window duration
    let times = times.datetime().context("Get times")?;
    let window_duration = times
        .max()
        .context("Get max time")?
        .checked_sub(times.min().context("Get min time")?)
        .context("Get window duration")?;
    let window_duration = std::time::Duration::from_nanos(window_duration as u64);
    log::debug!("Window duration: {window_duration:?}",);

    // get first element
    let min = series.first().context("Get first")? as u64;
    let max = series.last().context("Get last")? as u64;
    let count = max
        .checked_sub(min)
        .ok_or(anyhow::anyhow!("Counter underflow: min {min} > max {max}",))?;
    let rate = if count == 0 || select.height() < 2 {
        0.0
    } else {
        (count as f64) / window_duration.as_secs_f64()
    };

    Ok(CounterStats {
        count,
        rate_per_second: round_to_n_dp(rate, 2),
        measurement_duration: window_duration,
    })
}

pub(crate) fn partitioned_rate_stats(
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

        let summary_rate = standard_rate(filtered, column, window_duration)
            .with_context(|| format!("Standard rate for {key:?}"))?;

        rates.push(PartitionRateStats { key, summary_rate });
    }
    rates.sort_by_key(|v| v.key.clone());

    let mean = rates.iter().map(|t| t.summary_rate.mean).sum::<f64>() / rates.len() as f64;

    Ok(PartitionedRateStats {
        mean: round_to_n_dp(mean, 2),
        rates,
    })
}

pub fn bound_pct(value: f64) -> f64 {
    (value.clamp(0.0, 1.0) * 10_000.0).round() / 10_000.0
}

pub fn round_to_n_dp(value: f64, n: u32) -> f64 {
    let places = 10.0_f64.powi(n as i32);
    (value * places).round() / places
}

/// Return the mean of a single column in a [`DataFrame`], or 0.0 if the column
/// is missing or empty.
pub fn column_mean(df: &DataFrame, column: &str) -> f64 {
    df.column(column)
        .ok()
        .and_then(|c| c.as_materialized_series().mean())
        .unwrap_or(0.0)
}

/// Calculate percentiles (p50, p95, p99) from a DataFrame column.
///
/// The column is cast to Float64 to handle integer-typed columns from InfluxDB.
pub(crate) fn percentiles(df: &DataFrame, column: &str) -> anyhow::Result<(f64, f64, f64)> {
    let series = df.column(column)?.cast(&DataType::Float64)?;
    let sorted = series.f64()?.sort(false);

    let count = sorted.len();
    if count == 0 {
        log::info!("No data points found for percentile calculation");
        return Ok((0.0, 0.0, 0.0));
    }

    let p50_idx = ((count as f64 * 0.50) as usize).min(count - 1);
    let p95_idx = ((count as f64 * 0.95) as usize).min(count - 1);
    let p99_idx = ((count as f64 * 0.99) as usize).min(count - 1);

    Ok((
        sorted.get(p50_idx).unwrap_or(0.0),
        sorted.get(p95_idx).unwrap_or(0.0),
        sorted.get(p99_idx).unwrap_or(0.0),
    ))
}

/// Detect if values are trending upward (potential leak/growth)
///
/// Compute the growth rate of `column` in units-per-second by fitting a
/// least-squares line to the sample values.
///
/// Uses the row index as the x-axis (evenly spaced samples) and
/// `slope = cov(x, y) / var(x)` to get the gradient, then converts from
/// per-sample to per-second via `duration_secs / n`.
pub(crate) fn growth_rate(df: &DataFrame, column: &str, duration_secs: f64) -> anyhow::Result<f64> {
    let values = df.column(column)?.cast(&DataType::Float64)?.f64()?.clone();
    let n = values.len();

    if n < 2 || duration_secs == 0.0 {
        return Ok(0.0);
    }

    let nf = n as f64;
    // For x_i = 0..n-1:  mean = (n-1)/2,  var = (n²-1)/12
    let x_mean = (nf - 1.0) / 2.0;
    let x_var = (nf * nf - 1.0) / 12.0;

    let y_mean = values.mean().unwrap_or(0.0);

    // cov(x, y) = mean((x - x̄)(y - ȳ)) = mean(x·y) - x̄·ȳ
    let mean_xy = values
        .iter()
        .enumerate()
        .map(|(i, v)| i as f64 * v.unwrap_or(0.0))
        .sum::<f64>()
        / nf;
    let cov_xy = mean_xy - x_mean * y_mean;

    // slope per sample → slope per second
    let slope_per_sample = cov_xy / x_var;
    let secs_per_sample = duration_secs / nf;

    Ok(slope_per_sample / secs_per_sample)
}
