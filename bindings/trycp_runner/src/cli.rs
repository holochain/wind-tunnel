use clap::Parser;
use serde::Deserialize;
use std::path::PathBuf;
use wind_tunnel_runner::parse_agent_behaviour;
use wind_tunnel_runner::prelude::{ReporterOpt, WindTunnelScenarioCli};

#[derive(Deserialize)]
struct Targets {
    nodes: Vec<String>,
}

#[derive(Parser)]
#[command(about, long_about = None)]
pub struct WindTunnelTryCPScenarioCli {
    /// Path to the targets file to use.
    ///
    /// Should be a YAML file with a `nodes` field containing a list of TryCP targets.
    #[clap(long)]
    pub targets: PathBuf,

    /// Assign a behaviour to a number of agents. Specify the behaviour and number of agents to assign
    /// it to in the format `behaviour:count`. For example `--behaviour=login:5`.
    ///
    /// Specifying the count is optional and will default to 1. This is a useful default if you want to
    /// run distributed tests and want a single agent to use a single behaviour on that node.
    ///
    /// You can specify multiple behaviours by using the flag multiple times. For example `--behaviour=add_to_list:5 --behaviour=favourite_items:5`.
    ///
    /// For however many agents you assign to behaviours in total, it must be less than or equal to the total number of agents for this scenario.
    /// If it is less than the total number of agents then the remaining agents will be assigned the default behaviour.
    ///
    /// If the configuration is invalid then the scenario will fail to start.
    #[clap(long, short, value_parser = parse_agent_behaviour)]
    pub behaviour: Vec<(String, usize)>,

    /// The number of seconds to run the scenario for
    #[clap(long)]
    pub duration: Option<u64>,

    /// Run this test as a soak test, ignoring any configured duration and continuing to run until stopped
    #[clap(long, default_value = "false")]
    pub soak: bool,

    /// Do not show a progress bar on the CLI.
    ///
    /// This is recommended for CI/CD environments where the progress bar isn't being looked at by anyone and is just adding noise to the logs.
    #[clap(long, default_value = "false")]
    pub no_progress: bool,

    /// The reporter to use.
    #[arg(long, value_enum, default_value_t = ReporterOpt::InMemory)]
    pub reporter: ReporterOpt,
}

impl TryInto<WindTunnelScenarioCli> for WindTunnelTryCPScenarioCli {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<WindTunnelScenarioCli, Self::Error> {
        let targets = std::fs::read_to_string(&self.targets)?;
        let targets: Targets = serde_yaml::from_str(&targets)?;

        Ok(WindTunnelScenarioCli {
            // Connection string is already forwarded but is supposed to be a single value.
            // Pack values together and extract by agent id in helpers.
            connection_string: targets.nodes.join(","),
            agents: Some(targets.nodes.len()),
            behaviour: self.behaviour,
            duration: self.duration,
            soak: self.soak,
            no_progress: self.no_progress,
            reporter: self.reporter,
        })
    }
}