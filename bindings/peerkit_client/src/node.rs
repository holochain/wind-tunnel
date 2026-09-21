use crate::event::{PeerStatus, PeerkitEvent, parse_line, short_agent_id};
use crate::stream::{ReplTokenizer, Token};
use anyhow::{Context, bail};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandKind {
    Connect,
    Disconnect,
    Peers,
}

enum CommandResponse {
    Connect(Option<PeerkitEvent>),
    Disconnect(Option<PeerkitEvent>),
    Peers(Vec<PeerInfo>),
}

#[derive(Debug, Default)]
struct NodeState {
    agent_id: Option<String>,
    relay_connected_at: Option<Instant>,
    /// Seconds from relay connection to each `[Peer discovered]` event.
    discovery_times_s: Vec<f64>,
    discovered: HashSet<String>,
    /// alias -> latest known peer info, refreshed by `peers` output and
    /// connect/disconnect events.
    peers: HashMap<String, PeerInfo>,
    /// Completed commands, including the startup prompt. Event redraws
    /// never change this counter, even when split across stdout reads.
    commands_completed: u64,
    /// Event lines whose following redraw prompts have not arrived yet.
    pending_event_prompts: u64,
    last_connect: Option<PeerkitEvent>,
    last_disconnect: Option<PeerkitEvent>,
    exited: bool,
}

fn prepare_command(state: &mut NodeState, kind: CommandKind) -> u64 {
    match kind {
        CommandKind::Connect => state.last_connect = None,
        CommandKind::Disconnect => state.last_disconnect = None,
        CommandKind::Peers => state.peers.clear(),
    }
    state.commands_completed
}

fn finish_command(state: &mut NodeState, kind: CommandKind) -> CommandResponse {
    match kind {
        CommandKind::Connect => CommandResponse::Connect(state.last_connect.take()),
        CommandKind::Disconnect => CommandResponse::Disconnect(state.last_disconnect.take()),
        CommandKind::Peers => CommandResponse::Peers(state.peers.values().cloned().collect()),
    }
}

/// A `peerkit node` child process driven over its stdin/stdout REPL.
#[derive(Debug)]
pub struct PeerkitNode {
    agent_id: String,
    child: Mutex<Child>,
    stdin: Mutex<ChildStdin>,
    /// Serialises commands so each completion prompt can be attributed to
    /// the one command that was waiting for it.
    command_lock: Mutex<()>,
    /// Latched when a command did not complete: a timeout, a cancellation, or
    /// a write error after the command may have reached the CLI. Its late
    /// prompt would be attributed to the next command, so no further command
    /// is allowed.
    desynced: AtomicBool,
    /// The stdout reader is joined during shutdown so all buffered output is
    /// applied to state before teardown drains it.
    reader: Mutex<Option<tokio::task::JoinHandle<()>>>,
    state: Arc<(Mutex<NodeState>, Notify)>,
    reporter: Arc<Reporter>,
}

/// Latches [`PeerkitNode::desynced`] unless the command it guards completes.
struct CommandGuard<'a> {
    desynced: &'a AtomicBool,
    completed: bool,
}

impl Drop for CommandGuard<'_> {
    fn drop(&mut self) {
        if !self.completed {
            self.desynced.store(true, Ordering::Release);
        }
    }
}

impl PeerkitNode {
    /// Spawn the CLI, wait for the startup banner and prompt, and return the
    /// running node.
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
        let mut stdout = child.stdout.take().expect("stdout is piped");
        let stdin = child.stdin.take().expect("stdin is piped");

        let state: Arc<(Mutex<NodeState>, Notify)> = Arc::default();
        let reader_state = state.clone();
        let reader = tokio::spawn(async move {
            let mut tokenizer = ReplTokenizer::new();
            let mut chunk = vec![0u8; 64 * 1024];
            loop {
                let read = match stdout.read(&mut chunk).await {
                    Ok(0) | Err(_) => break,
                    Ok(read) => read,
                };
                let tokens = tokenizer.feed(&chunk[..read]);
                if tokens.is_empty() {
                    continue;
                }
                let mut guard = reader_state.0.lock().await;
                for token in tokens {
                    match token {
                        Token::Prompt if guard.pending_event_prompts > 0 => {
                            guard.pending_event_prompts -= 1;
                        }
                        Token::Prompt => guard.commands_completed += 1,
                        Token::Line { text, event } => {
                            if event {
                                guard.pending_event_prompts += 1;
                            }
                            let Some(event) = parse_line(&text) else {
                                continue;
                            };
                            if let PeerkitEvent::Other(content) = &event {
                                log::debug!("peerkit stdout: {content}");
                            }
                            apply_event(&mut guard, event);
                        }
                    }
                }
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
            command_lock: Mutex::new(()),
            desynced: AtomicBool::new(false),
            reader: Mutex::new(Some(reader)),
            state,
            reporter,
        };
        // The startup prompt is the CLI's first "completion"; waiting for it
        // keeps it from being mistaken for the first command's prompt.
        node.wait_for(Duration::from_secs(60), |state| {
            state.agent_id.is_some() && state.commands_completed >= 1
        })
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
        let mut stdin = self.stdin.lock().await;
        stdin
            .write_all(format!("{command}\n").as_bytes())
            .await
            .context("failed to write to peerkit stdin")?;
        stdin.flush().await.context("failed to flush peerkit stdin")
    }

    /// Write one REPL command and wait until the CLI has finished running it.
    ///
    /// The CLI reprints its prompt once a command's handler has run to
    /// completion, so the prompt is the completion record for every command.
    /// For `peers` it is the only way to know that every row has been
    /// printed. Commands are serialised so each prompt can be attributed to
    /// the one command waiting for it; a command that does not complete
    /// (timeout, cancellation, failed write) marks the node unusable because
    /// its late prompt could no longer be told apart from the next command's.
    ///
    /// # Errors
    ///
    /// Returns an error when the command contains a line break, when an
    /// earlier command left the node unusable, when the write fails, when the
    /// CLI exits, or when no completion arrives within `timeout`.
    async fn run_command(
        &self,
        kind: CommandKind,
        command: &str,
        timeout: Duration,
    ) -> anyhow::Result<CommandResponse> {
        if command.contains('\n') || command.contains('\r') {
            bail!("peerkit REPL command contains a line break");
        }
        let name = command.split_whitespace().next().unwrap_or("");
        if self.desynced.load(Ordering::Acquire) {
            bail!("peerkit node is unusable after a command did not complete");
        }
        let _serial = self.command_lock.lock().await;
        if self.desynced.load(Ordering::Acquire) {
            bail!("peerkit node is unusable after a command did not complete");
        }
        let before = {
            let mut state = self.state.0.lock().await;
            prepare_command(&mut state, kind)
        };
        let mut guard = CommandGuard {
            desynced: &self.desynced,
            completed: false,
        };
        self.write_command(command).await?;
        self.wait_for(timeout, |state| state.commands_completed > before)
            .await
            .with_context(|| format!("no completion for `{name}` command"))?;
        guard.completed = true;
        let response = {
            let mut state = self.state.0.lock().await;
            finish_command(&mut state, kind)
        };
        Ok(response)
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
                let response = self
                    .run_command(CommandKind::Peers, "peers", Duration::from_secs(10))
                    .await?;
                let CommandResponse::Peers(peers) = response else {
                    unreachable!("peers command returned a non-peers response");
                };
                if let Some(alias) = peers
                    .into_iter()
                    .find(|peer| peer.short_agent_id == wanted)
                    .map(|peer| peer.alias)
                {
                    return Ok::<_, anyhow::Error>(alias);
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        })
        .await
        .with_context(|| format!("could not resolve alias for {agent_id}"))?
    }

    /// Connect to a discovered peer by alias.
    ///
    /// A peer that is already connected, for example because it dialled this
    /// node first, is treated as success: the requested state is reached.
    #[wind_tunnel_instrument]
    pub async fn connect(&self, alias: &str) -> anyhow::Result<()> {
        let response = self
            .run_command(
                CommandKind::Connect,
                &format!("conn {alias}"),
                Duration::from_secs(30),
            )
            .await?;
        let CommandResponse::Connect(response) = response else {
            unreachable!("connect command returned a non-connect response");
        };
        match response {
            Some(PeerkitEvent::ConnectSucceeded { .. }) => Ok(()),
            Some(PeerkitEvent::ConnectFailed { reason, .. })
                if reason.contains("Already connected to") =>
            {
                Ok(())
            }
            Some(PeerkitEvent::ConnectFailed { reason, .. }) => bail!("connect failed: {reason}"),
            _ => bail!("no response to conn command"),
        }
    }

    /// Disconnect from a connected peer by alias.
    ///
    /// A peer that is not connected any more, for example because it hung up
    /// first, is treated as success: the requested state is reached.
    #[wind_tunnel_instrument]
    pub async fn disconnect(&self, alias: &str) -> anyhow::Result<()> {
        let response = self
            .run_command(
                CommandKind::Disconnect,
                &format!("dsct {alias}"),
                Duration::from_secs(30),
            )
            .await?;
        let CommandResponse::Disconnect(response) = response else {
            unreachable!("disconnect command returned a non-disconnect response");
        };
        match response {
            Some(PeerkitEvent::DisconnectSucceeded { .. }) => Ok(()),
            Some(PeerkitEvent::DisconnectFailed { reason, .. })
                if reason.contains("Not connected to") =>
            {
                Ok(())
            }
            Some(PeerkitEvent::DisconnectFailed { reason, .. }) => {
                bail!("disconnect failed: {reason}")
            }
            _ => bail!("no response to dsct command"),
        }
    }

    /// Refresh and return the peer table by running the `peers` command.
    ///
    /// The table is discarded before the command is sent, so the returned
    /// snapshot holds only the rows this poll produced. The CLI prints every
    /// row before its completion prompt, so the snapshot is complete once the
    /// command is acknowledged. Rows for peers that have expired from the
    /// CLI's agent store are never removed by the CLI, so departed peers keep
    /// showing as `[not connected]`.
    pub async fn list_peers(&self) -> anyhow::Result<Vec<PeerInfo>> {
        let response = self
            .run_command(CommandKind::Peers, "peers", Duration::from_secs(10))
            .await?;
        let CommandResponse::Peers(peers) = response else {
            unreachable!("peers command returned a non-peers response");
        };
        Ok(peers)
    }

    /// Drain discovery times (seconds from relay connection to each peer
    /// discovery) recorded since the last call.
    pub async fn take_discovery_times(&self) -> Vec<f64> {
        std::mem::take(&mut self.state.0.lock().await.discovery_times_s)
    }

    /// Ask the CLI to exit and wait for the process to stop.
    pub async fn shutdown(&self) -> anyhow::Result<()> {
        let _serial = self.command_lock.lock().await;
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
        drop(child);
        if let Some(reader) = self.reader.lock().await.take() {
            reader.await.context("peerkit stdout reader task failed")?;
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
            let status = state.peers.get(&alias).and_then(|peer| match peer.status {
                Some(status @ (PeerStatus::Direct | PeerStatus::Relayed)) => Some(status),
                _ => None,
            });
            state.peers.insert(
                alias.clone(),
                PeerInfo {
                    alias,
                    short_agent_id: short,
                    status,
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
            state.peers.insert(
                alias.clone(),
                PeerInfo {
                    alias,
                    short_agent_id,
                    status,
                },
            );
        }
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
        // This client never sends, so message and send-failure lines carry
        // nothing it tracks. They are still parsed so that they are counted
        // as event lines and never mistaken for a command response.
        PeerkitEvent::MessageReceived { .. }
        | PeerkitEvent::SendFailed { .. }
        | PeerkitEvent::Other(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_response_is_taken_while_its_command_is_still_serialized() {
        let mut state = NodeState {
            last_connect: Some(PeerkitEvent::ConnectSucceeded {
                alias: "1".to_string(),
            }),
            ..NodeState::default()
        };

        let response = finish_command(&mut state, CommandKind::Connect);

        assert!(matches!(
            response,
            CommandResponse::Connect(Some(PeerkitEvent::ConnectSucceeded { alias }))
                if alias == "1"
        ));
        assert!(state.last_connect.is_none());
    }

    #[test]
    fn delayed_peer_connected_event_preserves_known_connection_status() {
        let mut state = NodeState::default();
        apply_event(
            &mut state,
            PeerkitEvent::PeersEntry {
                alias: "1".to_string(),
                short_agent_id: "aaaa…aaaa".to_string(),
                status: Some(PeerStatus::Direct),
            },
        );

        apply_event(
            &mut state,
            PeerkitEvent::PeerConnected {
                alias: "1".to_string(),
                agent_id: "a".repeat(64),
            },
        );

        assert_eq!(
            state.peers.get("1").and_then(|peer| peer.status),
            Some(PeerStatus::Direct)
        );
    }

    #[test]
    fn incomplete_command_guard_marks_the_node_unusable() {
        let desynced = AtomicBool::new(false);
        {
            let _guard = CommandGuard {
                desynced: &desynced,
                completed: false,
            };
        }
        assert!(desynced.load(Ordering::Acquire));

        let desynced = AtomicBool::new(false);
        {
            let mut guard = CommandGuard {
                desynced: &desynced,
                completed: false,
            };
            guard.completed = true;
        }
        assert!(!desynced.load(Ordering::Acquire));
    }
}
