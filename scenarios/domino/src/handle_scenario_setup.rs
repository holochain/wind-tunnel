use anyhow::Context;
// use holochain_types::{prelude::AgentPubKey, signal::Signal};
use holochain_wind_tunnel_runner::prelude::*;
// use std::sync::atomic::AtomicUsize;
// use std::sync::Arc;
use tokio::time::Instant;

#[derive(Debug, Default)]
pub struct ScenarioValues {
    pub session_start_time: Option<Instant>,
    // pub signal_tx: Option<tokio::sync::broadcast::Sender<Signal>>,
    // pub initiate_with_peers: Vec<AgentPubKey>,
    // pub session_attempts: Arc<AtomicUsize>,
    // pub session_successes: Arc<AtomicUsize>,
    // pub session_failures: Arc<AtomicUsize>,
}

impl UserValuesConstraint for ScenarioValues {}

pub fn setup(ctx: &mut RunnerContext<HolochainRunnerContext>) -> HookResult {
    log::info!("Setting up domino scenario");
    configure_app_ws_url(ctx)?;

    // creating a progenitor agent pubkey that will be used for the DNA properties
    let admin_ws_url = ctx.get_connection_string().to_string();
    let reporter = ctx.reporter();
    let progenitor_agent_pubkey = ctx
        .executor()
        .execute_in_place(async move {
            log::debug!("Connecting a Holochain admin client: {}", admin_ws_url);
            let admin_client = AdminWebsocket::connect(admin_ws_url, reporter)
                .await
                .context("Unable to connect admin client")?;
            let agent_pubkey = admin_client
                .generate_agent_pub_key()
                .await
                .map_err(handle_api_err)?;
            Ok(agent_pubkey)
        })
        .context("Failed to set up app port")?;
    log::info!(
        "Generated progenitor agent pubkey: {:?}",
        progenitor_agent_pubkey
    );
    ctx.get_mut().progenitor_agent_pubkey = Some(progenitor_agent_pubkey);

    log::info!("Domino scenario setup complete");
    Ok(())
}
