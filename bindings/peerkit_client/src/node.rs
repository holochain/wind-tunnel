use crate::event::{PeerStatus, PeerkitEvent, parse_line, short_agent_id};
use anyhow::{Context, bail};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
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

/// One message received from a peer, as observed on this node's stdout.
#[derive(Debug, Clone)]
pub struct ReceivedMessage {
    /// CLI alias of the sender.
    pub alias: String,
    /// First 64 characters of the message text (enough for scenario headers;
    /// full payloads are not retained to bound memory).
    pub text_prefix: String,
    /// Full byte length of the message text.
    pub len: usize,
    /// When the message line was read from the CLI's stdout.
    pub received_at: Instant,
}

/// Snapshot of one row of the `peers` command output.
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// CLI alias assigned to the peer.
    pub alias: String,
    /// Truncated `first8…last4` form of the peer's agent ID.
    pub short_agent_id: String,
    /// `None` when connected but the type is not yet known (e.g. straight
    /// after a `[Peer connected]` event, before the next `peers` poll).
    pub status: Option<PeerStatus>,
}

#[derive(Debug, Default)]
struct NodeState {
    agent_id: Option<String>,
    relay_connected_at: Option<Instant>,
    /// Seconds from relay connection to each `[Peer discovered]` event.
    discovery_times_s: Vec<f64>,
    discovered: HashSet<String>,
    /// short agent ID -> alias, refreshed by `peers` output.
    aliases: HashMap<String, String>,
    /// alias -> latest known peer info, refreshed by `peers` output and
    /// connect/disconnect events.
    peers: HashMap<String, PeerInfo>,
    messages: Vec<ReceivedMessage>,
    /// Count of async `Send failed:` lines since the last drain.
    send_failures: u64,
    last_connect: Option<PeerkitEvent>,
    last_disconnect: Option<PeerkitEvent>,
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
        self.wait_for(timeout, |state| state.relay_connected_at.is_some())
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

    /// Disconnect from a connected peer by alias.
    #[wind_tunnel_instrument]
    pub async fn disconnect(&self, alias: &str) -> anyhow::Result<()> {
        self.state.0.lock().await.last_disconnect = None;
        self.write_command(&format!("dsct {alias}")).await?;
        self.wait_for(Duration::from_secs(30), |state| {
            state.last_disconnect.is_some()
        })
        .await
        .context("no response to dsct command")?;
        match self.state.0.lock().await.last_disconnect.clone() {
            Some(PeerkitEvent::DisconnectSucceeded { .. }) => Ok(()),
            Some(PeerkitEvent::DisconnectFailed { reason, .. }) => {
                bail!("disconnect failed: {reason}")
            }
            _ => bail!("no response to dsct command"),
        }
    }

    /// Refresh and return the peer table by running the `peers` command.
    ///
    /// The table is discarded before the command is sent, so the returned
    /// snapshot holds only the rows this poll produced rather than every peer
    /// ever seen with a possibly stale status.
    ///
    /// The CLI prints one row per peer with no terminator, so this waits a
    /// fixed 300ms for the rows to arrive before taking a snapshot. A poll
    /// that outruns that window therefore reports fewer peers than the CLI
    /// knows about. Rows for peers that have expired from the CLI's agent
    /// store are never removed by the CLI, so departed peers keep showing as
    /// `[not connected]`.
    pub async fn list_peers(&self) -> anyhow::Result<Vec<PeerInfo>> {
        self.state.0.lock().await.peers.clear();
        self.write_command("peers").await?;
        tokio::time::sleep(Duration::from_millis(300)).await;
        let state = self.state.0.lock().await;
        Ok(state.peers.values().cloned().collect())
    }

    /// Send a text message to a peer by alias.
    ///
    /// The CLI prints nothing on success, so this only measures the command
    /// dispatch. Delivery is observed on the receiving side.
    #[wind_tunnel_instrument]
    pub async fn send_text(&self, alias: &str, text: &str) -> anyhow::Result<()> {
        self.write_command(&format!("send {alias} {text}")).await
    }

    /// Drain messages received since the last call.
    pub async fn take_messages(&self) -> Vec<ReceivedMessage> {
        std::mem::take(&mut self.state.0.lock().await.messages)
    }

    /// Drain discovery times (seconds from relay connection to each peer
    /// discovery) recorded since the last call.
    pub async fn take_discovery_times(&self) -> Vec<f64> {
        std::mem::take(&mut self.state.0.lock().await.discovery_times_s)
    }

    /// Drain the count of asynchronous `Send failed:` lines since the last call.
    pub async fn take_send_failures(&self) -> u64 {
        std::mem::take(&mut self.state.0.lock().await.send_failures)
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
    let now = Instant::now();
    match event {
        PeerkitEvent::SessionStarted { agent_id } => state.agent_id = Some(agent_id),
        PeerkitEvent::RelayConnected { .. } => {
            state.relay_connected_at.get_or_insert(now);
        }
        PeerkitEvent::PeerDiscovered { agent_id } => {
            if state.discovered.insert(agent_id)
                && let Some(relay_at) = state.relay_connected_at
            {
                state
                    .discovery_times_s
                    .push(now.duration_since(relay_at).as_secs_f64());
            }
        }
        PeerkitEvent::PeerConnected { alias, agent_id } => {
            state.discovered.insert(agent_id.clone());
            let short = short_agent_id(&agent_id);
            state.aliases.insert(short.clone(), alias.clone());
            state.peers.insert(
                alias.clone(),
                PeerInfo {
                    alias,
                    short_agent_id: short,
                    status: None,
                },
            );
        }
        PeerkitEvent::PeerDisconnected { alias } => {
            if let Some(info) = state.peers.get_mut(&alias) {
                info.status = Some(PeerStatus::NotConnected);
            }
        }
        PeerkitEvent::PeersEntry {
            alias,
            short_agent_id,
            status,
        } => {
            state.aliases.insert(short_agent_id.clone(), alias.clone());
            state.peers.insert(
                alias.clone(),
                PeerInfo {
                    alias,
                    short_agent_id,
                    status,
                },
            );
        }
        PeerkitEvent::MessageReceived { alias, text } => state.messages.push(ReceivedMessage {
            alias,
            text_prefix: text.chars().take(64).collect(),
            len: text.len(),
            received_at: now,
        }),
        event @ (PeerkitEvent::ConnectSucceeded { .. } | PeerkitEvent::ConnectFailed { .. }) => {
            state.last_connect = Some(event)
        }
        event @ (PeerkitEvent::DisconnectSucceeded { .. }
        | PeerkitEvent::DisconnectFailed { .. }) => {
            if let PeerkitEvent::DisconnectSucceeded { alias } = &event
                && let Some(info) = state.peers.get_mut(alias)
            {
                info.status = Some(PeerStatus::NotConnected);
            }
            state.last_disconnect = Some(event)
        }
        PeerkitEvent::SendFailed { reason } => {
            log::warn!("peerkit send failed: {reason}");
            state.send_failures += 1;
        }
        PeerkitEvent::Other(_) => {}
    }
}
