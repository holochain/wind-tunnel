use std::sync::Arc;

use crate::{KitsuneAgentContext, KitsuneRunnerContext};
use anyhow::{Context, bail};
use kitsune_client_instrumented::WtChatter;
use serde::{Deserialize, Serialize};
use wind_tunnel_runner::prelude::{
    AgentContext, HookResult, ScenarioDefinitionBuilder, WindTunnelResult,
};

#[derive(Debug, Deserialize, Serialize)]
struct KitsuneServerUrls {
    bootstrap_server_url: String,
    signal_server_url: String,
}

/// Parse cli argument "connection-string" for bootstrap and signal server URLs and return
/// them separately.
pub fn get_server_urls(
    ctx: &AgentContext<KitsuneRunnerContext, KitsuneAgentContext>,
) -> anyhow::Result<(String, String)> {
    let connection_string = ctx
        .runner_context()
        .get_connection_string()
        .expect("connection-string is empty even though it is required");
    let connections = serde_json::from_str::<KitsuneServerUrls>(connection_string)
        .context("failed to parse bootstrap and server URL from connection string")?;
    Ok((
        connections.bootstrap_server_url,
        connections.signal_server_url,
    ))
}

/// Convert bootstrap and signal server URL into single connection string.
pub fn to_connection_string(bootstrap_server_url: String, signal_server_url: String) -> String {
    let server_urls = KitsuneServerUrls {
        bootstrap_server_url,
        signal_server_url,
    };
    serde_json::to_string(&server_urls)
        .expect("failed to convert bootstrap and signal server URLs to connection string")
}

/// Create a Kitsune chatter instance.
pub fn create_chatter(
    ctx: &mut AgentContext<KitsuneRunnerContext, KitsuneAgentContext>,
) -> HookResult {
    if ctx.get().chatter.is_some() {
        bail!("create_chatter: Chatter already created.");
    }
    let (bootstrap_server_url, signal_server_url) = get_server_urls(ctx)?;
    let space_id = ctx.runner_context().get_run_id();
    let reporter = ctx.runner_context().reporter();
    let chatter = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move {
            WtChatter::create(
                &bootstrap_server_url,
                &signal_server_url,
                space_id,
                reporter,
            )
            .await
        })?;
    ctx.get_mut().chatter = Some(Arc::new(chatter));
    Ok(())
}

/// Return chatter id.
pub fn chatter_id(ctx: &mut AgentContext<KitsuneRunnerContext, KitsuneAgentContext>) -> String {
    ctx.get()
        .chatter
        .clone()
        .expect("chatter_id: chatter is not created")
        .id()
        .to_string()
}

/// Join the chatter network.
pub fn join_chatter_network(
    ctx: &mut AgentContext<KitsuneRunnerContext, KitsuneAgentContext>,
) -> HookResult {
    let chatter = ctx.get().chatter();
    ctx.runner_context()
        .executor()
        .execute_in_place(async move { chatter.join_space().await })?;
    Ok(())
}

/// Send messages to peers.
pub fn say(
    ctx: &mut AgentContext<KitsuneRunnerContext, KitsuneAgentContext>,
    messages: Vec<String>,
) -> anyhow::Result<()> {
    let chatter = ctx.get().chatter();
    ctx.runner_context()
        .executor()
        .execute_in_place(async move { chatter.say(messages).await })?;
    Ok(())
}

/// Run Kitsune scenario with WindTunnel runner.
pub fn run(
    definition: ScenarioDefinitionBuilder<KitsuneRunnerContext, KitsuneAgentContext>,
) -> WindTunnelResult<usize> {
    wind_tunnel_runner::prelude::run(definition)
}
