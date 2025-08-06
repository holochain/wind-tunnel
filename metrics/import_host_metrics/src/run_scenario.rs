use serde::Deserialize;

/// Defines a run scenario data from `run_summary.jsonl`.
#[derive(Debug, Clone, Deserialize)]
pub struct RunScenario {
    /// Unique identifier for the run.
    pub run_id: String,
    /// Name of the scenario being run.
    pub scenario_name: String,
    /// Timestamp when the run started, in seconds since the epoch.
    pub started_at: u64,
    /// Run duration in seconds.
    pub run_duration: u64,
}
