use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sha3::Digest;
use std::collections::HashMap;
use std::io::{BufRead, Read, Write};
use std::path::PathBuf;

/// Summary of a run
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunSummary {
    /// The unique run id
    ///
    /// Chosen by the runner. Unique for each run.
    pub run_id: String,
    /// The name of the scenario that was run
    pub scenario_name: String,
    /// The time the run started
    ///
    /// This is a Unix timestamp in seconds.
    pub started_at: i64,
    /// The duration that the run was configured with, in seconds
    ///
    /// If the run was configured for soak testing, then this will not be set.
    ///
    /// It is possible that the run finished sooner than `started_at + run_duration` if all
    /// behaviours failed. As long as [RunSummary::peer_end_count] is greater than 0 then that
    /// number of agents will have run for the full duration.
    pub run_duration: Option<u64>,
    /// The number of peers configured
    ///
    /// This is the number of peers that were either configured or required by the behaviour
    /// configuration.
    ///
    /// Note: This is only meaningful for single-conductor tests with the standard Wind Tunnel runner
    /// or with the TryCP runner. In general, each node only sees the roles it was assigned and not
    /// the roles that were assigned across the network.
    pub peer_count: usize,
    /// The number of peers at the end of the test
    ///
    /// If some peers exit early, for example due to a fatal error during a behaviour run or an
    /// unavailable conductor, then this will be less than [RunSummary::peer_count].
    ///
    /// Note: This is only meaningful for single-conductor tests with the standard Wind Tunnel runner
    /// or with the TryCP runner. In general, each node only sees the roles it was assigned and not
    /// the roles that were assigned across the network.
    pub peer_end_count: usize,
    /// The behaviour configuration
    ///
    /// This is the number of agents that were assigned to each behaviour.
    ///
    /// Note: This is only meaningful for single-conductor tests with the standard Wind Tunnel runner
    /// or with the TryCP runner. In general, each node only sees the roles it was assigned and not
    /// the roles that were assigned across the network.
    pub assigned_behaviours: HashMap<String, usize>,
    /// Environment variables set for the run
    ///
    /// This won't capture all environment variables. Just the ones that the runner is aware of or
    /// that are included by the scenario itself.
    pub env: HashMap<String, String>,
    /// The version of Wind Tunnel that was used for this run
    ///
    /// This is the version of the Wind Tunnel runner that was used to run the scenario.
    pub wind_tunnel_version: String,
}

impl RunSummary {
    /// Create a new run summary
    pub fn new(
        run_id: String,
        scenario_name: String,
        started_at: i64,
        run_duration: Option<u64>,
        peer_count: usize,
        assigned_behaviours: HashMap<String, usize>,
        wind_tunnel_version: String,
    ) -> Self {
        Self {
            run_id,
            scenario_name,
            started_at,
            run_duration,
            peer_count,
            peer_end_count: 0,
            assigned_behaviours,
            env: HashMap::with_capacity(0),
            wind_tunnel_version,
        }
    }

    /// Set the peer end count
    pub fn set_peer_end_count(&mut self, peer_end_count: usize) {
        self.peer_end_count = peer_end_count;
    }

    /// Add an environment variable
    pub fn add_env(&mut self, key: String, value: String) {
        self.env.insert(key, value);
    }

    /// Compute a fingerprint for this run summary
    ///
    /// The fingerprint is intended to uniquely identify the configuration used to run the scenario.
    /// It uses the
    ///     - Scenario name
    ///     - Run duration
    ///     - Assigned behaviours
    ///     - Selected environment variables
    ///     - Wind Tunnel version
    ///
    /// The fingerprint is computed using [sha3::Sha3_256].
    pub fn fingerprint(&self) -> String {
        let mut hasher = sha3::Sha3_256::new();
        Digest::update(&mut hasher, self.scenario_name.as_bytes());
        if let Some(run_duration) = self.run_duration {
            Digest::update(&mut hasher, run_duration.to_le_bytes());
        }
        self.assigned_behaviours
            .iter()
            .sorted_by_key(|(k, _)| k.to_owned())
            .for_each(|(k, v)| {
                Digest::update(&mut hasher, k.as_bytes());
                Digest::update(&mut hasher, v.to_le_bytes());
            });
        self.env
            .iter()
            .sorted_by_key(|(k, _)| k.to_owned())
            .for_each(|(k, v)| {
                Digest::update(&mut hasher, k.as_bytes());
                Digest::update(&mut hasher, v.as_bytes());
            });
        Digest::update(&mut hasher, self.wind_tunnel_version.as_bytes());

        format!("{:x}", hasher.finalize())
    }
}

/// Append the run summary to a file
///
/// The summary will be serialized to JSON and output as a single line followed by a newline. The
/// recommended file extension is `.jsonl`.
pub fn append_run_summary(run_summary: RunSummary, path: PathBuf) -> anyhow::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)?;
    store_run_summary(run_summary, &mut file)?;
    let _ = file.write("\n".as_bytes())?;
    Ok(())
}

/// Serialize the run summary to a writer
pub fn store_run_summary<W: Write>(run_summary: RunSummary, writer: &mut W) -> anyhow::Result<()> {
    serde_json::to_writer(writer, &run_summary)?;
    Ok(())
}

/// Load a run summary from a reader
pub fn load_run_summary<R: Read>(reader: R) -> anyhow::Result<RunSummary> {
    let reader = std::io::BufReader::new(reader);
    let run_summary: RunSummary = serde_json::from_reader(reader)?;
    Ok(run_summary)
}

/// Load run summaries from a file
///
/// The file should contain one JSON object per line. This is the format produced by
/// [append_run_summary].
pub fn load_summary_runs(path: PathBuf) -> anyhow::Result<Vec<RunSummary>> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut runs = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let run: RunSummary = serde_json::from_str(&line)?;
        runs.push(run);
    }
    Ok(runs)
}
