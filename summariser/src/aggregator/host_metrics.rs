use anyhow::Context;
use polars::frame::{DataFrame, UniqueKeepStrategy};
use polars::prelude::*;
use std::collections::{BTreeMap, HashMap, HashSet};
use wind_tunnel_summary_model::RunSummary;

use crate::analyze::{
    column_mean, counter_stats, gauge_stats, gauge_stats_dp, growth_rate, percentiles,
    round_to_n_dp,
};
use crate::model::{
    AnomalyStatus, CounterStats, CpuMetrics, DiskMetrics, DiskSpace, DiskThroughput, GaugeStats,
    HostAnomalies, HostMetrics, MemMetrics, NetMetrics, PrimaryNetStats, ProcessMetrics,
    PsiMetrics, PsiResource, PsiStall, Severity, SystemLoadMetrics,
};
use crate::query::host_metrics::{
    self as host_metrics_query, CpuField, CpuFieldSet, DiskField, DiskFieldSet, DiskIoField,
    DiskIoFieldSet, HostMetricMeasurement, MemField, NetField, NetFieldSet, PressureField,
    PressureFieldSet, ProcstatField, ProcstatFieldSet, SystemField, SystemFieldSet,
};
use crate::query::query_host_metrics;

/// The host metrics aggregator takes care of collecting the [`HostMetrics`] and aggregate them accordingly.
///
/// It is basically a helper around the query and analyze modules for host metrics, summarizing the data
/// and providing a unified interface for accessing it.
pub struct HostMetricsAggregator<'a> {
    client: &'a influxdb::Client,
    summary: &'a RunSummary,
}

impl<'a> HostMetricsAggregator<'a> {
    /// Create a new host metrics aggregator.
    pub fn new(client: &'a influxdb::Client, summary: &'a RunSummary) -> Self {
        Self { client, summary }
    }
}

impl HostMetricsAggregator<'_> {
    /// Try to aggregate host metrics.
    ///
    /// It returns an [`Option`] of [`HostMetrics`]. If it fails to collect metrics it returns [`None`] and
    /// reports the error in the logs.
    pub async fn try_aggregate(&self) -> Option<HostMetrics> {
        match self.aggregate().await {
            Ok(metrics) => Some(metrics),
            Err(e) => {
                log::error!("Failed to aggregate host metrics: {e}");
                None
            }
        }
    }

    /// Aggregate all [`HostMetrics`] and return them.
    pub async fn aggregate(&self) -> anyhow::Result<HostMetrics> {
        log::debug!("Aggregating host metrics for run {}", self.summary.run_id);

        let cpu = self.aggregate_cpu_metrics().await?;
        let memory = self.aggregate_mem_metrics().await?;
        let network = self.aggregate_net_metrics().await?;
        let disk = self.aggregate_disk_metrics().await?;
        let system_load = self.aggregate_system_load().await?;
        let pressure = self
            .aggregate_psi_metrics()
            .await
            .inspect_err(|e| log::warn!("Failed to aggregate PSI metrics: {e}"))
            .ok();
        let process = self
            .aggregate_process_metrics()
            .await
            .inspect_err(|e| log::warn!("Failed to aggregate process metrics: {e}"))
            .ok();

        let anomalies = self.detect_anomalies(&cpu, &memory, &disk, &system_load);

        Ok(HostMetrics {
            cpu,
            memory,
            network,
            disk,
            system_load,
            pressure,
            holochain_process: process,
            anomalies,
        })
    }

    /// Aggregate [`CpuMetrics`].
    ///
    /// Queries user and system CPU raw data, joins them point-wise to compute
    /// total usage, then derives all stats from the total.
    async fn aggregate_cpu_metrics(&self) -> anyhow::Result<CpuMetrics> {
        log::debug!("Aggregating CPU metrics");

        // Two queries instead of three: get raw data for both components
        let cpu_data = query_host_metrics(
            self.client,
            self.summary,
            HostMetricMeasurement::Cpu(CpuFieldSet::Default),
        )
        .await?;

        // Create a new table called "total" with the sum of user and system columns
        let combined = cpu_data
            .lazy()
            .with_column(
                (col(CpuField::UsageUser.as_ref()) + col(CpuField::UsageSystem.as_ref()))
                    .alias("total"),
            )
            .collect()
            .context("Failed to compute total CPU usage")?;

        // Full stats on total CPU usage
        let total_usage = gauge_stats(combined.clone(), "total", "60s")?;

        // User/system means for context (where is the load coming from?)
        let usage_user_mean = column_mean(&combined, CpuField::UsageUser.as_ref());
        let usage_system_mean = column_mean(&combined, CpuField::UsageSystem.as_ref());

        // Percentiles on total usage (not just user)
        let (p50, p95, p99) = percentiles(&combined, "total").unwrap_or((0.0, 0.0, 0.0));

        // Time above 80% — computed per host so a single overloaded machine is not diluted
        // by the fleet size. Reports count of hosts that exceeded the threshold and
        // the mean time spent there for those hosts.
        let duration_secs = self.summary.run_duration.map(|d| d as f64).unwrap_or(1.0);
        let (hosts_above_80_percent, mean_time_above_80_percent_s) =
            if combined.column(CpuField::Host.as_ref()).is_ok() {
                let by_host = self
                    .aggregate_data_frame_by_tag(combined.clone(), CpuField::Host.as_ref())
                    .await?;
                let mut times_above: Vec<f64> = Vec::new();
                for host_frame in by_host.values() {
                    let n_total = host_frame.height();
                    if n_total == 0 {
                        continue;
                    }
                    let above = host_frame
                        .clone()
                        .lazy()
                        .select([col("total").gt(lit(80.0_f64)).sum()])
                        .collect()
                        .context("Failed to count CPU samples above 80% for host")?
                        .column("total")?
                        .u32()?
                        .get(0)
                        .unwrap_or(0);
                    if above > 0 {
                        times_above.push((above as f64 / n_total as f64) * duration_secs);
                    }
                }
                let count = times_above.len();
                let mean = if count > 0 {
                    round_to_n_dp(times_above.iter().sum::<f64>() / count as f64, 2)
                } else {
                    0.0
                };
                (count, mean)
            } else {
                // Fallback: no host column (old test data) — treat fleet as single host
                let n_total = combined.height();
                let above = combined
                    .clone()
                    .lazy()
                    .select([col("total").gt(lit(80.0_f64)).sum()])
                    .collect()
                    .context("Failed to count CPU samples above 80%")?
                    .column("total")?
                    .u32()?
                    .get(0)
                    .unwrap_or(0);
                let time_s = if n_total > 0 {
                    round_to_n_dp((above as f64 / n_total as f64) * duration_secs, 2)
                } else {
                    0.0
                };
                let count = if above > 0 { 1 } else { 0 };
                (count, if count > 0 { time_s } else { 0.0 })
            };

        Ok(CpuMetrics {
            total_usage,
            usage_user_mean: round_to_n_dp(usage_user_mean, 2),
            usage_system_mean: round_to_n_dp(usage_system_mean, 2),
            p50: round_to_n_dp(p50, 2),
            p95: round_to_n_dp(p95, 2),
            p99: round_to_n_dp(p99, 2),
            hosts_above_80_percent,
            mean_time_above_80_percent_s,
        })
    }

    /// Aggregate [`MemMetrics`].
    ///
    /// A single query fetches all memory fields, then derives gauge stats on the
    /// key columns and computes OOM risk, swap activity, and growth rate.
    async fn aggregate_mem_metrics(&self) -> anyhow::Result<MemMetrics> {
        log::debug!("Aggregating Memory metrics");

        let mem_data = query_host_metrics(
            self.client,
            self.summary,
            HostMetricMeasurement::Mem(host_metrics_query::MemFieldSet::Default),
        )
        .await?;

        // Primary gauge stats
        let used_percent = gauge_stats(mem_data.clone(), MemField::UsedPercent.as_ref(), "60s")?;
        let available_percent =
            gauge_stats(mem_data.clone(), MemField::AvailablePercent.as_ref(), "60s")?;

        // All three per-host metrics (growth rate, swap, max used %) are computed in one
        // pass over the by-host split. This avoids calling aggregate_data_frame_by_tag
        // multiple times and gives correct per-host values rather than fleet-wide averages.
        let duration_secs = self.summary.run_duration.map(|d| d as f64).unwrap_or(1.0);
        let (growth_rate_bytes, swap_used_percent, max_host_used_percent) = if mem_data
            .column(MemField::Host.as_ref())
            .is_ok()
        {
            let by_host = self
                .aggregate_data_frame_by_tag(mem_data.clone(), MemField::Host.as_ref())
                .await?;
            let mut max_growth = f64::NEG_INFINITY;
            let mut max_swap = f64::NEG_INFINITY;
            let mut max_used_pct = f64::NEG_INFINITY;
            for hf in by_host.values() {
                if let Ok(rate) = growth_rate(hf, MemField::Used.as_ref(), duration_secs) {
                    max_growth = max_growth.max(rate);
                }
                let swap_total = column_mean(hf, MemField::SwapTotal.as_ref());
                let swap_free = column_mean(hf, MemField::SwapFree.as_ref());
                if swap_total > 0.0 {
                    max_swap = max_swap.max(((swap_total - swap_free) / swap_total) * 100.0);
                }
                max_used_pct = max_used_pct.max(column_mean(hf, MemField::UsedPercent.as_ref()));
            }
            let growth = if max_growth.is_finite() {
                max_growth
            } else {
                0.0
            };
            let swap = if max_swap.is_finite() {
                round_to_n_dp(max_swap, 2)
            } else {
                0.0
            };
            let used_pct = if max_used_pct.is_finite() {
                round_to_n_dp(max_used_pct, 2)
            } else {
                0.0
            };
            (growth, swap, used_pct)
        } else {
            // Fallback for old test data without host column
            let growth =
                growth_rate(&mem_data, MemField::Used.as_ref(), duration_secs).unwrap_or(0.0);
            let swap_total = column_mean(&mem_data, MemField::SwapTotal.as_ref());
            let swap_free = column_mean(&mem_data, MemField::SwapFree.as_ref());
            let swap = if swap_total > 0.0 {
                round_to_n_dp(((swap_total - swap_free) / swap_total) * 100.0, 2)
            } else {
                0.0
            };
            let used_pct = round_to_n_dp(column_mean(&mem_data, MemField::UsedPercent.as_ref()), 2);
            (growth, swap, used_pct)
        };
        let growth_rate_mb = growth_rate_bytes / (1024.0 * 1024.0);

        Ok(MemMetrics {
            used_percent,
            available_percent,
            max_host_used_percent,
            swap_used_percent,
            growth_rate_mb_per_sec: round_to_n_dp(growth_rate_mb, 2),
        })
    }

    /// Aggregate the [`NetMetrics`] with host-aware primary interface detection.
    ///
    /// A single query fetches all net fields with host + interface tags.
    /// The "primary" interface per host is the one carrying the most bytes
    /// (recv + sent); counter derivatives and byte rates are computed per
    /// (host, interface) pair before aggregation, so multi-host values are correct.
    async fn aggregate_net_metrics(&self) -> anyhow::Result<NetMetrics> {
        log::debug!("Aggregating network metrics");

        let net_data = query_host_metrics(
            self.client,
            self.summary,
            HostMetricMeasurement::Net(NetFieldSet::Default),
        )
        .await?;

        let primary = self.detect_primary_interfaces(net_data).await?;

        Ok(NetMetrics { primary })
    }

    /// For each host, find the interface carrying the most total bytes
    /// (recv + sent), compute instantaneous byte-rate derivatives from the
    /// counter series, then aggregate all primary-interface rate samples
    /// through [`gauge_stats`] to produce mean / std / p5 / p95 / trend.
    async fn detect_primary_interfaces(
        &self,
        net_data: DataFrame,
    ) -> anyhow::Result<PrimaryNetStats> {
        // Discover unique hosts
        let hosts = {
            let c = net_data.column(NetField::Host.as_ref())?.str()?;
            c.iter()
                .flatten()
                .map(|s| s.to_string())
                .collect::<HashSet<String>>()
        };

        // Accumulate rate samples and counter deltas across primary interfaces
        let mut all_timestamps: Vec<i64> = Vec::new();
        let mut all_recv_rates: Vec<f64> = Vec::new();
        let mut all_sent_rates: Vec<f64> = Vec::new();
        let mut combined_recv_delta: u64 = 0;
        let mut combined_sent_delta: u64 = 0;
        let mut combined_duration = std::time::Duration::ZERO;

        for host in &hosts {
            // Filter to this host
            let host_frame = net_data
                .clone()
                .lazy()
                .filter(col(NetField::Host.as_ref()).eq(lit(host.as_str())))
                .collect()?;

            // Split by interface within this host
            let by_iface = self
                .aggregate_data_frame_by_tag(host_frame, NetField::Interface.as_ref())
                .await?;

            // Find the interface with the highest total bytes (recv + sent delta)
            let mut best: Option<(String, DataFrame, CounterStats, CounterStats)> = None;
            let mut best_total: u64 = 0;

            for (iface, frame) in by_iface {
                let recv = counter_stats(frame.clone(), NetField::BytesRecv.as_ref(), "60s")
                    .unwrap_or_default();
                let sent = counter_stats(frame.clone(), NetField::BytesSent.as_ref(), "60s")
                    .unwrap_or_default();
                let total = recv.count.saturating_add(sent.count);

                if total > best_total || best.is_none() {
                    best_total = total;
                    best = Some((iface, frame, recv, sent));
                }
            }

            if let Some((iface, frame, recv_cs, sent_cs)) = best {
                log::debug!("Primary interface for {host}: {iface} ({best_total} bytes)");

                // Accumulate counter deltas
                combined_recv_delta = combined_recv_delta.saturating_add(recv_cs.count);
                combined_sent_delta = combined_sent_delta.saturating_add(sent_cs.count);
                if recv_cs.measurement_duration > combined_duration {
                    combined_duration = recv_cs.measurement_duration;
                }

                // Compute instantaneous rates from counter derivatives
                Self::counter_derivative_rates(
                    &frame,
                    &mut all_timestamps,
                    &mut all_recv_rates,
                    &mut all_sent_rates,
                )?;
            }
        }

        // Combined counter stats across all primary interfaces
        let duration_secs = combined_duration.as_secs_f64();
        let bytes_recv = CounterStats {
            count: combined_recv_delta,
            measurement_duration: combined_duration,
            mean_rate_per_second: if duration_secs > 0.0 {
                round_to_n_dp(combined_recv_delta as f64 / duration_secs, 2)
            } else {
                0.0
            },
            ..Default::default()
        };
        let bytes_sent = CounterStats {
            count: combined_sent_delta,
            measurement_duration: combined_duration,
            mean_rate_per_second: if duration_secs > 0.0 {
                round_to_n_dp(combined_sent_delta as f64 / duration_secs, 2)
            } else {
                0.0
            },
            ..Default::default()
        };

        // Build a DataFrame of rate samples and feed through gauge_stats
        let (recv_rate, send_rate) = if all_timestamps.is_empty() {
            (GaugeStats::default(), GaugeStats::default())
        } else {
            let time_series = Series::new("time".into(), &all_timestamps)
                .cast(&DataType::Datetime(TimeUnit::Nanoseconds, None))?;
            let recv_series = Series::new("recv_rate".into(), &all_recv_rates);
            let sent_series = Series::new("send_rate".into(), &all_sent_rates);

            let rate_frame = DataFrame::new(vec![
                time_series.into_column(),
                recv_series.into_column(),
                sent_series.into_column(),
            ])?;

            let recv = gauge_stats(rate_frame.clone(), "recv_rate", "60s")?;
            let sent = gauge_stats(rate_frame, "send_rate", "60s")?;
            (recv, sent)
        };

        Ok(PrimaryNetStats {
            bytes_recv,
            bytes_sent,
            recv_rate,
            send_rate,
        })
    }

    /// Compute instantaneous byte rates from consecutive counter samples.
    ///
    /// For each consecutive pair of rows (sorted by time), computes
    /// `(value[i] − value[i−1]) / Δt` and appends the result to the
    /// provided vectors.  Counter resets (negative deltas) are skipped.
    fn counter_derivative_rates(
        frame: &DataFrame,
        timestamps: &mut Vec<i64>,
        recv_rates: &mut Vec<f64>,
        sent_rates: &mut Vec<f64>,
    ) -> anyhow::Result<()> {
        let recv_col = NetField::BytesRecv.as_ref();
        let sent_col = NetField::BytesSent.as_ref();

        let sorted = frame
            .clone()
            .lazy()
            .select([col("time"), col(recv_col), col(sent_col)])
            .filter(col("time").is_not_null())
            .filter(col(recv_col).is_not_null())
            .filter(col(sent_col).is_not_null())
            .sort(["time"], SortMultipleOptions::default())
            .collect()?;

        if sorted.height() < 2 {
            return Ok(());
        }

        let times = sorted
            .column("time")?
            .as_materialized_series()
            .datetime()
            .context("time as datetime")?;
        let recv = sorted
            .column(recv_col)?
            .as_materialized_series()
            .i64()
            .context("bytes_recv as i64")?;
        let sent = sorted
            .column(sent_col)?
            .as_materialized_series()
            .i64()
            .context("bytes_sent as i64")?;

        for i in 1..sorted.height() {
            let (Some(t_prev), Some(t_curr)) = (times.get(i - 1), times.get(i)) else {
                continue;
            };
            let dt_ns = t_curr - t_prev;
            if dt_ns <= 0 {
                continue;
            }
            let dt_secs = dt_ns as f64 / 1_000_000_000.0;

            let (Some(r_prev), Some(r_curr)) = (recv.get(i - 1), recv.get(i)) else {
                continue;
            };
            let (Some(s_prev), Some(s_curr)) = (sent.get(i - 1), sent.get(i)) else {
                continue;
            };

            let r_delta = r_curr - r_prev;
            let s_delta = s_curr - s_prev;

            // Skip counter resets
            if r_delta < 0 || s_delta < 0 {
                continue;
            }

            timestamps.push(t_curr);
            recv_rates.push(r_delta as f64 / dt_secs);
            sent_rates.push(s_delta as f64 / dt_secs);
        }

        Ok(())
    }

    /// Aggregate the [`DataFrame`] by tag.
    ///
    /// Given a [`DataFrame`] and a tag, it returns all the sub
    async fn aggregate_data_frame_by_tag(
        &self,
        data_frame: DataFrame,
        tag: &str,
    ) -> anyhow::Result<HashMap<String, DataFrame>> {
        let aggregators = data_frame
            .clone()
            .lazy()
            .select([col(tag)])
            .unique(Some(vec![tag.to_string()]), UniqueKeepStrategy::Any)
            .collect()?;

        let keys: HashSet<&str> = aggregators.column(tag)?.str()?.iter().flatten().collect();

        let mut aggregated = HashMap::with_capacity(keys.len());
        for key in keys {
            log::debug!("Aggregating for {tag}={key}");

            let filtered = data_frame
                .clone()
                .lazy()
                .select([col("*")])
                .filter(col(tag).eq(lit(key)))
                .collect()?;
            aggregated.insert(key.to_string(), filtered);
        }

        Ok(aggregated)
    }

    /// Aggregate disk metrics
    async fn aggregate_disk_metrics(&self) -> anyhow::Result<DiskMetrics> {
        log::debug!("Aggregating Disk metrics");

        // Query disk I/O — single query for both read and write, split by (host, device)
        let diskio_data = query_host_metrics(
            self.client,
            self.summary,
            HostMetricMeasurement::DiskIo(DiskIoFieldSet::Default),
        )
        .await?;

        log::debug!("diskio: {} rows", diskio_data.height());
        let mut total_read = 0.0;
        let mut total_write = 0.0;

        // If host column is present, split by (host, device) to avoid mixing counters
        // across hosts. If absent (old test data loaded via migration fallback), fall
        // back to splitting by device name only — equivalent to single-host behaviour.
        if diskio_data.column(DiskIoField::Host.as_ref()).is_ok() {
            let by_host = self
                .aggregate_data_frame_by_tag(diskio_data, DiskIoField::Host.as_ref())
                .await?;
            log::debug!("diskio: {} hosts", by_host.len());
            for (_host, host_frame) in by_host {
                let by_device = self
                    .aggregate_data_frame_by_tag(host_frame, DiskIoField::Name.as_ref())
                    .await?;
                for (dev, f) in by_device {
                    let read = counter_stats(f.clone(), DiskIoField::ReadBytes.as_ref(), "60s")
                        .unwrap_or_default();
                    let write = counter_stats(f, DiskIoField::WriteBytes.as_ref(), "60s")
                        .unwrap_or_default();
                    log::debug!(
                        "  {_host}/{dev}: read_rate={} write_rate={}",
                        read.mean_rate_per_second,
                        write.mean_rate_per_second
                    );
                    total_read += read.mean_rate_per_second;
                    total_write += write.mean_rate_per_second;
                }
            }
        } else {
            // Fallback: old test data without host column — split by device only
            log::debug!("diskio: no host column, falling back to device-only split");
            let by_device = self
                .aggregate_data_frame_by_tag(diskio_data, DiskIoField::Name.as_ref())
                .await?;
            log::debug!("diskio: {} devices", by_device.len());
            for (dev, f) in by_device {
                let read = counter_stats(f.clone(), DiskIoField::ReadBytes.as_ref(), "60s")
                    .unwrap_or_default();
                let write =
                    counter_stats(f, DiskIoField::WriteBytes.as_ref(), "60s").unwrap_or_default();
                log::debug!(
                    "  {dev}: read_rate={} write_rate={}",
                    read.mean_rate_per_second,
                    write.mean_rate_per_second
                );
                total_read += read.mean_rate_per_second;
                total_write += write.mean_rate_per_second;
            }
        }

        let io_throughput_mbps = DiskThroughput {
            read: round_to_n_dp(total_read / (1024.0 * 1024.0), 2),
            write: round_to_n_dp(total_write / (1024.0 * 1024.0), 2),
        };

        // Query disk space — group by (host, path) so each host's used_percent is
        // evaluated independently. Per path, count how many hosts exceed 90%.
        let disk_space_result = query_host_metrics(
            self.client,
            self.summary,
            HostMetricMeasurement::Disk(DiskFieldSet::Default),
        )
        .await;

        let space_utilization: BTreeMap<String, DiskSpace> = match disk_space_result {
            Ok(data) => {
                let has_host = data.column(DiskField::Host.as_ref()).is_ok();
                // Group by (host, path) if host is available; fall back to path only for
                // old test data captured before the host column was added.
                let group_cols: Vec<Expr> = if has_host {
                    vec![col(DiskField::Host.as_ref()), col(DiskField::Path.as_ref())]
                } else {
                    vec![col(DiskField::Path.as_ref())]
                };
                let per_group = data
                    .lazy()
                    .group_by(group_cols)
                    .agg([col(DiskField::UsedPercent.as_ref()).mean()])
                    .collect()?;

                let path_ca = per_group
                    .column(DiskField::Path.as_ref())?
                    .str()?
                    .to_owned();
                let used_ca = per_group
                    .column(DiskField::UsedPercent.as_ref())?
                    .f64()?
                    .to_owned();

                // Accumulate (hosts_at_risk, host_count) per path
                let mut path_stats: BTreeMap<String, (usize, usize)> = BTreeMap::new();
                for i in 0..per_group.height() {
                    let path = path_ca.get(i).unwrap_or("");
                    if path.starts_with("/sys/") || path.starts_with("/var/") {
                        continue;
                    }
                    let used_pct = used_ca.get(i).unwrap_or(0.0);
                    let entry = path_stats.entry(path.to_string()).or_insert((0, 0));
                    entry.1 += 1; // host_count
                    if used_pct > 90.0 {
                        entry.0 += 1; // hosts_at_risk
                    }
                }
                path_stats
                    .into_iter()
                    .map(|(path, (hosts_at_risk, host_count))| {
                        (
                            path,
                            DiskSpace {
                                hosts_at_risk,
                                host_count,
                            },
                        )
                    })
                    .collect()
            }
            Err(_) => {
                log::debug!("No disk space data available");
                BTreeMap::new()
            }
        };

        Ok(DiskMetrics {
            io_throughput_mbps,
            space_utilization,
        })
    }

    /// Aggregate system load metrics
    async fn aggregate_system_load(&self) -> anyhow::Result<SystemLoadMetrics> {
        log::debug!("Aggregating System load metrics");

        let system_data = query_host_metrics(
            self.client,
            self.summary,
            HostMetricMeasurement::System(SystemFieldSet::Default),
        )
        .await?;

        // Overall stats across all hosts
        let l1 = gauge_stats(system_data.clone(), SystemField::Load1.as_ref(), "60s")
            .unwrap_or_default();
        let l5 = gauge_stats(system_data.clone(), SystemField::Load5.as_ref(), "60s")
            .unwrap_or_default();
        let l15 = gauge_stats(system_data.clone(), SystemField::Load15.as_ref(), "60s")
            .unwrap_or_default();

        let load_1min = round_to_n_dp(l1.mean, 2);
        let load_5min = round_to_n_dp(l5.mean, 2);
        let load_15min = round_to_n_dp(l15.mean, 2);

        // Per-host overload detection: load5 / n_cpus > 1.0
        // Group by host in one pass: mean load5 (varies over time) + first ncpus (constant).
        let per_host = system_data
            .lazy()
            .group_by([col(SystemField::Host.as_ref())])
            .agg([
                col(SystemField::Load5.as_ref()).mean(),
                col(SystemField::NCpus.as_ref()).first(),
            ])
            .collect()?;
        let host_count = per_host.height();
        let mut overloaded_count = 0usize;

        let host_col = per_host.column(SystemField::Host.as_ref())?.str()?;
        let load5_col = per_host.column(SystemField::Load5.as_ref())?.f64()?;
        let ncpus_series = per_host
            .column(SystemField::NCpus.as_ref())?
            .cast(&DataType::Float64)?;
        let ncpus_col = ncpus_series.f64()?;

        for i in 0..host_count {
            let host = host_col.get(i).unwrap_or("unknown");
            let host_load5 = load5_col.get(i).unwrap_or(0.0);
            let host_ncpus = ncpus_col.get(i).unwrap_or(0.0);
            let normalized = if host_ncpus > 0.0 {
                host_load5 / host_ncpus
            } else {
                0.0
            };
            if normalized > 1.0 {
                log::debug!("{host}: load5/ncpus = {normalized:.2} (overloaded)");
                overloaded_count += 1;
            }
        }

        let overloaded_percent = if host_count > 0 {
            round_to_n_dp(overloaded_count as f64 / host_count as f64 * 100.0, 2)
        } else {
            0.0
        };

        Ok(SystemLoadMetrics {
            load_1min,
            load_5min,
            load_15min,
            host_count,
            overloaded_percent,
        })
    }

    /// Aggregate Linux Pressure Stall Information (PSI) metrics.
    ///
    /// Queries each resource/type combination separately and assembles
    /// a [`PsiMetrics`] with cpu (some only), memory (some + full),
    /// and io (some + full). Returns `None` if no PSI data is available.
    async fn aggregate_psi_metrics(&self) -> anyhow::Result<PsiMetrics> {
        log::debug!("Aggregating PSI metrics");

        // Helper: query one pressure variant and build a PsiStall
        let query_stall = |field_set: PressureFieldSet| async move {
            let frame = query_host_metrics(
                self.client,
                self.summary,
                HostMetricMeasurement::Pressure(field_set),
            )
            .await?;

            if frame.height() < 1 {
                return Err(anyhow::anyhow!("No data for {field_set:?}"));
            }

            let avg10 = gauge_stats_dp(frame.clone(), PressureField::Avg10.as_ref(), "60s", 4)
                .unwrap_or_default();
            let avg60_mean = column_mean(&frame, PressureField::Avg60.as_ref());
            let avg300_mean = column_mean(&frame, PressureField::Avg300.as_ref());
            Ok(PsiStall {
                avg10,
                avg60_mean: round_to_n_dp(avg60_mean, 4),
                avg300_mean: round_to_n_dp(avg300_mean, 4),
            })
        };

        // CPU only has "some"
        let cpu = query_stall(PressureFieldSet::CpuSome).await?;

        // Memory and IO have both "some" and "full"
        let mem_some = query_stall(PressureFieldSet::MemSome).await?;
        let mem_full = query_stall(PressureFieldSet::MemFull).await?;
        let io_some = query_stall(PressureFieldSet::IoSome).await?;
        let io_full = query_stall(PressureFieldSet::IoFull).await?;

        Ok(PsiMetrics {
            cpu,
            memory: PsiResource {
                some: mem_some,
                full: mem_full,
            },
            io: PsiResource {
                some: io_some,
                full: io_full,
            },
        })
    }

    /// Aggregate process metrics from procstat.
    ///
    /// Queries procstat data (filtered by `pattern = "holochain"`), normalizes
    /// `cpu_usage` by the number of CPU cores on each host (procstat reports
    /// 100% per core, so values are unbounded on multi-core machines), then
    /// computes gauge stats on CPU, memory, threads, and FDs.
    /// Returns `Err` if no procstat data is found.
    async fn aggregate_process_metrics(&self) -> anyhow::Result<ProcessMetrics> {
        log::debug!("Aggregating process metrics");

        let frame = query_host_metrics(
            self.client,
            self.summary,
            HostMetricMeasurement::Procstat(ProcstatFieldSet::Default {
                pattern: "holochain".to_string(),
            }),
        )
        .await?;

        if frame.height() < 1 {
            log::debug!("No procstat data available");
            return Err(anyhow::anyhow!("No procstat data"));
        }

        let system_data = query_host_metrics(
            self.client,
            self.summary,
            HostMetricMeasurement::System(SystemFieldSet::Default),
        )
        .await?;

        // procstat reports cpu_usage where 100% = one core, so on a 16-core
        // machine a fully-loaded process shows 1600%.  Normalize per host so
        // values are comparable across machines with different core counts.
        //
        // ncpus is constant per host — take first non-null value per host in one vectorized pass.
        let ncpus_df = system_data
            .lazy()
            .group_by([col(SystemField::Host.as_ref())])
            .agg([col(SystemField::NCpus.as_ref()).first()])
            .collect()?;
        let ncpus_hosts = ncpus_df.column(SystemField::Host.as_ref())?.str()?;
        let ncpus_series = ncpus_df
            .column(SystemField::NCpus.as_ref())?
            .cast(&DataType::Float64)?;
        let ncpus_vals = ncpus_series.f64()?;
        let ncpus_by_host: HashMap<String, f64> = ncpus_hosts
            .into_iter()
            .zip(ncpus_vals.into_iter())
            .filter_map(|(h, n)| Some((h?.to_string(), n?)))
            .collect();

        // Old test-data captures lack the `host` tag column (it wasn't in the SELECT).
        // Infer a single host from ncpus_by_host so normalisation still works.
        let frame = if frame.column(ProcstatField::Host.as_ref()).is_err() {
            let inferred = ncpus_by_host.keys().next().cloned().unwrap_or_default();
            log::debug!("Procstat frame missing 'host' column; treating all rows as '{inferred}'");
            frame
                .lazy()
                .with_column(lit(inferred).alias(ProcstatField::Host.as_ref()))
                .collect()?
        } else {
            frame
        };

        let by_host = self
            .aggregate_data_frame_by_tag(frame, ProcstatField::Host.as_ref())
            .await?;

        let normalized: Vec<LazyFrame> = by_host
            .into_iter()
            .map(|(host, host_frame)| {
                let ncpus = ncpus_by_host.get(&host).copied().unwrap_or(1.0).max(1.0);
                log::debug!("Normalizing cpu_usage for host {host} by {ncpus} cores");
                host_frame.lazy().with_column(
                    (col(ProcstatField::CpuUsage.as_ref()) / lit(ncpus))
                        .alias(ProcstatField::CpuUsage.as_ref()),
                )
            })
            .collect();

        let frame = concat(normalized, UnionArgs::default())?.collect()?;

        let cpu_usage =
            gauge_stats(frame.clone(), ProcstatField::CpuUsage.as_ref(), "60s").unwrap_or_default();
        let memory_pss = gauge_stats(frame.clone(), ProcstatField::MemoryPss.as_ref(), "60s")
            .unwrap_or_default();
        let num_threads = gauge_stats(frame.clone(), ProcstatField::NumThreads.as_ref(), "60s")
            .unwrap_or_default();
        let num_fds = gauge_stats(frame, ProcstatField::NumFds.as_ref(), "60s").unwrap_or_default();

        Ok(ProcessMetrics {
            cpu_usage,
            memory_pss,
            num_threads,
            num_fds,
        })
    }

    /// Detect anomalies across host metrics.
    fn detect_anomalies(
        &self,
        cpu: &CpuMetrics,
        memory: &MemMetrics,
        disk: &DiskMetrics,
        system_load: &SystemLoadMetrics,
    ) -> HostAnomalies {
        // CPU spike detection
        let cpu_spike = if cpu.p99 > 90.0 {
            AnomalyStatus::Detected {
                severity: Severity::Warning,
                description: format!("CPU p99 reached {:.1}%", cpu.p99),
            }
        } else {
            AnomalyStatus::NotDetected
        };

        // Memory leak detection
        let memory_leak = if memory.growth_rate_mb_per_sec > 1.0 {
            AnomalyStatus::Detected {
                severity: Severity::Warning,
                description: format!(
                    "Memory growing at {:.2} MB/s",
                    memory.growth_rate_mb_per_sec
                ),
            }
        } else {
            AnomalyStatus::NotDetected
        };

        // Disk full detection — any mount point where at least one host is above 90%
        let disk_full = disk
            .space_utilization
            .iter()
            .filter(|(_, space)| space.hosts_at_risk > 0)
            .max_by_key(|(_, space)| space.hosts_at_risk)
            .map(|(mount, space)| AnomalyStatus::Detected {
                severity: Severity::Critical,
                description: format!(
                    "Disk {}: {}/{} hosts above 90% capacity",
                    mount, space.hosts_at_risk, space.host_count
                ),
            })
            .unwrap_or(AnomalyStatus::NotDetected);

        // Swap thrashing detection
        let swap_thrashing = if memory.swap_used_percent > 20.0 {
            AnomalyStatus::Detected {
                severity: Severity::Critical,
                description: format!(
                    "Heavy swap usage ({:.1}% swap used)",
                    memory.swap_used_percent
                ),
            }
        } else if memory.swap_used_percent > 10.0 {
            AnomalyStatus::Detected {
                severity: Severity::Warning,
                description: format!(
                    "Moderate swap usage ({:.1}% swap used)",
                    memory.swap_used_percent
                ),
            }
        } else {
            AnomalyStatus::NotDetected
        };

        // System overload detection (any host with load5/ncpus > 1.0)
        let system_overload = if system_load.overloaded_percent > 0.0 {
            AnomalyStatus::Detected {
                severity: Severity::Warning,
                description: format!(
                    "{:.0}% of hosts overloaded (load5/ncpus > 1.0)",
                    system_load.overloaded_percent
                ),
            }
        } else {
            AnomalyStatus::NotDetected
        };

        HostAnomalies {
            cpu_spike,
            memory_leak,
            disk_full,
            swap_thrashing,
            system_overload,
        }
    }
}
