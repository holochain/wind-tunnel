use crate::cli::WindTunnelScenarioCli;
use clap::Parser;

/// Initialise the CLI and logging for the wind tunnel runner.
pub fn init() -> WindTunnelScenarioCli {
    env_logger::init();

    WindTunnelScenarioCli::parse()
}
