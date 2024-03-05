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
