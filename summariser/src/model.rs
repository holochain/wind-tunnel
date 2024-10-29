use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SummaryOutput {
    pub run_summary: RunSummary,
    pub data: serde_json::Value,
}

impl SummaryOutput {
    pub fn new<V>(run_summary: RunSummary, data: V) -> anyhow::Result<Self>
    where
        V: serde::Serialize,
    {
        Ok(Self {
            run_summary,
            data: serde_json::to_value(data)?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingTrend {
    pub trend: Vec<f64>,
    pub window_duration: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardTimingsStats {
    pub mean: f64,
    pub std: f64,
    pub within_std: f64,
    pub within_2std: f64,
    pub within_3std: f64,
    pub trend: TimingTrend,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardRateStats {
    pub mean: f64,
    pub trend: Vec<u32>,
    pub window_duration: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
pub struct PartitionKey {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionTimingStats {
    pub key: Vec<PartitionKey>,
    pub summary_timing: StandardTimingsStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
