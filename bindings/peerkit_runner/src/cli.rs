use crate::common::to_connection_string;
use clap::Parser;
use wind_tunnel_runner::parse_agent_behaviour;
use wind_tunnel_runner::prelude::{ReporterOpt, WindTunnelScenarioCli};

#[derive(Parser)]
#[command(about, long_about = None)]
pub struct WindTunnelPeerkitScenarioCli {
    /// The dial multiaddr of the Peerkit relay, e.g.
    /// `/ip4/1.2.3.4/udp/9000/webrtc-direct/certhash/uEi...` (a trailing
    /// `/p2p/<peer-id>` component is accepted but not required).
    ///
    /// Repeat the flag to supply more than one relay. Falls back to the
    /// `PEERKIT_RELAY_DIAL_ADDR` environment variable when the flag is absent
    /// (used by Nomad runs, where the deployed relay address is stored as a
    /// Nomad job secret because the repository is public).
    #[clap(
        long,
        required = true,
        env = "PEERKIT_RELAY_DIAL_ADDR",
        hide_env_values = true,
        value_delimiter = ','
    )]
    pub relay_dial_addr: Vec<String>,

    /// The number of agents to run. All agents will run on the local machine.
    /// Each agent spawns a `peerkit node` process.
    ///
    /// Defaults to 1.
    #[clap(long)]
    pub agents: Option<usize>,

    /// The number of seconds to run the scenario for.
    #[clap(long)]
    pub duration: Option<u64>,

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

    /// Run this test as a soak test, ignoring any configured duration and continuing to run until stopped.
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

    /// Set the ID of this run
    ///
    /// If not set, a random ID is used.
    #[arg(long, short)]
    pub run_id: Option<String>,
}

impl TryInto<WindTunnelScenarioCli> for WindTunnelPeerkitScenarioCli {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<WindTunnelScenarioCli, Self::Error> {
        let connection_string = to_connection_string(self.relay_dial_addr);
        Ok(WindTunnelScenarioCli {
            connection_string: Some(connection_string),
            agents: self.agents,
            behaviour: self.behaviour,
            duration: self.duration,
            soak: self.soak,
            no_progress: self.no_progress,
            reporter: self.reporter,
            run_id: self.run_id,
        })
    }
}
