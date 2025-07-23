use holo_hash::AgentPubKey;
use wind_tunnel_runner::prelude::UserValuesConstraint;

/// Holochain-specific context values for the [wind_tunnel_runner::prelude::RunnerContext].
#[derive(Default, Debug)]
pub struct HolochainRunnerContext {
    pub(crate) app_ws_url: Option<String>,
    pub progenitor_agent_pubkey: Option<AgentPubKey>,
}

impl UserValuesConstraint for HolochainRunnerContext {}

impl HolochainRunnerContext {
    /// Get the `app_ws_url` that was configured during setup.
    pub fn app_ws_url(&self) -> String {
        self.app_ws_url.clone().expect(
            "app_port is not set, did you forget to call `configure_app_port` in your setup?",
        )
    }

    pub fn progenitor_agent_pubkey(&self) -> AgentPubKey {
        self.progenitor_agent_pubkey
            .clone()
            .expect("progenitor_agent_pubkey is not set, did you forget to call `configure_progenitor_agent_pubkey` in your setup?")
    }
}
