use holochain_client_instrumented::prelude::AppAgentWebsocket;
use holochain_types::prelude::CellId;
use std::collections::HashMap;
use std::fmt::Debug;
use wind_tunnel_runner::prelude::UserValuesConstraint;

#[derive(Debug, Default)]
pub struct DefaultScenarioValues {
    pub values: HashMap<String, String>,
}

impl UserValuesConstraint for DefaultScenarioValues {}

/// Holochain-specific context values for the [wind_tunnel_runner::prelude::AgentContext].
#[derive(Default, Debug)]
pub struct HolochainAgentContext<T: UserValuesConstraint = DefaultScenarioValues> {
    pub(crate) installed_app_id: Option<String>,
    pub(crate) cell_id: Option<CellId>,
    pub(crate) app_agent_client: Option<AppAgentWebsocket>,
    pub scenario_values: T,
}

impl<T: UserValuesConstraint> UserValuesConstraint for HolochainAgentContext<T> {}

impl<T: UserValuesConstraint> HolochainAgentContext<T> {
    /// Get the `installed_app_id` that was configured during agent setup.
    pub fn installed_app_id(&self) -> String {
        self.installed_app_id.clone().expect("installed_app_id is not set, did you forget to call `install_app` in your agent_setup?")
    }

    /// Get the `cell_id` that was configured during agent setup.
    pub fn cell_id(&self) -> CellId {
        self.cell_id
            .clone()
            .expect("cell_id is not set, did you forget to call `install_app` in your agent_setup?")
    }

    /// Get the `app_agent_client` that was configured during agent setup.
    pub fn app_agent_client(&self) -> AppAgentWebsocket {
        self.app_agent_client.clone().expect("app_agent_client is not set, did you forget to call `install_app` in your agent_setup?")
    }
}
