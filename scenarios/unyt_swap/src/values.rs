use holochain_types::prelude::{ActionHashB64, AgentPubKeyB64};
use holochain_wind_tunnel_runner::prelude::*;
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use wind_tunnel_unyt_scenario::{CommonScenarioValues, UnytScenarioValues};

/// Per-agent scenario state: the shared Unyt state plus the swap behaviour's
/// cached swap agent key (the commitment counterparty), fetched once from the
/// Durable Object.
///
/// The count fields are monotonically increasing per-agent running totals. The
/// owning behaviour emits its running total each round (tagged with `agent`, plus
/// `arc` where the behaviour is arc-split) so the summariser can treat them as true
/// counters and recover per-agent totals with `last - first`.
#[derive(Debug, Default)]
pub struct ScenarioValues {
    pub common: CommonScenarioValues,
    pub swap_agent_key: Option<AgentPubKeyB64>,
    /// Running total of proof-of-deposit parked links the bridge agent has created.
    pub bridge_parked_links_completed: u64,
    /// Running total of failures of proof-of-deposit parked links the bridge agent has created.
    pub bridge_parked_links_failed: u64,
    /// Running total of deposit RAVEs this agent has collected (HOT -> wHOT).
    pub deposit_raves_collected: u64,
    /// Running total of swap commitments this agent has created (wHOT -> HF offered).
    pub swap_commitments_created: u64,
    /// Running total of swap commitments this swap agent has accepted.
    pub commitments_accepted: u64,
    /// Running total of swap receipts this agent has created (completed wHOT -> HF).
    pub swap_receipts_created: u64,
    /// When each outstanding swap commitment was created, keyed by its action hash.
    /// Used to measure swap completion time when the matching accept is finalized.
    pub commitment_started_at: HashMap<ActionHashB64, Instant>,
}

impl UserValuesConstraint for ScenarioValues {}

impl UnytScenarioValues for ScenarioValues {
    fn participating_agents(&self) -> &[AgentPubKeyB64] {
        self.common.participating_agents()
    }
    fn set_participating_agents(&mut self, agents: Vec<AgentPubKeyB64>) {
        self.common.set_participating_agents(agents);
    }
    fn executor_pubkey(&self) -> Option<&AgentPubKeyB64> {
        self.common.executor_pubkey()
    }
    fn set_executor_pubkey(&mut self, key: AgentPubKeyB64) {
        self.common.set_executor_pubkey(key);
    }
    fn smart_agreement_hash(&self) -> Option<&ActionHashB64> {
        self.common.smart_agreement_hash()
    }
    fn set_smart_agreement_hash(&mut self, hash: ActionHashB64) {
        self.common.set_smart_agreement_hash(hash);
    }
    fn session_start_time(&self) -> Option<tokio::time::Instant> {
        self.common.session_start_time()
    }
    fn set_session_start_time(&mut self, time: tokio::time::Instant) {
        self.common.set_session_start_time(time);
    }
    fn network_initialized(&self) -> bool {
        self.common.network_initialized()
    }
    fn set_network_initialized(&mut self, initialized: bool) {
        self.common.set_network_initialized(initialized);
    }
    fn progenitor_agent_pubkey(&self) -> Option<&AgentPubKeyB64> {
        self.common.progenitor_agent_pubkey()
    }
    fn set_progenitor_agent_pubkey(&mut self, key: AgentPubKeyB64) {
        self.common.set_progenitor_agent_pubkey(key);
    }
    fn seen_templates(&self) -> &HashSet<ActionHashB64> {
        self.common.seen_templates()
    }
    fn seen_templates_mut(&mut self) -> &mut HashSet<ActionHashB64> {
        self.common.seen_templates_mut()
    }
    fn seen_transactions(&self) -> &HashSet<(ActionHashB64, &'static str)> {
        self.common.seen_transactions()
    }
    fn seen_transactions_mut(&mut self) -> &mut HashSet<(ActionHashB64, &'static str)> {
        self.common.seen_transactions_mut()
    }
    fn watched_transactions(&self) -> &Vec<ActionHashB64> {
        self.common.watched_transactions()
    }
    fn watched_transactions_mut(&mut self) -> &mut Vec<ActionHashB64> {
        self.common.watched_transactions_mut()
    }
}
