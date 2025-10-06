//! Provides the ability to configure and run a Holochain conductor as a [`Child`] process.

use std::{fs, path::PathBuf, process::Stdio, time::Duration};

use anyhow::{anyhow, Context};
use holochain_conductor_api::{
    conductor::{paths::ConfigRootPath, ConductorConfig, KeystoreConfig},
    AdminInterfaceConfig, InterfaceDriver,
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
#[derive(Debug, Clone, Default)]
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
    conductor_config_root_path: Option<ConfigRootPath>,

    /// The target arc factor for the conductor, leave as [`None`] to use the Holochain default or
    /// set to `0` to be configured as a zero-arc conductor.
    target_arc_factor: Option<u32>,
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
        self.conductor_config_root_path = Some(path.into().into());
        self
    }

    /// Build a [`HolochainConfig`], applying the overrides and defaults where appropriate.
    ///
    /// Returns an error if required fields are not set.
    pub(crate) fn build(self) -> WindTunnelResult<HolochainConfig> {
        let bin_path = self.bin_path.unwrap_or(PathBuf::from("holochain"));
        let conductor_config_root_path = self.conductor_config_root_path.ok_or(anyhow!(
            "Conductor config root path not set, this should be set by the Wind Tunnel runner"
        ))?;
        let conductor_data_root_path = conductor_config_root_path.is_also_data_root_path();
        let mut conductor_config = if let Some(admin_port) = self.admin_port {
            ConductorConfig {
                data_root_path: Some(conductor_data_root_path.clone()),
                admin_interfaces: Some(vec![AdminInterfaceConfig {
                    driver: InterfaceDriver::Websocket {
                        port: admin_port,
                        allowed_origins: AllowedOrigins::Any,
                    },
                }]),
                keystore: KeystoreConfig::LairServerInProc {
                    lair_root: Some(conductor_data_root_path.try_into()?),
                },
                ..Default::default()
            }
        } else {
            ConductorConfig::default()
        };
        if let Some(target_arc_factor) = self.target_arc_factor {
            conductor_config.network.target_arc_factor = target_arc_factor;
        }

        Ok(HolochainConfig {
            bin_path,
            agent_name: self.agent_name,
            conductor_config_root_path,
            conductor_config,
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
    conductor_config_root_path: ConfigRootPath,

    /// The conductor configuration that is written to a file and passed, as a path, to the
    /// conductor when started.
    pub conductor_config: ConductorConfig,
}

/// Holds the [`Child`] process that is running the Holochain conductor, as well as the path to the
/// directory that the conductor stores its data so that they can be cleaned up when
/// [`Drop::drop`]'ed.
#[derive(Debug)]
pub struct HolochainRunner {
    /// The [`Child`] process that is running the Holochain conductor.
    _holochain_handle: Child,

    /// The path where the running conductor stores its data.
    conductor_root_path: PathBuf,
}

impl HolochainRunner {
    /// Runs an instance of Holochain, using the passed [`HolochainConfig`]. Storing the [`Child`]
    /// process internally so it can be gracefully shutdown and clean up directories on
    /// [`Drop::drop`].
    pub async fn run(config: &HolochainConfig) -> WindTunnelResult<Self> {
        let conductor_root_path = config.conductor_config_root_path.to_path_buf();
        fs::create_dir_all(&conductor_root_path).with_context(|| {
            format!(
                "Failed to create conductor root directory '{}'",
                conductor_root_path.display()
            )
        })?;
        let conductor_root_path = conductor_root_path.canonicalize()?;

        log::trace!(
            "Writing conductor config file to '{}'",
            conductor_root_path.display()
        );
        let conductor_config_path = write_config(
            config.conductor_config_root_path.clone(),
            &config.conductor_config,
        )
        .context("Failed to write conductor config file")?
        .to_path_buf();

        log::info!("Running a Holochain conductor");
        let mut holochain_handle = Command::new(&config.bin_path)
            .arg("--config-path")
            .arg(conductor_config_path)
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

        timeout(Duration::from_secs(5), async move {
            let mut stdout_lines = BufReader::new(holochain_stdout).lines();
            loop {
                let line = stdout_lines
                    .next_line()
                    .await
                    .context("Failed to read line from Holochain conductor stdout")?
                    .ok_or(anyhow!("Holochain conductor shutdown before it was ready"))?;
                let mut log_target = "holochain_conductor".to_string();
                if let Some(agent_name) = &config.agent_name {
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

        Ok(Self {
            _holochain_handle: holochain_handle,
            conductor_root_path,
        })
    }
}

impl Drop for HolochainRunner {
    fn drop(&mut self) {
        log::trace!("Cleaning up the conductor files");
        if let Err(err) = fs::remove_dir_all(&self.conductor_root_path) {
            log::error!("Failed to cleanup the conductor files: {err}");
        } else {
            log::info!("Successfully cleaned up the conductor files");
        }

        if let Some(parent) = self.conductor_root_path.parent() {
            if fs::remove_dir(parent).is_ok() {
                log::info!("Successfully cleaned up all conductor directories");
            }
        }
    }
}
