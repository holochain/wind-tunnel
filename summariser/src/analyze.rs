use crate::frame::LoadError;
use crate::model::{
    ChainHeadStats, CounterStats, Float64Trend, GaugeStats, PartitionGaugeStats, PartitionKey,
    PartitionedCounterStats, PartitionedGaugeStats, PartitionedRateStats, PartitionedTimingStats,
    StandardRateStats, StandardTimingsStats,
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

    let cast_series = value_series.cast(&DataType::Float64)?;
    let sorted = cast_series.f64()?.sort(false);
    let p50 = round_to_n_dp(sorted_percentile(&sorted, 0.50), 6);
    let p95 = round_to_n_dp(sorted_percentile(&sorted, 0.95), 6);
    let p99 = round_to_n_dp(sorted_percentile(&sorted, 0.99), 6);

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
        .agg([col(column).mean().alias("mean")])
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
        p50,
        p95,
        p99,
        trend: Float64Trend {
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

    let partition_count = unique.height();
    if partition_count == 0 {
        return Ok(PartitionedTimingStats {
            mean: 0.0,
            mean_std_dev: 0.0,
            max_mean: 0.0,
            min_mean: 0.0,
            trend: vec![],
            window_duration: window_duration.to_string(),
        });
    }

    let mut sum_mean = 0.0_f64;
    let mut sum_std = 0.0_f64;
    let mut max_mean = 0.0_f64;
    let mut min_mean = f64::MAX;

    for i in 0..partition_count {
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

        let filtered = frame
            .clone()
            .lazy()
            .select(select_cols.clone())
            .filter(filter_expr)
            .collect()
            .context("filter by partition")?;

        let summary_timing = standard_timing_stats(filtered, column, window_duration, None)
            .with_context(|| format!("Timing stats for {key:?}"))?;

        sum_mean += summary_timing.mean;
        sum_std += summary_timing.std;
        if summary_timing.mean > max_mean {
            max_mean = summary_timing.mean;
        }
        if summary_timing.mean < min_mean {
            min_mean = summary_timing.mean;
        }
    }

    let mean = sum_mean / partition_count as f64;
    let mean_std_dev = round_to_n_dp(sum_std / partition_count as f64, 6);

    // Compute partitioned trend vectorially: per-(window, partition) mean latency,
    // then averaged across partitions per window.
    let partition_cols: Vec<Expr> = partition_by.iter().map(|&s| col(s)).collect();
    let trend_frame = frame
        .lazy()
        .filter(col("time").is_not_null())
        .sort(
            ["time"],
            SortMultipleOptions::default().with_maintain_order(true),
        )
        .group_by_dynamic(
            col("time"),
            partition_cols,
            DynamicGroupOptions {
                every: Duration::parse(window_duration),
                period: Duration::parse(window_duration),
                offset: Duration::parse("0s"),
                ..Default::default()
            },
        )
        .agg([col(column).mean().alias("mean_val")])
        .group_by([col("time")])
        .agg([col("mean_val").mean().alias("mean_across")])
        .sort(
            ["time"],
            SortMultipleOptions::default().with_maintain_order(true),
        )
        .collect()?;

    let trend: Vec<f64> = trend_frame
        .column("mean_across")?
        .f64()?
        .iter()
        .map(|v| round_to_n_dp(v.unwrap_or(0.0), 6))
        .collect();

    Ok(PartitionedTimingStats {
        mean: round_to_n_dp(mean, 6),
        mean_std_dev,
        max_mean: round_to_n_dp(max_mean, 6),
        min_mean: round_to_n_dp(if min_mean == f64::MAX { 0.0 } else { min_mean }, 6),
        trend,
        window_duration: window_duration.to_string(),
    })
}

/// Get the [`PartitionedCounterStats`] for a frame and the given column, partitioned
/// by tags. Applies [`counter_stats`] per partition, then sums counts across partitions.
pub(crate) fn partitioned_counter_stats(
    frame: DataFrame,
    column: &str,
    window_duration: &str,
    partition_by: &[&str],
) -> anyhow::Result<PartitionedCounterStats> {
    let mut select_cols = vec![col("time"), col(column)];
    select_cols.extend(partition_by.iter().map(|&s| col(s)));

    let unique = frame
        .clone()
        .lazy()
        .select(partition_by.iter().map(|&s| col(s)).collect::<Vec<_>>())
        .unique_stable(None, UniqueKeepStrategy::First)
        .collect()?;

    let partition_count = unique.height();
    let mut total_count: u64 = 0;
    let mut partitions_above_zero: usize = 0;
    let mut max_per_partition: u64 = 0;
    let mut min_per_partition: u64 = u64::MAX;

    for i in 0..partition_count {
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

        let filtered = frame
            .clone()
            .lazy()
            .select(select_cols.clone())
            .filter(filter_expr)
            .collect()
            .context("filter by partition")?;

        let stats = counter_stats(filtered, column, window_duration)
            .with_context(|| format!("Counter stats for {key:?}"))?;

        total_count += stats.count;
        if stats.count > 0 {
            partitions_above_zero += 1;
        }
        if stats.count > max_per_partition {
            max_per_partition = stats.count;
        }
        if stats.count < min_per_partition {
            min_per_partition = stats.count;
        }
    }

    // Compute partitioned trend vectorially: delta (last - first) per (window, partition),
    // clamped to zero for counter resets, then averaged across partitions per window.
    let partition_cols: Vec<Expr> = partition_by.iter().map(|&s| col(s)).collect();
    let trend_frame = frame
        .lazy()
        .filter(col("time").is_not_null())
        .filter(col(column).is_not_null())
        .sort(
            ["time"],
            SortMultipleOptions::default().with_maintain_order(true),
        )
        .group_by_dynamic(
            col("time"),
            partition_cols,
            DynamicGroupOptions {
                every: Duration::parse(window_duration),
                period: Duration::parse(window_duration),
                offset: Duration::parse("0s"),
                ..Default::default()
            },
        )
        .agg([(col(column).last().cast(DataType::Float64)
            - col(column).first().cast(DataType::Float64))
        .alias("delta")])
        .with_column(
            when(col("delta").lt(lit(0.0_f64)))
                .then(lit(0.0_f64))
                .otherwise(col("delta"))
                .alias("delta"),
        )
        .group_by([col("time")])
        .agg([col("delta").mean().alias("mean_delta")])
        .sort(
            ["time"],
            SortMultipleOptions::default().with_maintain_order(true),
        )
        .collect()?;

    let trend: Vec<f64> = trend_frame
        .column("mean_delta")?
        .f64()?
        .iter()
        .map(|v| round_to_n_dp(v.unwrap_or(0.0), 4))
        .collect();

    let mean_count = if partition_count == 0 {
        0
    } else {
        (total_count as f64 / partition_count as f64).round() as u64
    };

    Ok(PartitionedCounterStats {
        total_count,
        partition_count,
        partitions_above_zero,
        mean_count,
        max_per_partition,
        min_per_partition: if min_per_partition == u64::MAX {
            0
        } else {
            min_per_partition
        },
        trend,
        window_duration: window_duration.to_string(),
    })
}

/// Like [`partitioned_counter_stats`] but returns an empty result when no series is
/// found in InfluxDB. Use this for metrics that track errors where zero occurrences
/// is a valid and expected outcome.
pub(crate) fn partitioned_counter_stats_allow_empty(
    frame: anyhow::Result<DataFrame>,
    column: &str,
    window_duration: &str,
    partition_by: &[&str],
) -> anyhow::Result<PartitionedCounterStats> {
    match frame {
        Ok(df) => partitioned_counter_stats(df, column, window_duration, partition_by),
        Err(e) => {
            if let Some(LoadError::NoSeriesInResult { .. }) = e.downcast_ref::<LoadError>() {
                Ok(PartitionedCounterStats {
                    total_count: 0,
                    partition_count: 0,
                    partitions_above_zero: 0,
                    mean_count: 0,
                    max_per_partition: 0,
                    min_per_partition: 0,
                    trend: vec![],
                    window_duration: window_duration.to_string(),
                })
            } else {
                Err(e)
            }
        }
    }
}

/// Compute [`ChainHeadStats`] from a frame containing observations from multiple reading agents
/// of multiple writing agents' chain heads.
///
/// The reading dimension is collapsed by grouping on `writer_tag` and taking the maximum
/// observed value per writing agent — any successful read by any reader counts as propagation.
/// The writing dimension is then summarised with mean and max across all writing agents.
///
/// A windowed trend is also computed: per time window, take `max(observed_value)` per write
/// agent, then average those maxes across write agents. This shows how chain advancement
/// progressed over the run — a flat or slowing trend indicates writing stalled or readers
/// caught up.
///
/// # Use cases
/// - `chain_len` (must_get_agent_activity scenarios): `writer_tag` = `"write_agent"`
/// - `highest_observed_action_seq` (get_agent_activity scenarios): `writer_tag` = `"write_agent"`
pub(crate) fn chain_head_stats(
    frame: DataFrame,
    column: &str,
    writer_tag: &str,
    window_duration: &str,
) -> anyhow::Result<ChainHeadStats> {
    let per_writer = frame
        .clone()
        .lazy()
        .filter(col("time").is_not_null())
        .group_by([col(writer_tag)])
        .agg([col(column).cast(DataType::Float64).max().alias("max_val")])
        .collect()
        .context("Group by writer and take max")?;

    let write_agent_count = per_writer.height();

    if write_agent_count == 0 {
        return Ok(ChainHeadStats {
            mean_max: 0.0,
            max: 0.0,
            write_agent_count: 0,
            trend: Float64Trend {
                trend: vec![],
                window_duration: window_duration.to_string(),
            },
        });
    }

    let max_vals = per_writer
        .column("max_val")
        .context("Get max_val column")?
        .f64()
        .context("Cast max_val to f64")?;

    let mean_max = max_vals.mean().unwrap_or(0.0);
    let max = max_vals.max().unwrap_or(0.0);

    // Per-window trend: for each window, take max per write_agent then average across agents.
    let trend_frame = frame
        .lazy()
        .filter(col("time").is_not_null())
        .sort(
            ["time"],
            SortMultipleOptions::default().with_maintain_order(true),
        )
        .group_by_dynamic(
            col("time"),
            [col(writer_tag)],
            DynamicGroupOptions {
                every: Duration::parse(window_duration),
                period: Duration::parse(window_duration),
                offset: Duration::parse("0s"),
                ..Default::default()
            },
        )
        .agg([col(column)
            .cast(DataType::Float64)
            .max()
            .alias("writer_max")])
        .group_by([col("time")])
        .agg([col("writer_max").mean().alias("mean_writer_max")])
        .sort(
            ["time"],
            SortMultipleOptions::default().with_maintain_order(true),
        )
        .collect()
        .context("Windowed chain head trend")?;

    let trend: Vec<f64> = trend_frame
        .column("mean_writer_max")
        .context("Get mean_writer_max column")?
        .f64()
        .context("Cast mean_writer_max to f64")?
        .iter()
        .map(|v| round_to_n_dp(v.unwrap_or(0.0), 2))
        .collect();

    Ok(ChainHeadStats {
        mean_max: round_to_n_dp(mean_max, 2),
        max: round_to_n_dp(max, 2),
        write_agent_count,
        trend: Float64Trend {
            trend,
            window_duration: window_duration.to_string(),
        },
    })
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

    let trend: Vec<u32> = rate
        .column("count")?
        .u32()?
        .iter()
        .map(|v| v.unwrap_or(0))
        .collect();

    // Slice to drop the first and last because they're likely to be partially filled windows.
    // What we really want is the average rate when the system is under load for the complete window.
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

/// Look up the value at a given percentile from a pre-sorted Float64 series.
///
/// `p` is in the range `0.0..=1.0`. Uses nearest-rank indexing clamped to valid
/// bounds. Returns `0.0` for an empty series.
fn sorted_percentile(sorted: &ChunkedArray<Float64Type>, p: f64) -> f64 {
    let len = sorted.len();
    if len == 0 {
        return 0.0;
    }
    let idx = ((len as f64 * p) as usize).min(len - 1);
    sorted.get(idx).unwrap_or(0.0)
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
    gauge_stats_dp(frame, column, window_duration, 2)
}

/// Like [`gauge_stats`] but rounds all output values to `dp` decimal places instead of 2.
///
/// Use this for metrics whose values are naturally very small (e.g. PSI percentages that
/// routinely sit below 0.01) where 2 dp would always produce 0.00.
pub(crate) fn gauge_stats_dp(
    frame: DataFrame,
    column: &str,
    window_duration: &str,
    dp: u32,
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
    let p5 = sorted_percentile(&sorted, 0.05);
    let p95 = sorted_percentile(&sorted, 0.95);

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
        .filter_map(|v| v.map(|v| round_to_n_dp(v, dp)))
        .collect_vec();

    Ok(GaugeStats {
        mean: round_to_n_dp(mean, dp),
        std: round_to_n_dp(std, dp),
        p5: round_to_n_dp(p5, dp),
        p95: round_to_n_dp(p95, dp),
        trend: Float64Trend {
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
/// - Total count (last - first)
/// - Instantaneous rate distribution (mean, std, p5, p95, peak) via vectorized diff
/// - Windowed rate trend
pub(crate) fn counter_stats(
    frame: DataFrame,
    column: &str,
    window_duration: &str,
) -> anyhow::Result<CounterStats> {
    let counter_by_time = frame
        .lazy()
        .select([col("time"), col(column)])
        .filter(col("time").is_not_null())
        .filter(col(column).is_not_null())
        .sort(
            ["time"],
            SortMultipleOptions::default().with_maintain_order(true),
        )
        .collect()?;

    let times = counter_by_time.column("time")?.as_materialized_series();
    let series = counter_by_time
        .column(column)?
        .as_materialized_series()
        .i64()
        .context("Get series as i64")?;

    // Measurement duration from timestamps
    let times_dt = times.datetime().context("Get times")?;
    let measurement_duration_ns = times_dt
        .last()
        .context("Get max time")?
        .checked_sub(times_dt.first().context("Get min time")?)
        .context("Get window duration")?;
    let measurement_duration = std::time::Duration::from_nanos(measurement_duration_ns as u64);
    log::debug!("Measurement duration: {measurement_duration:?}");

    // Total count: last - first
    let first = series.first().context("Get first")? as u64;
    let last = series.last().context("Get last")? as u64;
    let count = last.checked_sub(first).ok_or(anyhow::anyhow!(
        "Counter underflow: first {first} > last {last}"
    ))?;

    if count == 0 || counter_by_time.height() < 2 {
        return Ok(CounterStats {
            count,
            measurement_duration,
            ..Default::default()
        });
    }

    // Compute instantaneous rates from consecutive samples
    let times = counter_by_time
        .column("time")?
        .as_materialized_series()
        .datetime()
        .context("time as datetime")?;
    let values = counter_by_time
        .column(column)?
        .as_materialized_series()
        .i64()
        .context("values as i64")?;

    let mut rate_timestamps: Vec<i64> = Vec::new();
    let mut rate_values: Vec<f64> = Vec::new();

    for i in 1..counter_by_time.height() {
        let (Some(t_prev), Some(t_curr)) = (times.get(i - 1), times.get(i)) else {
            continue;
        };
        let dt_ns = t_curr - t_prev;
        if dt_ns <= 0 {
            continue;
        }
        let (Some(v_prev), Some(v_curr)) = (values.get(i - 1), values.get(i)) else {
            continue;
        };
        let delta = v_curr - v_prev;
        if delta < 0 {
            continue; // skip counter resets
        }
        let dt_secs = dt_ns as f64 / 1_000_000_000.0;
        rate_timestamps.push(t_curr);
        rate_values.push(delta as f64 / dt_secs);
    }

    let (mean, std, p5, p95, peak) = if rate_values.is_empty() {
        (0.0, 0.0, 0.0, 0.0, 0.0)
    } else {
        let rate_series = Series::new("rate".into(), &rate_values);
        let mean = rate_series.mean().unwrap_or(0.0);
        let std = rate_series.std(0).unwrap_or(0.0);

        let sorted = rate_series.f64()?.sort(false);
        (
            mean,
            std,
            sorted_percentile(&sorted, 0.05),
            sorted_percentile(&sorted, 0.95),
            sorted_percentile(&sorted, 1.0),
        )
    };

    // Windowed rate trend — build a DataFrame and use group_by_dynamic
    let trend = if rate_timestamps.is_empty() {
        Vec::new()
    } else {
        let time_series = Series::new("time".into(), &rate_timestamps)
            .cast(&DataType::Datetime(TimeUnit::Nanoseconds, None))?;
        let rate_series = Series::new("rate".into(), &rate_values);
        let rate_frame =
            DataFrame::new(vec![time_series.into_column(), rate_series.into_column()])?;

        rate_frame
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
            .agg([col("rate").mean().alias("mean")])
            .collect()
            .context("Windowed rate mean")?
            .column("mean")?
            .f64()?
            .iter()
            .filter_map(|v| v.map(|v| round_to_n_dp(v, 2)))
            .collect_vec()
    };

    Ok(CounterStats {
        count,
        measurement_duration,
        mean_rate_per_second: round_to_n_dp(mean, 2),
        std_rate_per_second: round_to_n_dp(std, 2),
        p5_rate_per_second: round_to_n_dp(p5, 2),
        p95_rate_per_second: round_to_n_dp(p95, 2),
        peak_rate_per_second: round_to_n_dp(peak, 2),
        rate_trend: Float64Trend {
            trend,
            window_duration: window_duration.to_string(),
        },
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

    let partition_count = unique.height();
    if partition_count == 0 {
        return Ok(PartitionedRateStats {
            mean: 0.0,
            max_mean: 0.0,
            min_mean: 0.0,
            trend: vec![],
            window_duration: window_duration.to_string(),
        });
    }

    let mut sum_mean = 0.0_f64;
    let mut max_mean = 0.0_f64;
    let mut min_mean = f64::MAX;

    for i in 0..partition_count {
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

        let filtered = frame
            .clone()
            .lazy()
            .select(select_cols.clone())
            .filter(filter_expr)
            .collect()
            .context("filter by partition")?;

        let summary_rate = standard_rate(filtered, column, window_duration)
            .with_context(|| format!("Standard rate for {key:?}"))?;

        sum_mean += summary_rate.mean;
        if summary_rate.mean > max_mean {
            max_mean = summary_rate.mean;
        }
        if summary_rate.mean < min_mean {
            min_mean = summary_rate.mean;
        }
    }

    // Compute the partitioned trend vectorially: group by (window, partition) to get
    // per-partition counts per window, then average across partitions per window.
    // This avoids a per-partition loop and produces a single chronological series
    // of mean-across-partitions counts per time window.
    let partition_cols: Vec<Expr> = partition_by.iter().map(|&s| col(s)).collect();
    let trend_frame = frame
        .lazy()
        .filter(col("time").is_not_null())
        .sort(
            ["time"],
            SortMultipleOptions::default().with_maintain_order(true),
        )
        .group_by_dynamic(
            col("time"),
            partition_cols,
            DynamicGroupOptions {
                every: Duration::parse(window_duration),
                period: Duration::parse(window_duration),
                offset: Duration::parse("0s"),
                ..Default::default()
            },
        )
        .agg([col(column).count().alias("count")])
        .group_by([col("time")])
        .agg([col("count").mean().alias("mean_count")])
        .sort(
            ["time"],
            SortMultipleOptions::default().with_maintain_order(true),
        )
        .collect()?;

    let trend: Vec<f64> = trend_frame
        .column("mean_count")?
        .f64()?
        .iter()
        .map(|v| round_to_n_dp(v.unwrap_or(0.0), 2))
        .collect();

    Ok(PartitionedRateStats {
        mean: round_to_n_dp(sum_mean / partition_count as f64, 2),
        max_mean: round_to_n_dp(max_mean, 2),
        min_mean: round_to_n_dp(if min_mean == f64::MAX { 0.0 } else { min_mean }, 2),
        trend,
        window_duration: window_duration.to_string(),
    })
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

    Ok((
        sorted_percentile(&sorted, 0.50),
        sorted_percentile(&sorted, 0.95),
        sorted_percentile(&sorted, 0.99),
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

/// Compute the delivery ratio: the fraction of sent items that were received by each reader.
///
/// `sent_count` is the total number of items sent (e.g. entries created).
/// `recv` is the `PartitionedCounterStats` for receivers — `total_count` is the aggregate
/// received count and `partition_count` is the number of reading agents.
///
/// The formula normalises the received total by both the number of items sent and the number
/// of receivers (each receiver should see every sent item). Clamped to `[0, 1]` to absorb
/// windowing imprecision. Returns `0.0` when there are no sent items or no receivers.
pub(crate) fn delivery_ratio(sent_count: u64, recv: &PartitionedCounterStats) -> f64 {
    if sent_count > 0 && recv.partition_count > 0 {
        round_to_n_dp(
            (recv.total_count as f64 / (sent_count as f64 * recv.partition_count as f64)).min(1.0),
            4,
        )
    } else {
        0.0
    }
}

/// Compute the difference between two counter DataFrames, summed for all metrics.
///
/// This assumes that the two counters will have data from different times
///
/// This function:
/// 1. Concatenates both DataFrames with counter1 values as positive and counter2 values as negative
/// 2. Groups by time and sums to get net change at each time point
/// 3. Computes cumulative sum to get running total at each time
/// 4. Computes statistics on this running total over time
///
/// # Arguments
/// * `counter1` - First counter DataFrame (e.g., startups)
/// * `counter2` - Second counter DataFrame (e.g., shutdowns)
/// * `value_column` - Column name in counter1 and counter2 containing the counter values
/// * `sum_partition_by_column` - Column name that sum values should be calculated for (e.g., agent identifiers)
/// * `window_duration` - Duration for windowed aggregation (e.g., "10s")
///
/// The value of `sum_partition_by_column` must convert to a &str.
pub(crate) fn running_conductors_stats(
    startups_counter: DataFrame,
    shutdowns_counter: DataFrame,
    agent_column: &str,
    window_duration: &str,
) -> anyhow::Result<GaugeStats> {
    // Construct a dataframe a full join of startups_counter and shutdowns_counter on the time field, then:
    // - Sort by agent_column, then time
    // - Forward-fill all null values in value with the same agent_column
    // - Forward-fill all null values in value_right with the same agent_column
    // - Set any remaining null values to 0.0 (i.e. any null values before any counter values are recorded)
    //
    // This gives us a startup and shutdown count for every agent, at every time that a data point was recorded.
    let agent_column_right = format!("{agent_column}_right");

    let counters_merged = startups_counter
        .clone()
        .lazy()
        .join(
            shutdowns_counter.clone().lazy(),
            [col("time")],
            [col("time")],
            JoinArgs::new(JoinType::Full),
        )
        .with_column(coalesce(&[col("time"), col("time_right")]).alias("time"))
        .with_column(
            coalesce(&[col(agent_column), col(agent_column_right.clone())]).alias(agent_column),
        )
        .drop(["time_right", agent_column_right.as_str()])
        .sort([agent_column, "time"], Default::default())
        .with_column(
            col("value_right")
                .fill_null_with_strategy(FillNullStrategy::Forward(None))
                .over([col(agent_column)])
                .fill_null(lit(0.0)),
        )
        .with_column(
            col("value")
                .fill_null_with_strategy(FillNullStrategy::Forward(None))
                .over([col(agent_column)])
                .fill_null(lit(0.0)),
        )
        .with_column((col("value") - col("value_right")).alias("running"))
        .collect()?;

    // Get unique agents as strings
    let unique_agents_series = counters_merged
        .clone()
        .column(agent_column)?
        .as_materialized_series()
        .unique()?;
    let unique_agents_strs = unique_agents_series
        .str()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    // Create a base dataframe with just time
    let mut result = counters_merged
        .clone()
        .lazy()
        .select([col("time")])
        .unique(None, UniqueKeepStrategy::First)
        .sort(["time"], Default::default())
        .collect()?;

    // Join each unqiue sum_partition_by_column data as a separate column
    for agent in unique_agents_strs.clone() {
        let agent_data = counters_merged
            .clone()
            .lazy()
            .filter(col(agent_column).eq(lit(agent)))
            .select([col("time"), col("running").alias(agent)])
            .collect()?;

        result = result
            .lazy()
            .join(
                agent_data.lazy(),
                [col("time")],
                [col("time")],
                JoinArgs::new(JoinType::Left),
            )
            .with_column(
                col(agent)
                    .fill_null_with_strategy(FillNullStrategy::Forward(None))
                    .fill_null(lit(0.0)),
            )
            .collect()?;
    }

    // Add sum column across all agent columns
    let sum_expr = unique_agents_strs
        .iter()
        .map(|agent| col(*agent))
        .reduce(|acc, c| acc + c)
        .unwrap_or(lit(0.0));

    // Calculate the delta between sum of startups and sum of shutdowns
    result = result
        .lazy()
        .with_column(sum_expr.alias("delta"))
        .drop(unique_agents_strs)
        .collect()?;

    let series = result.column("delta")?.as_materialized_series();

    // Compute statistics on the running total
    let mean = series.mean().context("Mean")?;
    let std = series.std(0).context("Std")?;

    // Percentile range
    let sorted = series.f64()?.sort(false);
    let p5 = sorted_percentile(&sorted, 0.05);
    let p95 = sorted_percentile(&sorted, 0.95);

    // Windowed trend
    let trend = result
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
        .agg([col("delta").mean().alias("mean")])
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
        trend: Float64Trend {
            trend,
            window_duration: window_duration.to_string(),
        },
    })
}
