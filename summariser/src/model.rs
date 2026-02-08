mod holochain_metrics;
mod host_metrics;

use std::time::Duration;

use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

pub use self::holochain_metrics::{HolochainDatabaseKind, HolochainWorkflowKind};
pub use self::host_metrics::{
    AnomalyStatus, Bottleneck, CpuMetrics, DiskMetrics, DiskSpace, DiskThroughput, HostAnomalies,
    HostMetrics, MemMetrics, NetMetrics, OomRisk, PrimaryNetStats, ProcessMetrics, PsiMetrics,
    PsiResource, PsiStall, ResourcePressure, Severity, SystemLoadMetrics,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SummaryOutput {
    pub run_summary: RunSummary,
    pub scenario_metrics: serde_json::Value,
    pub host_metrics: Option<HostMetrics>,
}

impl SummaryOutput {
    pub fn new<V>(
        run_summary: RunSummary,
        data: V,
        host_metrics: Option<HostMetrics>,
    ) -> anyhow::Result<Self>
    where
        V: serde::Serialize,
    {
        Ok(Self {
            run_summary,
            scenario_metrics: serde_json::to_value(data)?,
            host_metrics,
        })
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimingTrend {
    pub trend: Vec<f64>,
    pub window_duration: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StandardTimingsStats {
    pub mean: f64,
    pub std: f64,
    pub within_std: f64,
    pub within_2std: f64,
    pub within_3std: f64,
    pub trend: TimingTrend,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StandardRateStats {
    pub mean: f64,
    pub trend: Vec<u32>,
    pub window_duration: String,
}

/// Gauge statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct GaugeStats {
    pub mean: f64,
    pub std: f64,
    /// 5th percentile (lower bound of the 90% operating range)
    pub p5: f64,
    /// 95th percentile (upper bound of the 90% operating range)
    pub p95: f64,
    /// Windowed mean trend over time
    pub trend: GaugeTrend,
}

/// Windowed mean trend for a gauge metric
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct GaugeTrend {
    /// Windowed mean values over time
    pub trend: Vec<f64>,
    /// Duration of each window (e.g. "10s", "60s")
    pub window_duration: String,
}

/// GaugeStats for a specific partition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PartitionGaugeStats {
    pub key: Vec<PartitionKey>,
    pub gauge_stats: GaugeStats,
}

/// GaugeStats partitioned by tag/key
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PartitionedGaugeStats {
    pub partitions: Vec<PartitionGaugeStats>,
}

/// Stats which increment during time
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct CounterStats {
    /// The difference between the first and last observed value of the counter during the measurement period
    pub count: u64,
    /// Duration of the measurement
    pub measurement_duration: Duration,
    /// Growth rate per second
    pub rate_per_second: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
pub struct PartitionKey {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PartitionTimingStats {
    pub key: Vec<PartitionKey>,
    pub summary_timing: StandardTimingsStats,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PartitionedTimingStats {
    pub mean: f64,
    pub mean_std_dev: f64,
    pub timings: Vec<PartitionTimingStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionRateStats {
    pub key: Vec<PartitionKey>,
    pub summary_rate: StandardRateStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionedRateStats {
    pub mean: f64,
    pub rates: Vec<PartitionRateStats>,
}
