use holochain_client::ClientAgentSigner;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use trycp_client_instrumented::prelude::TryCPClient;
use trycp_client_instrumented::TryCPClientInstrumented;
use wind_tunnel_runner::prelude::UserValuesConstraint;

#[derive(Debug, Default)]
pub struct DefaultScenarioValues {
    pub values: HashMap<String, String>,
}

impl UserValuesConstraint for DefaultScenarioValues {}

/// Holochain-specific context values for the [wind_tunnel_runner::prelude::AgentContext].
#[derive(Default, Debug)]
pub struct TryCPAgentContext<T: UserValuesConstraint = DefaultScenarioValues> {
    pub(crate) signer: Option<Arc<ClientAgentSigner>>,
    pub(crate) trycp_client: Option<TryCPClient>,
    pub scenario_values: T,
}

impl<T: UserValuesConstraint> UserValuesConstraint for TryCPAgentContext<T> {}

impl<T: UserValuesConstraint> TryCPAgentContext<T> {
    /// Get the [ClientAgentSigner] that was configured during agent setup.
    pub fn signer(&self) -> Arc<ClientAgentSigner> {
        self.signer.clone().expect(
            "signer is not set, did you forget to call `connect_trycp_client` in your agent_setup?",
        )
    }

    /// Get the [TryCPClient] that was configured during agent setup.
    pub fn trycp_client(&self) -> TryCPClient {
        self.trycp_client.clone().expect(
            "trycp_client is not set, did you forget to call `connect_trycp_client` in your agent_setup?",
        )
    }

    /// Close the TryCP client by dropping it.
    ///
    /// Calling [TryCPAgentContext::trycp_client] after this function, or this function again after, will panic.
    pub fn take_trycp_client(&mut self) -> TryCPClientInstrumented {
        self.trycp_client.take().expect("trycp_client is not set")
    }
}
