use std::{
    io::Write,
    process::{Child, Command, Stdio},
    time::Duration,
};

#[derive(Debug)]
pub struct HolochainSandbox {
    run_sandbox_handle: Child,
}

impl HolochainSandbox {
    /// Cleans existing sandboxes, creates a new one and runs it, storing the [`Child`] process
    /// internally so it can be gracefully shutdown on [`Drop::drop`].
    pub fn clean_create_run() -> Self {
        log::trace!("Cleaning sandbox");
        Command::new("hc")
            .arg("sandbox")
            .arg("clean")
            .spawn()
            .expect("Failed to run 'hc sandbox clean'")
            .wait()
            .expect("Failed to clean the Holochain sandbox");

        log::trace!("Creating new sandbox");
        let mut create_sandbox = Command::new("hc")
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
            .expect("Failed to run 'hc sandbox --piped create'");

        log::trace!("Passing passcode to new sandbox");
        create_sandbox
            .stdin
            .as_mut()
            .expect("Failed to get stdin for the process creating the sandbox")
            .write_all(b"1234\n")
            .expect("Failed to write the passcode to the process creating the sandbox");

        create_sandbox
            .wait()
            .expect("Failed to create the Holochain sandbox");

        log::trace!("Running the sandbox");
        let mut run_sandbox_handle = Command::new("hc")
            .arg("sandbox")
            .arg("--piped")
            .arg("--force-admin-ports=8888")
            .arg("run")
            .stdin(Stdio::piped())
            .spawn()
            .expect("Failed to start process to run Holochain sandbox");

        log::trace!("Passing passcode to running sandbox");
        run_sandbox_handle
            .stdin
            .as_mut()
            .expect("Failed to get stdin for the process running Holochain sandbox")
            .write_all(b"1234\n")
            .expect("Failed to write the passcode to the process running the sandbox");

        std::thread::sleep(Duration::from_secs(5));

        Self { run_sandbox_handle }
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
