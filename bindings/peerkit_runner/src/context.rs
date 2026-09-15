use peerkit_client_instrumented::PeerkitNode;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use wind_tunnel_runner::prelude::UserValuesConstraint;

/// Tracks the arrival of one sender's message batch on the receiving side.
#[derive(Debug)]
pub struct ReceiveTracker {
    /// When the first message of the batch was received.
    pub first_at: std::time::Instant,
    /// When the most recent message of the batch was received.
    pub last_at: std::time::Instant,
    /// Number of messages received so far in the batch.
    pub received: u64,
    /// Total bytes received so far in the batch.
    pub bytes: u64,
    /// Message sequence numbers already counted for the batch.
    pub sequences: HashSet<u64>,
}

/// Peerkit specific agent context values.
#[derive(Debug, Default)]
pub struct PeerkitAgentContext {
    /// The running `peerkit node` process for this agent.
    pub(crate) node: Option<Arc<PeerkitNode>>,
    /// The Ed25519 private key seed file backing this agent's identity. It
    /// lives in a shared temp directory and is deleted at teardown so that
    /// keys do not accumulate on long-lived hosts.
    pub(crate) identity_path: Option<PathBuf>,
    /// Scratch for scenarios: the current behaviour-loop iteration.
    pub cycle: u64,
    /// Scratch for scenarios: in-flight receive batches, keyed by
    /// `"<alias>:<sender cycle>"`.
    pub receive_trackers: HashMap<String, ReceiveTracker>,
    /// Highest fully completed sender cycle for each peer alias. This keeps
    /// duplicate messages from a completed batch from recreating its tracker.
    pub completed_receive_cycles: HashMap<String, u64>,
}

impl UserValuesConstraint for PeerkitAgentContext {}

impl PeerkitAgentContext {
    /// Clone the running node handle when teardown needs to retain access to
    /// its buffered reader state after the runner removes it from the context.
    pub fn node_handle(&self) -> Option<Arc<PeerkitNode>> {
        self.node.clone()
    }

    /// Put a stopped node back temporarily so teardown can drain its final
    /// reader state before releasing the handle.
    pub fn set_node(&mut self, node: Arc<PeerkitNode>) {
        self.node = Some(node);
    }

    /// Release the node handle after teardown has drained its state.
    pub fn clear_node(&mut self) {
        self.node = None;
    }

    /// Get the running node instance.
    pub fn node(&self) -> Arc<PeerkitNode> {
        self.node
            .clone()
            .expect("node is not set, did you forget to call `start_node` in your agent setup?")
    }
}
