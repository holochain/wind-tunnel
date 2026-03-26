use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use super::{CounterStats, GaugeStats, StandardTimingsStats};

/// String enum of all workflows in Holochain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, strum::Display)]
#[strum(serialize_all = "snake_case", suffix = "_consumer")]
pub enum HolochainWorkflowKind {
    AppValidation,
    Countersigning,
    IntegrateDhtOps,
    PublishDhtOps,
    SysValidation,
    ValidationReceipt,
    Witnessing,
}

/// Unified Holochain metrics collected from the conductor's OpenTelemetry instrumentation.
///
/// All fields are optional because not every metric is emitted in every scenario (e.g.
/// countersigning workflows only run in countersigning scenarios). Fields that have no
/// data in InfluxDB for a given run are serialized as absent (via `skip_serializing_none`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HolochainMetrics {
    // === Cascade ===
    /// Duration of cascade (get) operations inside Holochain (seconds)
    pub cascade_duration: Option<StandardTimingsStats>,
    /// Count of network fetch errors during cascade operations
    pub cascade_fetch_error_count: Option<CounterStats>,

    // === Ribosome ===
    /// Total WASM execution count across the run
    pub wasm_usage: Option<CounterStats>,
    /// Duration of zome calls as measured by Holochain (seconds).
    ///
    /// This is the Holochain-internal measurement of zome call time, complementary
    /// to wind-tunnel's own instrumented round-trip measurement.
    pub zome_call_duration: Option<StandardTimingsStats>,
    /// Duration of inner WASM calls (seconds), excluding Holochain overhead
    pub wasm_call_duration: Option<StandardTimingsStats>,
    /// Duration of host function calls invoked from within WASM (seconds).
    ///
    /// Aggregated across all host functions; individual function breakdown is not
    /// retained to limit output size.
    pub host_fn_call_duration: Option<StandardTimingsStats>,
    /// Count of local signals emitted via the `emit_signal` host function
    pub emit_signal_count: Option<CounterStats>,
    /// Count of remote signals sent via the `send_remote_signal` host function
    pub send_remote_signal_count: Option<CounterStats>,

    // === Conductor ===
    /// Duration of post-commit workflow executions (seconds)
    pub post_commit_duration: Option<StandardTimingsStats>,
    /// Conductor uptime (seconds).
    ///
    /// As a gauge, this is sampled periodically. A sudden drop in the trend indicates
    /// a conductor restart during the run.
    pub uptime: Option<GaugeStats>,
    /// Count of signals dropped from the app WebSocket due to channel overload
    pub dropped_signal_count: Option<CounterStats>,
    /// Count of DHT operations integrated across the run
    pub integrated_ops_count: Option<CounterStats>,
    /// Delay between an op being stored and being integrated (seconds).
    ///
    /// High values indicate the validation -> integration pipeline is falling behind.
    pub integration_delay: Option<StandardTimingsStats>,
    /// Number of validation attempts required per operation.
    ///
    /// Values consistently above 1 indicate validation retries, which may signal
    /// dependency-ordering issues or transient failures.
    pub validation_attempts: Option<StandardTimingsStats>,

    // === Workflows ===
    /// Duration of each Holochain workflow type (seconds)
    pub workflow_duration: Option<WorkflowDurations>,

    // === Database ===
    /// Time spent holding database connections, by database kind (seconds)
    pub db_connection_use_time: Option<DbConnectionUseTimes>,
    /// Duration of exclusive write transactions across all databases (seconds)
    pub write_txn_duration: Option<StandardTimingsStats>,

    // === Keystore ===
    /// Duration of signing and encryption requests to the Lair keystore (seconds)
    pub lair_request_duration: Option<StandardTimingsStats>,

    // === P2P ===
    /// Holochain peer-to-peer network metrics
    pub p2p: Option<P2pMetrics>,
}

/// Duration of each Holochain workflow type (seconds).
///
/// Each field corresponds to one workflow consumer. Not all workflows run in every
/// scenario, so all fields are optional.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowDurations {
    /// Duration of system validation workflow executions (seconds)
    pub sys_validation: Option<StandardTimingsStats>,
    /// Duration of app validation workflow executions (seconds)
    pub app_validation: Option<StandardTimingsStats>,
    /// Duration of DHT ops integration workflow executions (seconds)
    pub integrate_dht_ops: Option<StandardTimingsStats>,
    /// Duration of DHT ops publishing workflow executions (seconds)
    pub publish_dht_ops: Option<StandardTimingsStats>,
    /// Duration of validation receipt workflow executions (seconds)
    pub validation_receipt: Option<StandardTimingsStats>,
    /// Duration of countersigning workflow executions (seconds)
    pub countersigning: Option<StandardTimingsStats>,
    /// Duration of witnessing workflow executions (seconds)
    pub witnessing: Option<StandardTimingsStats>,
}

/// Time spent holding database connections, by database kind (seconds).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DbConnectionUseTimes {
    /// Authored database connection hold time (seconds)
    pub authored: Option<StandardTimingsStats>,
    /// DHT database connection hold time (seconds)
    pub dht: Option<StandardTimingsStats>,
    /// Conductor database connection hold time (seconds)
    pub conductor: Option<StandardTimingsStats>,
    /// Cache database connection hold time (seconds)
    pub cache: Option<StandardTimingsStats>,
    /// WASM database connection hold time (seconds)
    pub wasm: Option<StandardTimingsStats>,
    /// Peer meta store database connection hold time (seconds)
    pub peer_meta_store: Option<StandardTimingsStats>,
}

/// Holochain peer-to-peer network metrics.
///
/// Covers outgoing request round-trips, incoming request handling, and anomaly
/// counters (ignored requests, received remote signals).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct P2pMetrics {
    /// Outgoing P2P request round-trip durations by request type
    pub request_duration: Option<P2pRequestDurations>,
    /// Count of outgoing P2P requests by request type
    pub request_count: Option<P2pRequestCounts>,
    /// Incoming P2P request handling durations by message type
    pub handle_request_duration: Option<P2pHandleRequestDurations>,
    /// Count of incoming P2P requests handled by message type
    pub handle_request_count: Option<P2pHandleRequestCounts>,
    /// Count of incoming P2P requests that were ignored (e.g. for uninstalled DNAs)
    pub ignored_request_count: Option<CounterStats>,
    /// Count of remote signals received
    pub recv_remote_signal_count: Option<CounterStats>,
}

/// Outgoing P2P request round-trip duration by request type (seconds).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct P2pRequestDurations {
    /// Round-trip time for `get` requests (seconds)
    pub get: Option<StandardTimingsStats>,
    /// Round-trip time for `get_links` requests (seconds)
    pub get_links: Option<StandardTimingsStats>,
    /// Round-trip time for `count_links` requests (seconds)
    pub count_links: Option<StandardTimingsStats>,
    /// Round-trip time for `get_agent_activity` requests (seconds)
    pub get_agent_activity: Option<StandardTimingsStats>,
    /// Round-trip time for `must_get_agent_activity` requests (seconds)
    pub must_get_agent_activity: Option<StandardTimingsStats>,
    /// Round-trip time for `send_validation_receipts` requests (seconds)
    pub send_validation_receipts: Option<StandardTimingsStats>,
    /// Round-trip time for `call_remote` requests (seconds)
    pub call_remote: Option<StandardTimingsStats>,
}

/// Count of outgoing P2P requests by request type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct P2pRequestCounts {
    /// Number of outgoing `get` requests
    pub get: usize,
    /// Number of outgoing `get_links` requests
    pub get_links: usize,
    /// Number of outgoing `count_links` requests
    pub count_links: usize,
    /// Number of outgoing `get_agent_activity` requests
    pub get_agent_activity: usize,
    /// Number of outgoing `must_get_agent_activity` requests
    pub must_get_agent_activity: usize,
    /// Number of outgoing `send_validation_receipts` requests
    pub send_validation_receipts: usize,
    /// Number of outgoing `call_remote` requests
    pub call_remote: usize,
}

/// Incoming P2P request handling duration by message type (seconds).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct P2pHandleRequestDurations {
    /// Time to handle incoming response messages (seconds)
    pub response: Option<StandardTimingsStats>,
    /// Time to handle incoming `call_remote` requests (seconds)
    pub call_remote: Option<StandardTimingsStats>,
    /// Time to handle incoming `get` requests (seconds)
    pub get: Option<StandardTimingsStats>,
    /// Time to handle incoming `get_links` requests (seconds)
    pub get_links: Option<StandardTimingsStats>,
    /// Time to handle incoming `count_links` requests (seconds)
    pub count_links: Option<StandardTimingsStats>,
    /// Time to handle incoming `get_agent_activity` requests (seconds)
    pub get_agent_activity: Option<StandardTimingsStats>,
    /// Time to handle incoming `must_get_agent_activity` requests (seconds)
    pub must_get_agent_activity: Option<StandardTimingsStats>,
    /// Time to handle incoming `send_validation_receipts` requests (seconds)
    pub send_validation_receipts: Option<StandardTimingsStats>,
    /// Time to handle incoming `remote_signal` requests (seconds)
    pub remote_signal: Option<StandardTimingsStats>,
    /// Time to handle incoming `publish_counter_sign` requests (seconds)
    pub publish_counter_sign: Option<StandardTimingsStats>,
    /// Time to handle incoming `countersigning_session_negotiation` requests (seconds)
    pub countersigning_session_negotiation: Option<StandardTimingsStats>,
}

/// Count of incoming P2P requests handled by message type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct P2pHandleRequestCounts {
    /// Number of incoming response messages handled
    pub response: usize,
    /// Number of incoming `call_remote` requests handled
    pub call_remote: usize,
    /// Number of incoming `get` requests handled
    pub get: usize,
    /// Number of incoming `get_links` requests handled
    pub get_links: usize,
    /// Number of incoming `count_links` requests handled
    pub count_links: usize,
    /// Number of incoming `get_agent_activity` requests handled
    pub get_agent_activity: usize,
    /// Number of incoming `must_get_agent_activity` requests handled
    pub must_get_agent_activity: usize,
    /// Number of incoming `send_validation_receipts` requests handled
    pub send_validation_receipts: usize,
    /// Number of incoming `remote_signal` requests handled
    pub remote_signal: usize,
    /// Number of incoming `publish_counter_sign` requests handled
    pub publish_counter_sign: usize,
    /// Number of incoming `countersigning_session_negotiation` requests handled
    pub countersigning_session_negotiation: usize,
}
