use crate::{PeerkitAgentContext, PeerkitRunnerContext};
use anyhow::{Context as _, bail, ensure};
use peerkit_client_instrumented::{PeerInfo, PeerkitNode, PeerkitNodeConfig, ReceivedMessage};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use wind_tunnel_runner::prelude::{
    AgentContext, HookResult, ScenarioDefinitionBuilder, WindTunnelResult,
};

use crate::bin_path::peerkit_bin_path;

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct PeerkitConnection {
    pub(crate) relay_dial_addrs: Vec<String>,
}

/// Parse the CLI `connection-string` back into relay dial addresses.
pub fn get_relay_dial_addrs(
    ctx: &AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
) -> anyhow::Result<Vec<String>> {
    let connection_string = ctx
        .runner_context()
        .get_connection_string()
        .expect("connection-string is empty even though it is required");
    let connection = serde_json::from_str::<PeerkitConnection>(connection_string)
        .context("failed to parse relay dial addresses from connection string")?;
    Ok(connection.relay_dial_addrs)
}

/// Pack relay dial addresses into the framework's single connection string.
pub fn to_connection_string(relay_dial_addrs: Vec<String>) -> String {
    serde_json::to_string(&PeerkitConnection { relay_dial_addrs })
        .expect("failed to serialize relay dial addresses")
}

/// Generate a random Ed25519 identity.
///
/// Returns the raw 32-byte private key seed and the agent ID (lowercase hex
/// of the public key) that the Peerkit CLI will report for it. Identities are
/// random because agents connect to whichever peers they discover — no agent
/// needs to predict another's ID, so any number of agents may share a
/// behaviour.
fn generate_identity() -> ([u8; 32], String) {
    let seed: [u8; 32] = rand::random();
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
    let agent_id = hex::encode(signing_key.verifying_key().to_bytes());
    (seed, agent_id)
}

/// Write an agent's private key seed to a file that only its owner can read.
///
/// The file is created with its restrictive mode already applied rather than
/// narrowed afterwards, so the key is never readable by other users, not even
/// for the instant between creation and the permission change.
fn write_identity_file(agent_id: &str, seed: &[u8; 32]) -> anyhow::Result<PathBuf> {
    use std::io::Write as _;

    let dir = std::env::temp_dir().join("wind-tunnel-peerkit");
    std::fs::create_dir_all(&dir).context("failed to create identity dir")?;
    let path = dir.join(format!("{agent_id}.key"));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options
        .open(&path)
        .context("failed to create identity key")?;
    file.write_all(seed)
        .context("failed to write identity key")?;
    Ok(path)
}

/// Spawn a `peerkit node` for this agent and wait until it is connected to the
/// relay. The node is given a random identity, since agents discover and
/// connect to peers dynamically rather than predicting each other's IDs.
pub fn start_node(ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>) -> HookResult {
    if ctx.get().node.is_some() {
        bail!("start_node: node already started");
    }
    let relay_dial_addrs = get_relay_dial_addrs(ctx)?;
    let (seed, expected_agent_id) = generate_identity();
    let identity_path = write_identity_file(&expected_agent_id, &seed)?;
    ctx.get_mut().identity_path = Some(identity_path.clone());
    let peerkit_bin = peerkit_bin_path()?;
    let reporter = ctx.runner_context().reporter();
    let node = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move {
            let node = PeerkitNode::start(
                PeerkitNodeConfig {
                    peerkit_bin,
                    relay_dial_addrs,
                    identity_path,
                },
                reporter,
            )
            .await?;
            node.wait_for_relay(Duration::from_secs(60)).await?;
            Ok(node)
        })?;
    let reported_agent_id = node.agent_id().to_string();
    ensure!(
        reported_agent_id == expected_agent_id,
        "peerkit reported agent ID {reported_agent_id} but {expected_agent_id} was derived — identity file mismatch"
    );
    ctx.get_mut().node = Some(Arc::new(node));
    Ok(())
}

/// Connect to a discovered peer by its CLI alias.
pub fn connect_to_alias(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
    alias: &str,
) -> anyhow::Result<()> {
    let node = ctx.get().node();
    let alias = alias.to_string();
    ctx.runner_context()
        .executor()
        .execute_in_place(async move { node.connect(&alias).await })
}

/// Disconnect from a connected peer by its CLI alias.
pub fn disconnect_from_alias(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
    alias: &str,
) -> anyhow::Result<()> {
    let node = ctx.get().node();
    let alias = alias.to_string();
    ctx.runner_context()
        .executor()
        .execute_in_place(async move { node.disconnect(&alias).await })
}

/// Refresh and return the peer table for this agent's node.
pub fn list_peers(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
) -> anyhow::Result<Vec<PeerInfo>> {
    let node = ctx.get().node();
    ctx.runner_context()
        .executor()
        .execute_in_place(async move { node.list_peers().await })
}

/// Send a text message to a connected peer by alias.
pub fn send_text(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
    alias: &str,
    text: &str,
) -> anyhow::Result<()> {
    let node = ctx.get().node();
    let alias = alias.to_string();
    let text = text.to_string();
    ctx.runner_context()
        .executor()
        .execute_in_place(async move { node.send_text(&alias, &text).await })
}

/// Drain messages received by this agent since the last call.
pub fn take_received_messages(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
) -> anyhow::Result<Vec<ReceivedMessage>> {
    let node = ctx.get().node();
    ctx.runner_context()
        .executor()
        .execute_in_place(async move { Ok(node.take_messages().await) })
}

/// Drain the discovery times recorded by this agent's node.
pub fn take_discovery_times(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
) -> anyhow::Result<Vec<f64>> {
    let node = ctx.get().node();
    ctx.runner_context()
        .executor()
        .execute_in_place(async move { Ok(node.take_discovery_times().await) })
}

/// Drain the count of asynchronous send failures on this agent's node.
pub fn take_send_failures(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
) -> anyhow::Result<u64> {
    let node = ctx.get().node();
    ctx.runner_context()
        .executor()
        .execute_in_place(async move { Ok(node.take_send_failures().await) })
}

/// Agent teardown hook: stop the `peerkit node` process and remove its
/// identity key file.
///
/// Deleting the key is best effort: a failure is logged rather than failing
/// the teardown, since the node itself has already been stopped by then.
pub fn shutdown_node(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
) -> HookResult {
    let shutdown = match ctx.get_mut().node.take() {
        Some(node) => ctx
            .runner_context()
            .executor()
            .execute_in_place(async move { node.shutdown().await }),
        None => Ok(()),
    };
    // Remove the key even if the node refused to stop, so it cannot outlive
    // the agent either way.
    if let Some(identity_path) = ctx.get_mut().identity_path.take()
        && let Err(e) = std::fs::remove_file(&identity_path)
    {
        log::warn!(
            "failed to remove identity key {path}: {e}",
            path = identity_path.display()
        );
    }
    shutdown
}

/// Run a Peerkit scenario with the WindTunnel runner.
pub fn run(
    definition: ScenarioDefinitionBuilder<PeerkitRunnerContext, PeerkitAgentContext>,
) -> WindTunnelResult<usize> {
    wind_tunnel_runner::prelude::run(definition)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_identities_are_unique_and_valid() {
        let (seed_a, id_a) = generate_identity();
        let (seed_b, id_b) = generate_identity();
        assert_ne!(seed_a, seed_b);
        assert_ne!(id_a, id_b);
        assert_eq!(id_a.len(), 64);
        assert!(id_a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[cfg(unix)]
    #[test]
    fn identity_files_are_created_private() {
        use std::os::unix::fs::PermissionsExt as _;

        let (seed, agent_id) = generate_identity();
        let path = write_identity_file(&agent_id, &seed).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn connection_string_round_trips() {
        let addrs = vec!["/ip4/1.2.3.4/udp/9000/webrtc-direct".to_string()];
        let connection_string = to_connection_string(addrs.clone());
        let parsed: PeerkitConnection = serde_json::from_str(&connection_string).unwrap();
        assert_eq!(parsed.relay_dial_addrs, addrs);
    }
}
