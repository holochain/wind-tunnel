use clap::Parser;

#[derive(Parser)]
#[command(about, long_about = None)]
pub struct WindTunnelScenarioCli {
    /// A connection string for the service to test
    #[clap(short, long)]
    pub connection_string: String,

    /// The number of agents to run
    #[clap(long)]
    pub agents: Option<usize>,

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
}

fn parse_agent_behaviour(s: &str) -> anyhow::Result<(String, usize)> {
    let mut parts = s.split(':');
    let name = parts.next().map(|s| s.to_string()).ok_or(anyhow::anyhow!("No name specified for behaviour"))?;

    let count = parts.next().and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);

    Ok((name, count))
}
