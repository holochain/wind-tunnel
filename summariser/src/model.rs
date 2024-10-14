use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
pub struct StandardTimingsStats {
    pub mean: f64,
    pub std: f64,
    pub within_std: f64,
    pub within_2std: f64,
    pub within_3std: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardRatioStats {
    pub mean: f64,
    pub std: f64,
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardRateStats {
    pub trend: serde_json::Value,
    pub mean_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionedRateStats {
    pub trend: serde_json::Value,
    pub rates: HashMap<String, f64>,
    pub by_partition: HashMap<String, serde_json::Value>,
}
