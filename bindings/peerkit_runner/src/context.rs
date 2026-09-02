use peerkit_client_instrumented::PeerkitNode;
use std::collections::HashMap;
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
