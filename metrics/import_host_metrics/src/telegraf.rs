mod config;

use std::path::PathBuf;

pub use self::config::TelegrafConfig;

/// An interface to the Telegraf metrics reporter to import aggregated host metrics.
pub struct Telegraf {
    /// The path to the Telegraf configuration file.
    config_path: PathBuf,
}

impl Telegraf {
    /// Creates a new [`Telegraf`] instance with the specified configuration file path.
    pub fn new<P>(config_path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        Telegraf {
            config_path: config_path.into(),
        }
    }

    /// Runs the Telegraf reporter with the current configuration.
    pub fn run(&self) -> anyhow::Result<()> {
        let mut process = std::process::Command::new("telegraf")
            .arg("--config")
            .arg(&self.config_path)
            .arg("--once")
            .arg("--debug")
            .spawn()?;

        debug!("Running Telegraf with PID: {pid}", pid = process.id());
        // wait for the process to finish
        let status = process.wait()?;
        // write stdout
        debug!("Telegraf process finished with status: {status}");
        if status.success() {
            info!("Telegraf ran successfully");
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Telegraf process failed with status: {}",
                status
            ))
        }
    }
}
