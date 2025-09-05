mod holochain_metrics;
mod host_metrics;

use std::time::Duration;

use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

pub use self::holochain_metrics::{
    HolochainDatabaseKind, HolochainDatabaseMetrics, HolochainMetrics, HolochainMetricsConfig,
    HolochainWorkflowKind, StrumVariantSelector,
};
pub use self::host_metrics::{CpuMetrics, HostMetrics, MemMetrics, NetMetrics};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SummaryOutput {
    pub run_summary: RunSummary,
    pub scenario_metrics: serde_json::Value,
    pub host_metrics: Option<HostMetrics>,
    pub holochain_metrics: Option<HolochainMetrics>,
}

impl SummaryOutput {
    pub fn new<V>(
        run_summary: RunSummary,
        data: V,
        host_metrics: Option<HostMetrics>,
        holochain_metrics: Option<HolochainMetrics>,
    ) -> anyhow::Result<Self>
    where
        V: serde::Serialize,
    {
        Ok(Self {
            run_summary,
            scenario_metrics: serde_json::to_value(data)?,
            host_metrics,
            holochain_metrics,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

/// GaugeStats statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct GaugeStats {
    pub count: usize,
    pub max: f64,
    pub mean: f64,
    pub min: f64,
    pub std: f64,
}

/// Stats which increment during time
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct CounterStats {
    /// initial value
    pub start: u64,
    /// value at the end of the measurement
    pub end: u64,
    /// end - start
    pub delta: u64,
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
