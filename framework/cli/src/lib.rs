use clap::Parser;

#[derive(Parser)]
#[command(about, long_about = None)]
pub struct WindTunnelCli {
    /// The number of agents to run
    #[clap(long, default_value = "10")]
    pub agents: usize,
}

pub fn init() -> WindTunnelCli {
    WindTunnelCli::parse()
}
