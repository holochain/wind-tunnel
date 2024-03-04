use holochain_client_instrumented::prelude::{AppAgentWebsocket};
use wind_tunnel_runner::prelude::UserValuesConstraint;

#[derive(Default, Debug)]
pub struct HolochainAgentContext {
    pub app_agent_client: Option<AppAgentWebsocket>,
    pub installed_app_id: Option<String>,
}

impl UserValuesConstraint for HolochainAgentContext {}
