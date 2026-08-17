use crate::{PeerkitAgentContext, PeerkitRunnerContext};
use anyhow::{Context as _, bail, ensure};
use peerkit_client_instrumented::{PeerkitNode, PeerkitNodeConfig};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
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

/// Derive a deterministic Ed25519 identity for `(run_id, behaviour)`.
///
/// Returns the raw 32-byte private key seed and the agent ID (lowercase hex of
/// the public key) that the Peerkit CLI will report for it. Because the
/// derivation only depends on the run ID and the behaviour name, every agent in
/// a run can compute every other behaviour's agent ID without communication.
/// Consequence: at most ONE agent per behaviour, or identities collide.
pub fn derive_identity(run_id: &str, behaviour: &str) -> ([u8; 32], String) {
    let mut hasher = Sha3_256::new();
    hasher.update(b"wind-tunnel-peerkit-identity");
    hasher.update(run_id.as_bytes());
    hasher.update(b":");
    hasher.update(behaviour.as_bytes());
    let seed: [u8; 32] = hasher.finalize().into();
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
    let agent_id = hex::encode(signing_key.verifying_key().to_bytes());
    (seed, agent_id)
}

/// The agent ID that [derive_identity] produces for the given behaviour in the
/// current run.
pub fn agent_id_for_behaviour(
    ctx: &AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
    behaviour: &str,
) -> String {
    derive_identity(ctx.runner_context().get_run_id(), behaviour).1
}

fn write_identity_file(run_id: &str, behaviour: &str, seed: &[u8; 32]) -> anyhow::Result<PathBuf> {
    let dir = std::env::temp_dir().join("wind-tunnel-peerkit");
    std::fs::create_dir_all(&dir).context("failed to create identity dir")?;
    let path = dir.join(format!("{run_id}-{behaviour}.key"));
    std::fs::write(&path, seed).context("failed to write identity key")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .context("failed to set identity key permissions")?;
    }
    Ok(path)
}

/// Tracks `(run_id, behaviour)` pairs that have already claimed a derived
/// identity in this process, so a second agent assigned the same behaviour is
/// rejected instead of silently colliding with the first (see
/// [derive_identity]).
fn claimed_identities() -> &'static Mutex<HashSet<String>> {
    static CLAIMED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    CLAIMED.get_or_init(|| Mutex::new(HashSet::new()))
}

fn claim_identity_slot(run_id: &str, behaviour: &str) -> anyhow::Result<()> {
    let key = format!("{run_id}:{behaviour}");
    let mut claimed = claimed_identities()
        .lock()
        .expect("claimed identities mutex poisoned");
    ensure!(
        claimed.insert(key),
        "more than one agent was assigned the {behaviour} behaviour in this run — \
         Peerkit identities are derived from (run_id, behaviour) alone, so only one \
         agent per behaviour is supported"
    );
    Ok(())
}

/// Spawn a `peerkit node` for this agent and wait until it is connected to the
/// relay. The node identity is derived from the run ID and the agent's
/// assigned behaviour.
pub fn start_node(ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>) -> HookResult {
    if ctx.get().node.is_some() {
        bail!("start_node: node already started");
    }
    let relay_dial_addrs = get_relay_dial_addrs(ctx)?;
    let run_id = ctx.runner_context().get_run_id().to_string();
    let behaviour = ctx.assigned_behaviour().to_string();
    claim_identity_slot(&run_id, &behaviour)?;
    let (seed, expected_agent_id) = derive_identity(&run_id, &behaviour);
    let identity_path = write_identity_file(&run_id, &behaviour, &seed)?;
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

/// Wait for the target agent to be discovered, resolve its alias and connect.
/// Returns the alias for use with [send_text].
pub fn connect_to_agent(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
    target_agent_id: &str,
    timeout: Duration,
) -> anyhow::Result<String> {
    let node = ctx.get().node();
    let target = target_agent_id.to_string();
    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            node.wait_for_peer_discovered(&target, timeout).await?;
            let alias = node.request_alias(&target, timeout).await?;
            node.connect(&alias).await?;
            Ok(alias)
        })
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
) -> anyhow::Result<Vec<(String, String)>> {
    let node = ctx.get().node();
    ctx.runner_context()
        .executor()
        .execute_in_place(async move { Ok(node.take_messages().await) })
}

/// Agent teardown hook: stop the `peerkit node` process.
pub fn shutdown_node(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
) -> HookResult {
    if let Some(node) = ctx.get_mut().node.take() {
        ctx.runner_context()
            .executor()
            .execute_in_place(async move { node.shutdown().await })?;
    }
    Ok(())
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
    fn identity_is_deterministic_and_behaviour_specific() {
        let (seed_a, id_a) = derive_identity("run-1", "initiator");
        let (seed_a2, id_a2) = derive_identity("run-1", "initiator");
        let (seed_b, id_b) = derive_identity("run-1", "responder");
        let (_, id_other_run) = derive_identity("run-2", "initiator");

        assert_eq!(seed_a, seed_a2);
        assert_eq!(id_a, id_a2);
        assert_ne!(seed_a, seed_b);
        assert_ne!(id_a, id_b);
        assert_ne!(id_a, id_other_run);
        assert_eq!(id_a.len(), 64);
        assert!(id_a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn connection_string_round_trips() {
        let addrs = vec!["/ip4/1.2.3.4/udp/9000/webrtc-direct".to_string()];
        let connection_string = to_connection_string(addrs.clone());
        let parsed: PeerkitConnection = serde_json::from_str(&connection_string).unwrap();
        assert_eq!(parsed.relay_dial_addrs, addrs);
    }
}
