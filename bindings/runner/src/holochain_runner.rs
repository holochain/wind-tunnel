use std::{fs, path::PathBuf, process::Stdio, time::Duration};

use anyhow::{anyhow, Context};
use holochain_conductor_api::{
    conductor::{paths::ConfigRootPath, ConductorConfig, KeystoreConfig},
    AdminInterfaceConfig, InterfaceDriver,
};
use holochain_conductor_config::config::write_config;
use holochain_types::websocket::AllowedOrigins;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    time::timeout,
};
use wind_tunnel_runner::prelude::WindTunnelResult;

#[derive(Debug, Default)]
pub struct HolochainConfigBuilder {
    bin_path: Option<PathBuf>,
    admin_port: Option<u16>,
    conductor_config_root_path: Option<ConfigRootPath>,
    target_arc_factor: Option<u32>,
}

impl HolochainConfigBuilder {
    pub fn with_target_arc_factor(&mut self, target_arc_factor: u32) -> &mut Self {
        self.target_arc_factor = Some(target_arc_factor);
        self
    }

    pub(crate) fn with_bin_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.bin_path = Some(path.into());
        self
    }

    pub(crate) fn with_admin_port(mut self, port: u16) -> Self {
        self.admin_port = Some(port);
        self
    }

    pub(crate) fn with_conductor_root_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.conductor_config_root_path = Some(path.into().into());
        self
    }

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
            conductor_config_root_path,
            conductor_config,
        })
    }
}

#[derive(Debug)]
pub struct HolochainConfig {
    bin_path: PathBuf,
    conductor_config_root_path: ConfigRootPath,
    conductor_config: ConductorConfig,
}

#[derive(Debug)]
pub struct HolochainRunner {
    _holochain_handle: Child,
    conductor_root_path: PathBuf,
}

impl HolochainRunner {
    /// Runs an instance of Holochain, using the passed [`HolochainConfig`]. Storing the [`Child`]
    /// process internally so it can be gracefully shutdown on [`Drop::drop`].
    pub async fn run(config: &HolochainConfig) -> WindTunnelResult<Self> {
        let conductor_root_path = config.conductor_config_root_path.to_path_buf();
        if !fs::exists(&conductor_root_path)? {
            fs::create_dir(&conductor_root_path).with_context(|| {
                format!(
                    "Failed to create conductor root directory '{}'",
                    conductor_root_path.display()
                )
            })?;
        }

        let conductor_config_path = write_config(
            config.conductor_config_root_path.clone(),
            &config.conductor_config,
        )
        .context("Failed to write conductor config file")?
        .to_path_buf();

        log::trace!("Generating conductor password");
        let password: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .chain(['\n'])
            .collect();

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
            .as_mut()
            .context("Failed to get stdin for the running Holochain conductor")?
            .write_all(password.as_bytes())
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
                log::info!(target: "holochain_conductor", "{line}");
                if line == "Conductor ready." {
                    if log::log_enabled!(target: "holochain_conductor", log::Level::Info) {
                        tokio::spawn(async move {
                            while let Ok(Some(line)) = stdout_lines.next_line().await {
                                log::info!(target: "holochain_conductor", "{line}");
                            }
                        });
                    }

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
    }
}
