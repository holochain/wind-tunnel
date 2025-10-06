use holochain_client_instrumented::prelude::AppWebsocket;
use holochain_types::prelude::CellId;
use std::fmt::Debug;
use std::{collections::HashMap, net::SocketAddr};
use wind_tunnel_runner::prelude::UserValuesConstraint;

use crate::holochain_runner::{HolochainConfig, HolochainConfigBuilder, HolochainRunner};

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
    pub(crate) app_client: Option<AppWebsocket>,
    pub(crate) app_ws_url: Option<SocketAddr>,
    pub(crate) admin_ws_url: Option<SocketAddr>,
    pub(crate) holochain_config: Option<HolochainConfigBuilder>,
    pub(crate) holochain_runner: Option<HolochainRunner>,
    pub scenario_values: T,
}

impl<T: UserValuesConstraint> UserValuesConstraint for HolochainAgentContext<T> {}

impl<T: UserValuesConstraint> HolochainAgentContext<T> {
    /// Get the `installed_app_id` that was configured during agent setup.
    pub fn installed_app_id(&self) -> anyhow::Result<String> {
        self.installed_app_id.clone().ok_or_else(|| anyhow::anyhow!("installed_app_id is not set, did you forget to call `install_app` in your agent_setup?"))
    }

    /// Get the `cell_id` that was configured during agent setup.
    pub fn cell_id(&self) -> CellId {
        self.cell_id
            .clone()
            .expect("cell_id is not set, did you forget to call `install_app` in your agent_setup?")
    }

    /// Get the `app_client` that was configured during agent setup.
    pub fn app_client(&self) -> AppWebsocket {
        self.app_client.clone().expect(
            "app_client is not set, did you forget to call `install_app` in your agent_setup?",
        )
    }

    /// Get the `admin_ws_url` that was configured during agent setup.
    pub fn admin_ws_url(&self) -> SocketAddr {
        self.admin_ws_url.expect(
            "admin_ws_url is not set, did you forget to call `configure_admin_ws_url` in your agent_setup?",
        )
    }

    /// Get the `app_ws_url` that was configured during agent setup.
    pub fn app_ws_url(&self) -> SocketAddr {
        self.app_ws_url.expect(
            "app_ws_url is not set, did you forget to call `configure_app_ws_url` in your agent_setup?",
        )
    }

    /// Get a mutable reference to the [`HolochainConfigBuilder`] used when running holochain with
    /// [`crate::common::run_holochain_conductor`].
    pub fn holochain_config_mut(&mut self) -> &mut HolochainConfigBuilder {
        self.holochain_config.get_or_insert_default()
    }

    /// Call [`Option::take`] on [`Self::holochain_config`], returning the current value or default
    /// value if not set, and setting the current internal value to [`None`].
    pub(crate) fn take_holochain_config(&mut self) -> HolochainConfigBuilder {
        self.holochain_config.take().unwrap_or_default()
    }

    /// Get a [`HolochainConfig`] if it has been built from the internal
    /// [`HolochainConfigBuilder`].
    pub fn holochain_config(&self) -> Option<HolochainConfig> {
        self.holochain_config
            .as_ref()
            .and_then(|b| b.clone().build().ok())
    }
}
