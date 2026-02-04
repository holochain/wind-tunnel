use holochain_types::prelude::{ActionHashB64, AgentPubKeyB64};
use holochain_wind_tunnel_runner::prelude::*;
use tokio::time::Instant;

#[derive(Debug, Default)]
pub struct ScenarioValues {
    pub session_start_time: Option<Instant>,
    pub network_initialized: bool,
    pub participating_agents: Vec<AgentPubKeyB64>,
    pub executor_pubkey: Option<AgentPubKeyB64>,
    pub smart_agreement_hash: Option<ActionHashB64>,
    pub progenitor_agent_pubkey: Option<AgentPubKeyB64>,
    // pub signal_tx: Option<tokio::sync::broadcast::Sender<Signal>>,
    // pub initiate_with_peers: Vec<AgentPubKey>,
    // pub session_attempts: Arc<AtomicUsize>,
    // pub session_successes: Arc<AtomicUsize>,
    // pub session_failures: Arc<AtomicUsize>,
}

impl UserValuesConstraint for ScenarioValues {}

pub fn setup(_ctx: &mut RunnerContext<HolochainRunnerContext>) -> HookResult {
    log::info!("Setting up unyt scenario");
    // TODO: Just remove the setup I guess?
    log::info!("Unyt scenario setup complete");
    Ok(())
}
