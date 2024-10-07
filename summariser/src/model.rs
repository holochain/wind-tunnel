use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
