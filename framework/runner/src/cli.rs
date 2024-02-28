use clap::Parser;

#[derive(Parser)]
#[command(about, long_about = None)]
pub struct WindTunnelScenarioCli {
    /// The number of agents to run
    #[clap(long, default_value = "10")]
    pub agents: usize,

    /// The number of seconds to run the scenario for
    #[clap(long)]
    pub duration: Option<u64>,

    /// Run this test as a soak test, ignoring any configured duration and continuing to run until stopped
    #[clap(long, default_value = "false")]
    pub soak: bool,
}
