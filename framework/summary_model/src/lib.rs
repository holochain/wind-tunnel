use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sha3::Digest;

use std::collections::BTreeMap;
use std::io::{BufRead, Read, Write};
use std::path::PathBuf;

/// Arguments for initializing a [`RunSummary`]
#[derive(Debug, Clone)]
pub struct RunSummaryInitArgs {
    /// The unique run id for the run
    pub run_id: String,
    /// The name of the scenario that was run
    pub scenario_name: String,
    /// The time the run started as a Unix timestamp in seconds
    pub started_at: i64,
    /// The number of peers configured
    pub peer_count: usize,
    /// The version of Wind Tunnel that was used for this run
    pub wind_tunnel_version: String,
}

/// Build information of any software used in the run.
/// This is expected to be specific to the binding used by a scenario.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildInfo {
    /// A label of the type of build info included.
    /// This should be consistent for any scenarios providing
    /// the same type of build info.
    ///
    /// This is to facilitate duck-typing the info field.
    pub info_type: String,

    /// The actual build info
    pub info: serde_json::Value,
}

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
    /// Note: This is only meaningful for single-conductor tests with the standard Wind Tunnel
    /// runner. In general, each node only sees the roles it was assigned and not the roles that
    /// were assigned across the network.
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
    pub assigned_behaviours: BTreeMap<String, usize>,
    /// Environment variables set for the run
    ///
    /// This won't capture all environment variables. Just the ones that the runner is aware of or
    /// that are included by the scenario itself.
    pub env: BTreeMap<String, String>,
    /// The version of Wind Tunnel that was used for this run
    ///
    /// This is the version of the Wind Tunnel runner that was used to run the scenario.
    pub wind_tunnel_version: String,
    /// The build info that was used for this run
    pub build_info: Option<BuildInfo>,
}

impl RunSummary {
    /// Construct a new [`RunSummary`] from the specified args
    pub fn new(args: RunSummaryInitArgs) -> Self {
        Self {
            run_id: args.run_id,
            scenario_name: args.scenario_name,
            started_at: args.started_at,
            run_duration: None,
            peer_count: args.peer_count,
            peer_end_count: 0,
            assigned_behaviours: BTreeMap::new(),
            env: BTreeMap::new(),
            wind_tunnel_version: args.wind_tunnel_version,
            build_info: None,
        }
    }

    /// Construct [`RunSummary`] with the specified run duration
    pub fn with_run_duration(mut self, run_duration: Option<u64>) -> Self {
        self.run_duration = run_duration;
        self
    }

    /// Construct [`RunSummary`] with the specified assigned behaviours
    pub fn with_assigned_behaviours(
        mut self,
        assigned_behaviours: BTreeMap<String, usize>,
    ) -> Self {
        self.assigned_behaviours = assigned_behaviours;
        self
    }

    /// Set the peer end count
    pub fn set_peer_end_count(&mut self, peer_end_count: usize) {
        self.peer_end_count = peer_end_count;
    }

    /// Set the build info
    pub fn set_build_info(&mut self, build_info: BuildInfo) {
        self.build_info = Some(build_info);
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

#[cfg(test)]
mod tests {

    use super::*;

    #[derive(Serialize, Deserialize)]
    struct MyBuildInfo {
        version: String,
        library_version: String,
    }

    #[test]
    fn test_should_construct_run_summary() {
        let run_summary = RunSummary::new(RunSummaryInitArgs {
            run_id: "test".to_string(),
            scenario_name: "scenario".to_string(),
            started_at: 100,
            peer_count: 2,
            wind_tunnel_version: "1.0.0".to_string(),
        });
        assert_eq!(run_summary.run_id, "test");
        assert_eq!(run_summary.scenario_name, "scenario");
        assert_eq!(run_summary.started_at, 100);
        assert_eq!(run_summary.run_duration, None);
        assert_eq!(run_summary.peer_count, 2);
        assert_eq!(run_summary.peer_end_count, 0);
        assert!(run_summary.assigned_behaviours.is_empty());
        assert!(run_summary.env.is_empty());
        assert_eq!(run_summary.wind_tunnel_version, "1.0.0");
        assert_eq!(run_summary.build_info, None);

        let mut assigned_behaviours = BTreeMap::new();
        assigned_behaviours.insert("behaviour-1".to_string(), 3);
        assigned_behaviours.insert("behaviour-2".to_string(), 5);
        let mut run_summary = RunSummary::new(RunSummaryInitArgs {
            run_id: "test-run-1".to_string(),
            scenario_name: "test-scenario".to_string(),
            started_at: 1625078400,
            peer_count: 2,
            wind_tunnel_version: "1.0.0".to_string(),
        })
        .with_run_duration(Some(3600))
        .with_assigned_behaviours(assigned_behaviours);

        run_summary.set_peer_end_count(4);
        let build_info = build_info();
        run_summary.set_build_info(build_info.clone());

        assert_eq!(run_summary.run_id, "test-run-1");
        assert_eq!(run_summary.scenario_name, "test-scenario");
        assert_eq!(run_summary.started_at, 1625078400);
        assert_eq!(run_summary.run_duration, Some(3600));
        assert_eq!(run_summary.peer_count, 2);
        assert_eq!(run_summary.peer_end_count, 4);
        assert_eq!(run_summary.wind_tunnel_version, "1.0.0");
        assert_eq!(run_summary.build_info, Some(build_info));

        assert_eq!(run_summary.assigned_behaviours.get("behaviour-1"), Some(&3));
        assert_eq!(run_summary.assigned_behaviours.get("behaviour-2"), Some(&5));
    }

    #[test]
    fn test_should_set_build_info() {
        let mut run_summary = RunSummary::new(RunSummaryInitArgs {
            run_id: "test".to_string(),
            scenario_name: "scenario".to_string(),
            started_at: 100,
            peer_count: 2,
            wind_tunnel_version: "1.0.0".to_string(),
        });
        assert_eq!(run_summary.build_info, None);

        let build_info = build_info();

        run_summary.set_build_info(build_info.clone());
        assert_eq!(run_summary.build_info, Some(build_info));
    }

    #[inline(always)]
    fn build_info() -> BuildInfo {
        BuildInfo {
            info_type: "my_build_info".to_string(),
            info: serde_json::to_value(MyBuildInfo {
                version: "0.0.100".to_string(),
                library_version: "0.0.100".to_string(),
            })
            .unwrap(),
        }
    }
}
