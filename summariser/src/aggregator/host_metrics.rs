use anyhow::Context;
use polars::frame::{DataFrame, UniqueKeepStrategy};
use polars::prelude::*;
use std::collections::{BTreeMap, HashMap, HashSet};
use wind_tunnel_summary_model::RunSummary;

use crate::analyze::{
    column_mean, counter_stats, gauge_stats, growth_rate, percentiles, round_to_n_dp,
};
use crate::model::{
    AnomalyStatus, CounterStats, CpuMetrics, DiskMetrics, DiskSpace, DiskThroughput, GaugeStats,
    HostAnomalies, HostMetrics, MemMetrics, NetMetrics, PrimaryNetStats, ProcessMetrics,
    PsiMetrics, PsiResource, PsiStall, Severity, SystemLoadMetrics,
};
use crate::query::host_metrics::{
    self as host_metrics_query, CpuField, CpuFieldSet, DiskField, DiskFieldSet, DiskIoField,
    DiskIoFieldSet, HostMetricMeasurement, InfluxSourced, MemField, NetField, NetFieldSet,
    PressureField, PressureFieldSet, ProcstatField, ProcstatFieldSet, SystemField, SystemFieldSet,
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
        let psi = self.aggregate_psi_metrics().await;
        let process = self.aggregate_process_metrics().await;

        let anomalies = self.detect_anomalies(&cpu, &memory, &disk, &system_load);

        Ok(HostMetrics {
            cpu,
            memory,
            network,
            disk,
            system_load,
            pressure: psi,
            process,
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

        // Time above 80% - computed as the percentage of samples above 80% multiplied by the run duration
        let above_count = combined
            .clone()
            .lazy()
            .select([col("total").gt(lit(80.0)).sum()])
            .collect()
            .context("Failed to count samples above 80%")?
            .column("total")?
            .u32()?
            .get(0)
            .unwrap_or(0);
        let total_samples = combined.height();
        let duration_secs = self.summary.run_duration.map(|d| d as f64).unwrap_or(1.0);
        let time_above_80_percent_s = if total_samples > 0 {
            (above_count as f64 / total_samples as f64) * duration_secs
        } else {
            0.0
        };

        Ok(CpuMetrics {
            total_usage,
            usage_user_mean: round_to_n_dp(usage_user_mean, 2),
            usage_system_mean: round_to_n_dp(usage_system_mean, 2),
            p50: round_to_n_dp(p50, 2),
            p95: round_to_n_dp(p95, 2),
            p99: round_to_n_dp(p99, 2),
            time_above_80_percent_s: round_to_n_dp(time_above_80_percent_s, 2),
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

        // Detect swap activity
        let swap_total_mean = column_mean(&mem_data, MemField::SwapTotal.as_ref());
        let swap_free_mean = column_mean(&mem_data, MemField::SwapFree.as_ref());
        let swap_used_pct = if swap_total_mean > 0.0 {
            ((swap_total_mean - swap_free_mean) / swap_total_mean) * 100.0
        } else {
            0.0
        };
        let swap_used_percent = round_to_n_dp(swap_used_pct, 2);

        // Memory growth rate (leak detection)
        let duration_secs = self.summary.run_duration.map(|d| d as f64).unwrap_or(1.0);
        let growth_rate_bytes =
            growth_rate(&mem_data, MemField::Used.as_ref(), duration_secs).unwrap_or(0.0);
        let growth_rate_mb = growth_rate_bytes / (1024.0 * 1024.0);

        Ok(MemMetrics {
            used_percent,
            available_percent,
            used_bytes_mean: round_to_n_dp(column_mean(&mem_data, MemField::Used.as_ref()), 2),
            swap_used_percent,
            growth_rate_mb_per_sec: round_to_n_dp(growth_rate_mb, 2),
        })
    }

    /// Aggregate the [`NetMetrics`] by interface, with host-aware primary
    /// interface detection.
    ///
    /// A single query fetches all net fields with host + interface tags.
    /// Per-interface stats are computed by splitting on the interface tag.
    /// The "primary" interface per host is the one carrying the most bytes
    /// (recv + sent); throughput is then aggregated across primaries.
    async fn aggregate_net_metrics(&self) -> anyhow::Result<NetMetrics> {
        log::debug!("Aggregating network metrics");

        let net_data = query_host_metrics(
            self.client,
            self.summary,
            HostMetricMeasurement::Net(NetFieldSet::Default),
        )
        .await?;

        // --- Per-interface stats (split by interface tag) ---
        let by_interface = self
            .aggregate_data_frame_by_tag(net_data.clone(), NetField::Interface.as_ref())
            .await?;

        let mut bytes_recv = BTreeMap::new();
        let mut bytes_sent = BTreeMap::new();
        let mut packets_recv = BTreeMap::new();
        let mut packets_sent = BTreeMap::new();

        for (iface, frame) in &by_interface {
            bytes_recv.insert(
                iface.clone(),
                counter_stats(frame.clone(), NetField::BytesRecv.as_ref()).unwrap_or_default(),
            );
            bytes_sent.insert(
                iface.clone(),
                counter_stats(frame.clone(), NetField::BytesSent.as_ref()).unwrap_or_default(),
            );
            packets_recv.insert(
                iface.clone(),
                counter_stats(frame.clone(), NetField::PacketsRecv.as_ref()).unwrap_or_default(),
            );
            packets_sent.insert(
                iface.clone(),
                counter_stats(frame.clone(), NetField::PacketsSent.as_ref()).unwrap_or_default(),
            );
        }

        // --- Primary interface detection (per host) ---
        let primary = self.detect_primary_interfaces(net_data).await?;

        Ok(NetMetrics {
            bytes_recv,
            bytes_sent,
            packets_recv,
            packets_sent,
            primary,
        })
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
                let recv =
                    counter_stats(frame.clone(), NetField::BytesRecv.as_ref()).unwrap_or_default();
                let sent =
                    counter_stats(frame.clone(), NetField::BytesSent.as_ref()).unwrap_or_default();
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
            rate_per_second: if duration_secs > 0.0 {
                round_to_n_dp(combined_recv_delta as f64 / duration_secs, 2)
            } else {
                0.0
            },
        };
        let bytes_sent = CounterStats {
            count: combined_sent_delta,
            measurement_duration: combined_duration,
            rate_per_second: if duration_secs > 0.0 {
                round_to_n_dp(combined_sent_delta as f64 / duration_secs, 2)
            } else {
                0.0
            },
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

    /// Query the [`DataFrame`] for the given [`HostMetricMeasurement`] and aggregate them by `tag` into a [`HashMap`] where the key
    /// is the tag value and the analyzed metrics as `T` stats.
    ///
    /// Stats are collected by a function `collect_stats` which takes (`data_frame`, `field_column`)
    /// and returns a `anyhow::Result<T>` stats.
    async fn query_and_aggregate<F, T>(
        &self,
        measurement: HostMetricMeasurement,
        tag: &str,
        collect_stats: F,
    ) -> anyhow::Result<BTreeMap<String, T>>
    where
        F: Fn(DataFrame, &str) -> anyhow::Result<T>,
        T: Default,
    {
        let data = query_host_metrics(self.client, self.summary, measurement).await?;

        Ok(self
            .aggregate_data_frame_by_tag(data, tag)
            .await?
            .into_iter()
            .map(|(tag_value, frame)| {
                // Use the first non-tag column as the data field
                let field_column = measurement
                    .select()
                    .iter()
                    .find(|&&c| c != tag)
                    .expect("measurement must have a field column");
                let stats = match collect_stats(frame, field_column) {
                    Ok(stats) => stats,
                    Err(e) => {
                        log::warn!(
                            "Failed to collect stats for {measurement:?} ({tag}={tag_value}): {e}"
                        );
                        T::default()
                    }
                };

                (tag_value, stats)
            })
            .collect::<BTreeMap<_, _>>())
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

        // Query disk I/O — single query for both read and write, split by device
        let diskio_data = query_host_metrics(
            self.client,
            self.summary,
            HostMetricMeasurement::DiskIo(DiskIoFieldSet::Default),
        )
        .await;

        let (read_rate, write_rate) = match diskio_data {
            Ok(df) => {
                log::debug!("diskio: {} rows", df.height());
                let by_device = self
                    .aggregate_data_frame_by_tag(df, DiskIoField::Name.as_ref())
                    .await?;
                log::debug!("diskio: {} devices", by_device.len());
                let mut total_read = 0.0;
                let mut total_write = 0.0;
                for (dev, f) in by_device {
                    let read = counter_stats(f.clone(), DiskIoField::ReadBytes.as_ref())
                        .unwrap_or_default();
                    let write =
                        counter_stats(f, DiskIoField::WriteBytes.as_ref()).unwrap_or_default();
                    log::debug!(
                        "  {dev}: read_rate={} write_rate={}",
                        read.rate_per_second,
                        write.rate_per_second
                    );
                    total_read += read.rate_per_second;
                    total_write += write.rate_per_second;
                }
                (total_read, total_write)
            }
            Err(e) => {
                log::debug!("No disk I/O data available: {e}");
                (0.0, 0.0)
            }
        };

        let io_throughput_mbps = DiskThroughput {
            read: round_to_n_dp(read_rate / (1024.0 * 1024.0), 2),
            write: round_to_n_dp(write_rate / (1024.0 * 1024.0), 2),
        };

        // Query disk space utilization
        let space_utilization_result = self
            .query_and_aggregate(
                HostMetricMeasurement::Disk(DiskFieldSet::Default),
                DiskField::Path.as_ref(),
                |df, col| gauge_stats(df, col, "60s"),
            )
            .await;

        let space_utilization = match space_utilization_result {
            Ok(data) => data
                .into_iter()
                .filter(|(mount, _)| !mount.starts_with("/sys/") && !mount.starts_with("/var/"))
                .map(|(mount, stats)| {
                    (
                        mount,
                        DiskSpace {
                            used_percent: round_to_n_dp(stats.mean, 2),
                        },
                    )
                })
                .collect(),
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
        .await;

        let data = match system_data {
            Ok(data) => data,
            Err(_) => {
                log::debug!("No system load data available");
                return Ok(SystemLoadMetrics {
                    load_1min: 0.0,
                    load_5min: 0.0,
                    load_15min: 0.0,
                    host_count: 0,
                    overloaded_percent: 0.0,
                });
            }
        };

        // Overall stats across all hosts
        let l1 = gauge_stats(data.clone(), SystemField::Load1.as_ref(), "60s").unwrap_or_default();
        let l5 = gauge_stats(data.clone(), SystemField::Load5.as_ref(), "60s").unwrap_or_default();
        let l15 =
            gauge_stats(data.clone(), SystemField::Load15.as_ref(), "60s").unwrap_or_default();

        let load_1min = round_to_n_dp(l1.mean, 2);
        let load_5min = round_to_n_dp(l5.mean, 2);
        let load_15min = round_to_n_dp(l15.mean, 2);

        // Per-host overload detection: load5 / n_cpus > 1.0
        let by_host = self
            .aggregate_data_frame_by_tag(data, SystemField::Host.as_ref())
            .await?;
        let host_count = by_host.len();
        let mut overloaded_count = 0usize;

        for (host, frame) in &by_host {
            let host_load5 = column_mean(frame, SystemField::Load5.as_ref());
            let host_ncpus = column_mean(frame, SystemField::NCpus.as_ref());
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
    async fn aggregate_psi_metrics(&self) -> Option<PsiMetrics> {
        log::debug!("Aggregating PSI metrics");

        // Helper: query one pressure variant and build a PsiStall
        let query_stall = |field_set: PressureFieldSet| async move {
            match query_host_metrics(
                self.client,
                self.summary,
                HostMetricMeasurement::Pressure(field_set),
            )
            .await
            {
                Ok(frame) if frame.height() > 0 => {
                    let avg10 = gauge_stats(frame.clone(), PressureField::Avg10.as_ref(), "60s")
                        .unwrap_or_default();
                    let avg60_mean = column_mean(&frame, PressureField::Avg60.as_ref());
                    let avg300_mean = column_mean(&frame, PressureField::Avg300.as_ref());
                    Some(PsiStall {
                        avg10,
                        avg60_mean: round_to_n_dp(avg60_mean, 4),
                        avg300_mean: round_to_n_dp(avg300_mean, 4),
                    })
                }
                _ => None,
            }
        };

        // CPU only has "some"
        let cpu = query_stall(PressureFieldSet::CpuSome).await?;

        // Memory and IO have both "some" and "full"
        let mem_some = query_stall(PressureFieldSet::MemSome).await;
        let mem_full = query_stall(PressureFieldSet::MemFull).await;
        let io_some = query_stall(PressureFieldSet::IoSome).await;
        let io_full = query_stall(PressureFieldSet::IoFull).await;

        // Default stall for missing variants
        let default_stall = || PsiStall {
            avg10: GaugeStats::default(),
            avg60_mean: 0.0,
            avg300_mean: 0.0,
        };

        Some(PsiMetrics {
            cpu,
            memory: PsiResource {
                some: mem_some.unwrap_or_else(default_stall),
                full: mem_full.unwrap_or_else(default_stall),
            },
            io: PsiResource {
                some: io_some.unwrap_or_else(default_stall),
                full: io_full.unwrap_or_else(default_stall),
            },
        })
    }

    /// Aggregate Holochain process metrics from procstat.
    ///
    /// Queries procstat data (filtered by `pattern = "holochain"`), computes
    /// gauge stats on CPU, memory, threads, and FDs, and derives context-switch
    /// rates from the counter fields. Returns `None` if no procstat data is found.
    async fn aggregate_process_metrics(&self) -> Option<ProcessMetrics> {
        log::debug!("Aggregating Holochain process metrics");

        let data = match query_host_metrics(
            self.client,
            self.summary,
            HostMetricMeasurement::Procstat(ProcstatFieldSet::Default),
        )
        .await
        {
            Ok(df) if df.height() > 0 => df,
            Ok(_) => {
                log::debug!("No procstat data available (empty)");
                return None;
            }
            Err(e) => {
                log::debug!("No procstat data available: {e}");
                return None;
            }
        };

        let cpu_usage =
            gauge_stats(data.clone(), ProcstatField::CpuUsage.as_ref(), "60s").unwrap_or_default();
        let memory_pss =
            gauge_stats(data.clone(), ProcstatField::MemoryPss.as_ref(), "60s").unwrap_or_default();
        let num_threads = gauge_stats(data.clone(), ProcstatField::NumThreads.as_ref(), "60s")
            .unwrap_or_default();
        let num_fds = gauge_stats(data, ProcstatField::NumFds.as_ref(), "60s").unwrap_or_default();

        Some(ProcessMetrics {
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

        // Disk full detection — report the worst mount point
        let disk_full = disk
            .space_utilization
            .iter()
            .filter(|(_, space)| space.used_percent > 90.0)
            .max_by(|a, b| a.1.used_percent.partial_cmp(&b.1.used_percent).unwrap())
            .map(|(mount, space)| AnomalyStatus::Detected {
                severity: Severity::Critical,
                description: format!("Disk {} at {:.1}% capacity", mount, space.used_percent),
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
        } else if memory.swap_used_percent > 5.0 {
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
