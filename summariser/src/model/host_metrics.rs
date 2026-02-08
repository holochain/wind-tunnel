use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::model::{CounterStats, GaugeStats};

/// Host metrics model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HostMetrics {
    /// [`CpuMetrics`] for the host; load is averaged among CPU cores
    pub cpu: CpuMetrics,
    /// [`MemMetrics`] for the host
    pub memory: MemMetrics,
    /// [`NetMetrics`] aggregated by the network interface name
    pub network: NetMetrics,
    /// [`DiskMetrics`] for the host
    pub disk: DiskMetrics,
    /// [`SystemLoadMetrics`] for the host
    pub system_load: SystemLoadMetrics,
    /// Linux Pressure Stall Information (PSI) metrics, when available.
    ///
    /// Only populated when telegraf collects PSI data via `inputs.kernel` with `collect = ["psi"]`.
    pub pressure: Option<PsiMetrics>,
    /// Holochain process metrics from procstat, when available.
    ///
    /// Only populated when telegraf collects procstat data with `pattern = "holochain"`.
    pub process: Option<ProcessMetrics>,
    /// [`HostAnomalies`] detected during the run
    pub anomalies: HostAnomalies,
}

/// CPU metrics model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CpuMetrics {
    /// Full gauge stats on total CPU usage (user + system, computed point-wise)
    pub total_usage: GaugeStats,
    /// Mean user-space CPU usage (%)
    pub usage_user_mean: f64,
    /// Mean kernel/system CPU usage (%)
    pub usage_system_mean: f64,
    /// 50th percentile of total CPU usage
    pub p50: f64,
    /// 95th percentile of total CPU usage
    pub p95: f64,
    /// 99th percentile of total CPU usage
    pub p99: f64,
    /// Estimated seconds spent above 80% total CPU usage
    pub time_above_80_percent_s: f64,
}

/// RAM metrics model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemMetrics {
    /// Full gauge stats on memory used percent — the primary memory utilisation metric
    pub used_percent: GaugeStats,
    /// Full gauge stats on memory available percent — headroom metric
    pub available_percent: GaugeStats,
    /// Mean used memory in bytes
    pub used_bytes_mean: f64,
    /// Average swap used percent across hosts (0–100)
    pub swap_used_percent: f64,
    /// Memory growth rate in MB/s (positive = growing, for leak detection)
    pub growth_rate_mb_per_sec: f64,
}

/// OOM risk assessment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OomRisk {
    Low,
    Medium,
    High,
    Critical,
}

/// Network metrics model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetMetrics {
    /// Per-interface counter stats (all interfaces, all hosts combined)
    pub bytes_recv: BTreeMap<String, CounterStats>,
    pub bytes_sent: BTreeMap<String, CounterStats>,
    pub packets_recv: BTreeMap<String, CounterStats>,
    pub packets_sent: BTreeMap<String, CounterStats>,
    /// Primary interface analysis (highest-use interface per host, aggregated)
    pub primary: PrimaryNetStats,
}

/// Primary-interface network stats.
///
/// For each host the interface carrying the most bytes (recv+sent) is selected
/// as "primary". Instantaneous byte rates are computed from the counter
/// derivatives, then fed through [`GaugeStats`] to produce mean, std, p5, p95,
/// and a windowed trend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrimaryNetStats {
    /// Combined counter stats for bytes received across primary interfaces
    pub bytes_recv: CounterStats,
    /// Combined counter stats for bytes sent across primary interfaces
    pub bytes_sent: CounterStats,
    /// Receive rate in bytes/sec across primary interfaces
    pub recv_rate: GaugeStats,
    /// Send rate in bytes/sec across primary interfaces
    pub send_rate: GaugeStats,
}

/// Disk metrics model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiskMetrics {
    pub io_throughput_mbps: DiskThroughput,
    pub space_utilization: BTreeMap<String, DiskSpace>,
}

/// Disk I/O throughput in MB/s
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiskThroughput {
    pub read: f64,
    pub write: f64,
}

/// Disk space utilization per mount point
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiskSpace {
    pub used_percent: f64,
}

/// System load metrics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemLoadMetrics {
    pub load_1min: f64,
    pub load_5min: f64,
    pub load_15min: f64,
    /// Number of hosts observed
    pub host_count: usize,
    /// Percentage of hosts where mean load5 / n_cpus > 1.0 (0.0–100.0)
    pub overloaded_percent: f64,
}

/// Overall resource pressure composite
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourcePressure {
    pub overall_score: f64,
    pub bottleneck: Option<Bottleneck>,
}

/// Resource bottleneck identification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Bottleneck {
    Cpu,
    Memory,
    Disk,
    Network,
    None,
}

/// Status of a single anomaly check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnomalyStatus {
    NotDetected,
    Detected {
        severity: Severity,
        description: String,
    },
}

/// Host anomalies — one named field per check, each either detected or not.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HostAnomalies {
    pub cpu_spike: AnomalyStatus,
    pub memory_leak: AnomalyStatus,
    pub disk_full: AnomalyStatus,
    pub swap_thrashing: AnomalyStatus,
    pub system_overload: AnomalyStatus,
}

/// Anomaly severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Severity {
    Warning,
    Critical,
}

/// Holochain process metrics aggregated across all matching processes and hosts.
///
/// Sourced from telegraf's `inputs.procstat` plugin with `pattern = "holochain"`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcessMetrics {
    /// CPU usage percentage across all Holochain processes
    pub cpu_usage: GaugeStats,
    /// Proportional set size (bytes) — shares mmap'd pages proportionally
    pub memory_pss: GaugeStats,
    /// Number of threads per process
    pub num_threads: GaugeStats,
    /// Number of open file descriptors per process
    pub num_fds: GaugeStats,
}

/// Linux Pressure Stall Information metrics.
/// Only available when telegraf is configured with `inputs.kernel` `collect=["psi"]`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PsiMetrics {
    /// CPU only has "some" (no "full" stall type)
    pub cpu: PsiStall,
    /// Memory has both "some" and "full" stall types
    pub memory: PsiResource,
    /// IO has both "some" and "full" stall types
    pub io: PsiResource,
}

/// PSI for a resource that has both "some" and "full" stall types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PsiResource {
    pub some: PsiStall,
    pub full: PsiStall,
}

/// PSI stall readings for one resource/type combination
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PsiStall {
    /// Full gauge stats on avg10 readings (most responsive kernel window)
    pub avg10: GaugeStats,
    /// Mean of avg60 readings over the run
    pub avg60_mean: f64,
    /// Mean of avg300 readings over the run
    pub avg300_mean: f64,
}
