use std::{
    io::Write,
    path::Path,
    process::{Child, Command, Stdio},
    time::Duration,
};

use anyhow::Context;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use wind_tunnel_runner::prelude::WindTunnelResult;

#[derive(Debug)]
pub struct HolochainSandbox {
    run_sandbox_handle: Child,
}

impl HolochainSandbox {
    /// Creates a new Holochain sandbox and runs it, storing the [`Child`] process internally so it
    /// can be gracefully shutdown on [`Drop::drop`].
    pub fn create_and_run(hc_path: &Path, admin_port: usize) -> WindTunnelResult<Self> {
        log::trace!("Generating sandbox password");
        let password: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .chain(['\n'])
            .collect();

        log::trace!("Creating new sandbox");
        let mut create_sandbox = Command::new(hc_path)
            .arg("sandbox")
            .arg("--piped")
            .arg("create")
            .arg("--in-process-lair")
            .arg("network")
            .arg("--bootstrap=https://bootstrap.holo.host")
            .arg("webrtc")
            .arg("wss://sbd.holo.host")
            .stdin(Stdio::piped())
            .spawn()
            .context("Failed to run 'hc sandbox --piped create'")?;

        log::trace!("Passing password to new sandbox");
        create_sandbox
            .stdin
            .as_mut()
            .context("Failed to get stdin for the process creating the sandbox")?
            .write_all(password.as_bytes())
            .context("Failed to write the passcode to the process creating the sandbox")?;

        create_sandbox
            .wait()
            .context("Failed to create the Holochain sandbox")?;

        log::info!("Setting admin port of conductor to '{admin_port}'");

        log::trace!("Running the sandbox");
        let mut run_sandbox_handle = Command::new(hc_path)
            .arg("sandbox")
            .arg("--piped")
            .arg(format!("--force-admin-ports={admin_port}"))
            .arg("run")
            .arg("--last")
            .stdin(Stdio::piped())
            .spawn()
            .context("Failed to start process to run Holochain sandbox")?;

        log::trace!("Passing password to running sandbox");
        run_sandbox_handle
            .stdin
            .as_mut()
            .context("Failed to get stdin for the process running Holochain sandbox")?
            .write_all(password.as_bytes())
            .context("Failed to write the passcode to the process running the sandbox")?;

        std::thread::sleep(Duration::from_secs(5));

        Ok(Self { run_sandbox_handle })
    }
}

impl Drop for HolochainSandbox {
    fn drop(&mut self) {
        log::trace!("Killing the running sandbox");

        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, SIGINT};
            use nix::unistd::Pid;

            let pid = self.run_sandbox_handle.id();
            kill(Pid::from_raw(pid as i32), SIGINT).expect("Failed to send SIGINT to `hc` process");
        }

        // TODO: Does just killing the child process work on Windows? It doesn't on Linux.
        #[cfg(windows)]
        self.run_sandbox_handle
            .kill()
            .expect("Failed to kill `hc` process");

        log::trace!("Waiting for the running sandbox to exit");
        self.run_sandbox_handle
            .wait()
            .expect("Failed to wait for the `hc` process to exit");

        log::trace!("Successfully killed the Holochain sandbox");
    }
}
