use peerkit_client_instrumented::PeerkitNode;
use std::sync::Arc;
use wind_tunnel_runner::prelude::UserValuesConstraint;

/// Peerkit specific agent context values.
#[derive(Debug, Default)]
pub struct PeerkitAgentContext {
    /// The running `peerkit node` process for this agent.
    pub(crate) node: Option<Arc<PeerkitNode>>,
    /// Scratch slot for scenarios: the alias of the peer this agent talks to.
    pub target_alias: Option<String>,
}

impl UserValuesConstraint for PeerkitAgentContext {}

impl PeerkitAgentContext {
    /// Get the running node instance.
    pub fn node(&self) -> Arc<PeerkitNode> {
        self.node
            .clone()
            .expect("node is not set, did you forget to call `start_node` in your agent setup?")
    }
}
