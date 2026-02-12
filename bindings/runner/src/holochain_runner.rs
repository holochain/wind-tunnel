//! Provides the ability to configure and run a Holochain conductor as a [`Child`] process.

use std::{fs, path::PathBuf, process::Stdio, time::Duration};

use anyhow::{Context, anyhow};
use holochain_conductor_api::{
    AdminInterfaceConfig, InterfaceDriver,
    conductor::{
        ConductorConfig, KeystoreConfig,
        paths::{ConfigFilePath, ConfigRootPath},
    },
};
use holochain_conductor_config::config::write_config;
use holochain_types::websocket::AllowedOrigins;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    time::timeout,
};
use wind_tunnel_runner::prelude::WindTunnelResult;

/// Used to build a [`HolochainConfig`], which is then passed into [`HolochainRunner::run`] to
/// spawn a [`Child`] process running a Holochain conductor with the specified config.
#[derive(Debug, Default)]
pub struct HolochainConfigBuilder {
    /// The path to the `holochain` binary used to start a conductor.
    ///
    /// If [`None`] when [`Self::build`] is called then it uses the binary in the user's `PATH`.
    bin_path: Option<PathBuf>,

    /// The name of the agent that runs on this conductor.
    ///
    /// If [`None`] when [`Self::build`] is called, then the [`HolochainConfig::agent_name`] field
    /// will also be [`None`].
    agent_name: Option<String>,

    /// If set when [`Self::build`] is called then an admin interface is created on the conductor
    /// that is accessible via this port.
    admin_port: Option<u16>,

    /// The root path where the generated config and the data for the Holochain conductor is
    /// stored.
    conductor_root_path: Option<PathBuf>,

    /// The target arc factor for the conductor, leave as [`None`] to use the Holochain default or
    /// set to `0` to be configured as a zero-arc conductor.
    target_arc_factor: Option<u32>,

    /// The path where influxive metrics will be written, by setting the
    /// env variable HOLOCHAIN_INFLUXIVE_FILE for the holochain process.
    metrics_path: Option<PathBuf>,
}

impl HolochainConfigBuilder {
    /// Set the target arc factor for the conductor, set to `0` to configure the conductor as a
    /// zero-arc conductor.
    pub fn with_target_arc_factor(&mut self, target_arc_factor: u32) -> &mut Self {
        self.target_arc_factor = Some(target_arc_factor);
        self
    }

    /// Override the path to the `holochain` binary that is used to start a conductor.
    pub(crate) fn with_bin_path(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.bin_path = Some(path.into());
        self
    }

    /// Create an admin interface on the conductor that is accessible via this port.
    pub(crate) fn with_admin_port(&mut self, port: u16) -> &mut Self {
        self.admin_port = Some(port);
        self
    }

    /// Set the name of the agent that will be running on this conductor.
    pub(crate) fn with_agent_name(&mut self, agent_name: impl Into<String>) -> &mut Self {
        self.agent_name = Some(agent_name.into());
        self
    }

    /// Set the root path of the conductor where the generated config is written and the conductor
    /// stores its data.
    pub(crate) fn with_conductor_root_path(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.conductor_root_path = Some(path.into());
        self
    }

    pub(crate) fn with_metrics_path(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.metrics_path = Some(path.into());
        self
    }

    /// Build a [`HolochainConfig`], applying the overrides and defaults where appropriate.
    ///
    /// Returns an error if required fields are not set.
    pub(crate) fn build(self) -> WindTunnelResult<HolochainConfig> {
        let bin_path = self.bin_path.unwrap_or(PathBuf::from("holochain"));
        let conductor_root_path = self.conductor_root_path.ok_or(anyhow!(
            "Conductor root path not set, this should be set by the Wind Tunnel runner"
        ))?;
        let keystore_path = conductor_root_path.clone().join("ks");
        let mut conductor_config = if let Some(admin_port) = self.admin_port {
            ConductorConfig {
                data_root_path: Some(conductor_root_path.clone().into()),
                admin_interfaces: Some(vec![AdminInterfaceConfig {
                    driver: InterfaceDriver::Websocket {
                        port: admin_port,
                        danger_bind_addr: None,
                        allowed_origins: AllowedOrigins::Any,
                    },
                }]),
                keystore: KeystoreConfig::LairServerInProc {
                    lair_root: Some(keystore_path.clone().into()),
                },
                ..Default::default()
            }
        } else {
            ConductorConfig::default()
        };
        if let Some(target_arc_factor) = self.target_arc_factor {
            conductor_config.network.target_arc_factor = target_arc_factor;
        }
        let metrics_path = self.metrics_path.ok_or(anyhow!(
            "Metrics path not set, this should be set by the Wind Tunnel runner"
        ))?;

        Ok(HolochainConfig {
            bin_path,
            agent_name: self.agent_name,
            conductor_root_path,
            conductor_config,
            metrics_path,
        })
    }
}

/// The configuration of the conductor itself as managed by Wind Tunnel as well as the
/// [`ConductorConfig`] that is written to the [`ConfigRootPath`] and passed to the conductor.
///
/// Must be created with a [`HolochainConfigBuilder`] by calling [`HolochainConfigBuilder::build`]
/// once all fields are correctly set.
#[derive(Debug, Clone)]
pub struct HolochainConfig {
    /// The path to the `holochain` binary used to start a conductor.
    bin_path: PathBuf,

    /// The name of the agent that runs on this conductor.
    agent_name: Option<String>,

    /// The path where the generated config and the data for the Holochain conductor is stored.
    conductor_root_path: PathBuf,

    /// The conductor configuration that is written to a file and passed, as a path, to the
    /// conductor when started.
    conductor_config: ConductorConfig,

    /// The path where influxive metrics will be written, by setting the
    /// env variable HOLOCHAIN_INFLUXIVE_FILE for the holochain process.
    metrics_path: PathBuf,
}

/// Holds the [`Child`] process that is running the Holochain conductor, as well as the path to the
/// directory that the conductor stores its data so that they can be cleaned up when
/// [`Drop::drop`]'ed.
#[derive(Debug)]
pub struct HolochainRunner {
    /// The config of this holochain runner instance.
    config: HolochainConfig,

    /// The [`Child`] process that is running the Holochain conductor.
    holochain_handle: Option<Child>,
}

impl HolochainRunner {
    /// Create a holochain runner with the provided config, but do not start it.
    pub fn create(config: &HolochainConfig) -> WindTunnelResult<Self> {
        fs::create_dir_all(&config.conductor_root_path).with_context(|| {
            format!(
                "Failed to create cChonductor root directory '{}'",
                config.conductor_root_path.display()
            )
        })?;
        let conductor_root_path = config.conductor_root_path.canonicalize()?;

        log::trace!(
            "Writing conductor config file to '{}'",
            conductor_root_path.display()
        );
        write_config(
            config.conductor_root_path.clone().into(),
            &config.conductor_config,
        )
        .context("Failed to write conductor config file")?;

        Ok(Self {
            config: config.clone(),
            holochain_handle: None,
        })
    }

    /// Runs an instance of Holochain from the stored [`HolochainConfig`]. Storing the [`Child`]
    /// process internally so it can be gracefully shutdown and clean up directories on
    /// [`Drop::drop`].
    pub async fn run(&mut self) -> WindTunnelResult<()> {
        let config_root_path: ConfigRootPath = self.config.conductor_root_path.clone().into();
        let config_file_path: ConfigFilePath = config_root_path.into();

        log::info!("Running a Holochain conductor");
        let mut holochain_handle = Command::new(&self.config.bin_path)
            .env("HOLOCHAIN_INFLUXIVE_FILE", self.config.metrics_path.clone())
            .arg("--config-path")
            .arg(config_file_path.as_os_str())
            .arg("--piped")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context("Failed to run Holochain conductor")?;

        log::trace!("Passing password to running conductor");
        holochain_handle
            .stdin
            .take()
            .context("Failed to get stdin for the running Holochain conductor")?
            .write_all(b"1234\n")
            .await
            .context("Failed to write the password to the process running the conductor")?;

        log::trace!("Waiting for the conductor to start");
        let holochain_stdout = holochain_handle
            .stdout
            .take()
            .context("Failed to get stdout for the running Holochain conductor")?;

        let agent_name = self.config.agent_name.clone();
        timeout(Duration::from_secs(30), async move {
            let mut stdout_lines = BufReader::new(holochain_stdout).lines();
            loop {
                let line = stdout_lines
                    .next_line()
                    .await
                    .context("Failed to read line from Holochain conductor stdout")?
                    .ok_or(anyhow!("Holochain conductor shutdown before it was ready"))?;
                let mut log_target = "holochain_conductor".to_string();
                if let Some(agent_name) = &agent_name {
                    log_target.push_str("::");
                    log_target.push_str(agent_name);
                }
                log::info!(target: &log_target, "{line}");
                if line == "Conductor ready." {
                    tokio::spawn(async move {
                        while let Ok(Some(line)) = stdout_lines.next_line().await {
                            if log::log_enabled!(target: &log_target, log::Level::Info) {
                                log::info!(target: &log_target, "{line}");
                            }
                        }
                    });

                    return Ok::<(), anyhow::Error>(());
                }
            }
        })
        .await
        .context("Timed-out whilst waiting for the Holochain conductor to be ready")??;

        self.holochain_handle = Some(holochain_handle);

        Ok(())
    }

    pub fn shutdown(&mut self) {
        log::info!("Shutting down holochain conductor");

        // This will drop the child process, which will shutdown the conductor
        self.holochain_handle = None;
    }
}

impl Drop for HolochainRunner {
    fn drop(&mut self) {
        log::trace!("Cleaning up the conductor files");
        if let Err(err) = fs::remove_dir_all(&self.config.conductor_root_path) {
            log::error!("Failed to cleanup the conductor files: {err}");
        } else {
            log::info!("Successfully cleaned up the conductor files");
        }

        if let Some(parent) = self.config.conductor_root_path.parent()
            && fs::remove_dir(parent).is_ok()
        {
            log::info!("Successfully cleaned up all conductor directories");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn conductor_dir_retained_after_shutdown() {
        let tmp = tempdir().unwrap();
        let conductor_root = tmp.path().join("conductor");
        let metrics_path = tmp.path().join("metrics.influx");

        let mut builder = HolochainConfigBuilder::default();
        builder
            .with_conductor_root_path(&conductor_root)
            .with_admin_port(0)
            .with_agent_name("test-agent")
            .with_metrics_path(&metrics_path);
        let config = builder.build().expect("Failed to build HolochainConfig");
        let mut runner = HolochainRunner::create(&config).expect("Failed to create runner");

        // Conductor dir created on runner create
        assert!(conductor_root.exists(),);

        // Shutdown the runner
        runner.shutdown();

        // Conductor dir still exists
        assert!(conductor_root.exists());

        // Drop the runner
        drop(runner);

        // Conductor dir deleted
        assert!(!conductor_root.exists());
    }
}
