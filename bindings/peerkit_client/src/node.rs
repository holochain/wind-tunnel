use crate::event::{PeerkitEvent, parse_line, short_agent_id};
use anyhow::{Context, bail};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{Mutex, Notify};
use wind_tunnel_instruments::prelude::Reporter;
use wind_tunnel_instruments_derive::wind_tunnel_instrument;

/// Configuration for spawning a `peerkit node` process.
#[derive(Debug, Clone)]
pub struct PeerkitNodeConfig {
    /// Path to the `peerkit` executable.
    pub peerkit_bin: PathBuf,
    /// One or more relay dial multiaddrs passed as positional arguments.
    pub relay_dial_addrs: Vec<String>,
    /// File holding the raw 32-byte Ed25519 private key (`PEERKIT_IDENTITY`).
    pub identity_path: PathBuf,
}

#[derive(Debug, Default)]
struct NodeState {
    agent_id: Option<String>,
    relay_connected: bool,
    discovered: HashSet<String>,
    /// short agent ID -> alias, refreshed by `peers` output.
    aliases: HashMap<String, String>,
    messages: Vec<(String, String)>,
    last_connect: Option<PeerkitEvent>,
    exited: bool,
}

/// A `peerkit node` child process driven over its stdin/stdout REPL.
#[derive(Debug)]
pub struct PeerkitNode {
    agent_id: String,
    child: Mutex<Child>,
    stdin: Mutex<ChildStdin>,
    state: Arc<(Mutex<NodeState>, Notify)>,
    reporter: Arc<Reporter>,
}

impl PeerkitNode {
    /// Spawn the CLI, wait for the startup line and return the running node.
    pub async fn start(config: PeerkitNodeConfig, reporter: Arc<Reporter>) -> anyhow::Result<Self> {
        let mut child = Command::new(&config.peerkit_bin)
            .arg("node")
            .args(&config.relay_dial_addrs)
            .env("PEERKIT_IDENTITY", &config.identity_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .context("failed to spawn `peerkit node`")?;
        let stdout = child.stdout.take().expect("stdout is piped");
        let stdin = child.stdin.take().expect("stdin is piped");

        let state: Arc<(Mutex<NodeState>, Notify)> = Arc::default();
        let reader_state = state.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let Some(event) = parse_line(&line) else {
                    continue;
                };
                if let PeerkitEvent::Other(content) = &event {
                    log::debug!("peerkit stdout: {content}");
                }
                let mut guard = reader_state.0.lock().await;
                apply_event(&mut guard, event);
                drop(guard);
                reader_state.1.notify_waiters();
            }
            reader_state.0.lock().await.exited = true;
            reader_state.1.notify_waiters();
        });

        let mut node = Self {
            agent_id: String::new(),
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            state,
            reporter,
        };
        node.wait_for(Duration::from_secs(60), |state| state.agent_id.is_some())
            .await
            .context("timed out waiting for peerkit session start")?;
        node.agent_id = node
            .state
            .0
            .lock()
            .await
            .agent_id
            .clone()
            .expect("agent id set by wait_for condition");
        Ok(node)
    }

    /// The full hex agent ID reported by the CLI at startup.
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    async fn wait_for<F>(&self, timeout: Duration, check: F) -> anyhow::Result<()>
    where
        F: Fn(&NodeState) -> bool,
    {
        tokio::time::timeout(timeout, async {
            loop {
                let notified = self.state.1.notified();
                tokio::pin!(notified);
                notified.as_mut().enable();
                {
                    let guard = self.state.0.lock().await;
                    if check(&guard) {
                        return Ok(());
                    }
                    if guard.exited {
                        bail!("peerkit node exited unexpectedly");
                    }
                }
                notified.await;
            }
        })
        .await
        .context("timed out")?
    }

    async fn write_command(&self, command: &str) -> anyhow::Result<()> {
        if command.contains('\n') || command.contains('\r') {
            bail!("peerkit REPL command contains a line break");
        }
        let mut stdin = self.stdin.lock().await;
        stdin
            .write_all(format!("{command}\n").as_bytes())
            .await
            .context("failed to write to peerkit stdin")?;
        stdin.flush().await.context("failed to flush peerkit stdin")
    }

    /// Wait until the node has a circuit address on the relay.
    pub async fn wait_for_relay(&self, timeout: Duration) -> anyhow::Result<()> {
        self.wait_for(timeout, |state| state.relay_connected)
            .await
            .context("relay connection not established")
    }

    /// Wait until the given full agent ID has been discovered via the relay.
    pub async fn wait_for_peer_discovered(
        &self,
        agent_id: &str,
        timeout: Duration,
    ) -> anyhow::Result<()> {
        self.wait_for(timeout, |state| state.discovered.contains(agent_id))
            .await
            .with_context(|| format!("peer {agent_id} not discovered"))
    }

    /// Resolve the CLI alias for a discovered peer by polling the `peers`
    /// command and matching the truncated agent ID.
    pub async fn request_alias(&self, agent_id: &str, timeout: Duration) -> anyhow::Result<String> {
        let wanted = short_agent_id(agent_id);
        tokio::time::timeout(timeout, async {
            loop {
                if let Some(alias) = self.state.0.lock().await.aliases.get(&wanted) {
                    return Ok::<_, anyhow::Error>(alias.clone());
                }
                self.write_command("peers").await?;
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        })
        .await
        .with_context(|| format!("could not resolve alias for {agent_id}"))?
    }

    /// Connect to a discovered peer by alias.
    #[wind_tunnel_instrument]
    pub async fn connect(&self, alias: &str) -> anyhow::Result<()> {
        self.state.0.lock().await.last_connect = None;
        self.write_command(&format!("conn {alias}")).await?;
        self.wait_for(Duration::from_secs(30), |state| {
            state.last_connect.is_some()
        })
        .await
        .context("no response to conn command")?;
        match self.state.0.lock().await.last_connect.clone() {
            Some(PeerkitEvent::ConnectSucceeded { .. }) => Ok(()),
            Some(PeerkitEvent::ConnectFailed { reason, .. }) => {
                bail!("connect failed: {reason}")
            }
            _ => bail!("no response to conn command"),
        }
    }

    /// Send a text message to a peer by alias.
    ///
    /// The CLI prints nothing on success, so this only measures the command
    /// dispatch. Delivery is observed on the receiving side.
    #[wind_tunnel_instrument]
    pub async fn send_text(&self, alias: &str, text: &str) -> anyhow::Result<()> {
        self.write_command(&format!("send {alias} {text}")).await
    }

    /// Drain messages received since the last call. Pairs of (alias, text).
    pub async fn take_messages(&self) -> Vec<(String, String)> {
        std::mem::take(&mut self.state.0.lock().await.messages)
    }

    /// Ask the CLI to exit and wait for the process to stop.
    pub async fn shutdown(&self) -> anyhow::Result<()> {
        let _ = self.write_command("exit").await;
        let mut child = self.child.lock().await;
        match tokio::time::timeout(Duration::from_secs(10), child.wait()).await {
            Ok(status) => {
                status.context("failed to wait for peerkit node")?;
            }
            Err(_) => {
                log::warn!("peerkit node did not exit in time, killing it");
                child.kill().await.context("failed to kill peerkit node")?;
            }
        }
        Ok(())
    }
}

fn apply_event(state: &mut NodeState, event: PeerkitEvent) {
    match event {
        PeerkitEvent::SessionStarted { agent_id } => state.agent_id = Some(agent_id),
        PeerkitEvent::RelayConnected { .. } => state.relay_connected = true,
        PeerkitEvent::PeerDiscovered { agent_id } => {
            state.discovered.insert(agent_id);
        }
        PeerkitEvent::PeerConnected { alias, agent_id } => {
            state.discovered.insert(agent_id.clone());
            state.aliases.insert(short_agent_id(&agent_id), alias);
        }
        PeerkitEvent::PeersEntry {
            alias,
            short_agent_id,
        } => {
            state.aliases.insert(short_agent_id, alias);
        }
        PeerkitEvent::MessageReceived { alias, text } => state.messages.push((alias, text)),
        event @ (PeerkitEvent::ConnectSucceeded { .. } | PeerkitEvent::ConnectFailed { .. }) => {
            state.last_connect = Some(event)
        }
        PeerkitEvent::SendFailed { reason } => {
            log::warn!("peerkit send failed: {reason}");
        }
        PeerkitEvent::PeerDisconnected { .. } | PeerkitEvent::Other(_) => {}
    }
}
