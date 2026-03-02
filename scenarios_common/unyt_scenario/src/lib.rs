//! Shared scenario infrastructure for Unyt load-testing scenarios.
//!
//! Provides common helpers for network initialization, agent setup,
//! durable object communication, and zome call wrappers that are
//! reused across the various `unyt_*` scenario binaries.

pub mod behaviour;
pub mod durable_object;
pub mod initiate_network;
pub mod setup;
pub mod unyt_agent;

use std::fmt;

use holochain_types::prelude::{ActionHashB64, AgentPubKeyB64};

/// DHT arc configuration for an agent.
#[derive(Debug, Clone, Copy)]
pub enum ArcType {
    /// Full-arc agent that stores all DHT data locally.
    Full,
    /// Zero-arc agent that relies on peers for data retrieval.
    Zero,
}

impl ArcType {
    /// Returns the tag value used in metric reporting.
    pub fn as_tag(&self) -> &'static str {
        match self {
            ArcType::Full => "full",
            ArcType::Zero => "zero",
        }
    }
}

impl fmt::Display for ArcType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_tag())
    }
}
use holochain_wind_tunnel_runner::prelude::UserValuesConstraint;
use std::collections::HashSet;

/// Shared state accessors for Unyt scenario values.
///
/// Scenario-specific `ScenarioValues` types must implement this trait
/// to enable the common agent extensions in [`unyt_agent::UnytAgentExt`]
/// and the shared setup/network-initialization helpers.
pub trait UnytScenarioValues: UserValuesConstraint {
    /// Returns the list of discovered participating agents.
    fn participating_agents(&self) -> &[AgentPubKeyB64];

    /// Replaces the participating agents list.
    fn set_participating_agents(&mut self, agents: Vec<AgentPubKeyB64>);

    /// Returns the executor's public key, if set.
    fn executor_pubkey(&self) -> Option<&AgentPubKeyB64>;

    /// Sets the executor's public key.
    fn set_executor_pubkey(&mut self, key: AgentPubKeyB64);

    /// Returns the smart agreement action hash, if set.
    fn smart_agreement_hash(&self) -> Option<&ActionHashB64>;

    /// Sets the smart agreement action hash.
    fn set_smart_agreement_hash(&mut self, hash: ActionHashB64);

    /// Returns the session start time, if recorded.
    fn session_start_time(&self) -> Option<tokio::time::Instant>;

    /// Records the session start time.
    fn set_session_start_time(&mut self, time: tokio::time::Instant);

    /// Returns whether the network has been initialized.
    fn network_initialized(&self) -> bool;

    /// Sets the network-initialized flag.
    fn set_network_initialized(&mut self, initialized: bool);

    /// Returns the progenitor agent's public key, if known.
    fn progenitor_agent_pubkey(&self) -> Option<&AgentPubKeyB64>;

    /// Sets the progenitor agent's public key.
    fn set_progenitor_agent_pubkey(&mut self, key: AgentPubKeyB64);

    /// Returns the set of code template hashes already seen by this agent.
    /// Used by the full_observer to track discovery progress.
    fn seen_templates(&self) -> &HashSet<ActionHashB64>;

    /// Returns a mutable reference to the seen templates set.
    fn seen_templates_mut(&mut self) -> &mut HashSet<ActionHashB64>;

    /// Returns the set of transaction hashes already seen by this agent.
    /// Used by zero-arc spend/smart_agreements behaviours to track sync lag.
    fn seen_transactions(&self) -> &HashSet<ActionHashB64>;

    /// Returns a mutable reference to the seen transactions set.
    fn seen_transactions_mut(&mut self) -> &mut HashSet<ActionHashB64>;

    /// Returns the list of transaction hashes being watched for completion
    /// via `get_status`. Mirrors the UI "watch list" feature.
    fn watched_transactions(&self) -> &Vec<ActionHashB64>;

    /// Returns a mutable reference to the watched transactions list.
    fn watched_transactions_mut(&mut self) -> &mut Vec<ActionHashB64>;
}

/// Default scenario values shared by all Unyt scenarios.
///
/// Scenarios that don't need additional per-agent state can use this
/// type directly via `type ScenarioValues = CommonScenarioValues;`.
/// Scenarios with extra fields can embed this struct and delegate the
/// [`UnytScenarioValues`] trait.
#[derive(Debug, Default)]
pub struct CommonScenarioValues {
    pub(crate) session_start_time: Option<tokio::time::Instant>,
    pub(crate) network_initialized: bool,
    pub(crate) participating_agents: Vec<AgentPubKeyB64>,
    pub(crate) executor_pubkey: Option<AgentPubKeyB64>,
    pub(crate) smart_agreement_hash: Option<ActionHashB64>,
    pub(crate) progenitor_agent_pubkey: Option<AgentPubKeyB64>,
    /// Tracks code template hashes already observed by this agent.
    /// Primarily used by the zero-arc observer behaviour; unused fields
    /// default to an empty set with no runtime cost.
    pub(crate) seen_templates: HashSet<ActionHashB64>,
    /// Tracks transaction hashes already observed by this agent.
    /// Used by zero-arc spend/smart_agreements behaviours for sync lag
    /// measurement; defaults to an empty set with no runtime cost.
    pub(crate) seen_transactions: HashSet<ActionHashB64>,
    /// Transaction hashes being watched for completion via `get_status`.
    /// Mirrors the UI "watch list" feature where initiated transactions
    /// are polled until they reach `WatchStatus::Completed`.
    pub(crate) watched_transactions: Vec<ActionHashB64>,
}

impl UserValuesConstraint for CommonScenarioValues {}

#[cfg(test)]
pub(crate) fn dummy_agent_pubkey_b64(seed: u8) -> AgentPubKeyB64 {
    holochain_types::prelude::AgentPubKey::from_raw_32(vec![seed; 32]).into()
}

#[cfg(test)]
pub(crate) fn dummy_action_hash_b64(seed: u8) -> ActionHashB64 {
    holochain_types::prelude::ActionHash::from_raw_32(vec![seed; 32]).into()
}

impl UnytScenarioValues for CommonScenarioValues {
    fn participating_agents(&self) -> &[AgentPubKeyB64] {
        &self.participating_agents
    }
    fn set_participating_agents(&mut self, agents: Vec<AgentPubKeyB64>) {
        self.participating_agents = agents;
    }
    fn executor_pubkey(&self) -> Option<&AgentPubKeyB64> {
        self.executor_pubkey.as_ref()
    }
    fn set_executor_pubkey(&mut self, key: AgentPubKeyB64) {
        self.executor_pubkey = Some(key);
    }
    fn smart_agreement_hash(&self) -> Option<&ActionHashB64> {
        self.smart_agreement_hash.as_ref()
    }
    fn set_smart_agreement_hash(&mut self, hash: ActionHashB64) {
        self.smart_agreement_hash = Some(hash);
    }
    fn session_start_time(&self) -> Option<tokio::time::Instant> {
        self.session_start_time
    }
    fn set_session_start_time(&mut self, time: tokio::time::Instant) {
        self.session_start_time = Some(time);
    }
    fn network_initialized(&self) -> bool {
        self.network_initialized
    }
    fn set_network_initialized(&mut self, initialized: bool) {
        self.network_initialized = initialized;
    }
    fn progenitor_agent_pubkey(&self) -> Option<&AgentPubKeyB64> {
        self.progenitor_agent_pubkey.as_ref()
    }
    fn set_progenitor_agent_pubkey(&mut self, key: AgentPubKeyB64) {
        self.progenitor_agent_pubkey = Some(key);
    }
    fn seen_templates(&self) -> &HashSet<ActionHashB64> {
        &self.seen_templates
    }
    fn seen_templates_mut(&mut self) -> &mut HashSet<ActionHashB64> {
        &mut self.seen_templates
    }
    fn seen_transactions(&self) -> &HashSet<ActionHashB64> {
        &self.seen_transactions
    }
    fn seen_transactions_mut(&mut self) -> &mut HashSet<ActionHashB64> {
        &mut self.seen_transactions
    }
    fn watched_transactions(&self) -> &Vec<ActionHashB64> {
        &self.watched_transactions
    }
    fn watched_transactions_mut(&mut self) -> &mut Vec<ActionHashB64> {
        &mut self.watched_transactions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_empty_state() {
        let sv = CommonScenarioValues::default();

        assert!(sv.session_start_time().is_none());
        assert!(!sv.network_initialized());
        assert!(sv.participating_agents().is_empty());
        assert!(sv.executor_pubkey().is_none());
        assert!(sv.smart_agreement_hash().is_none());
        assert!(sv.progenitor_agent_pubkey().is_none());
        assert!(sv.seen_templates().is_empty());
        assert!(sv.seen_transactions().is_empty());
        assert!(sv.watched_transactions().is_empty());
    }

    #[test]
    fn set_and_get_participating_agents() {
        let mut sv = CommonScenarioValues::default();
        let agents = vec![dummy_agent_pubkey_b64(1), dummy_agent_pubkey_b64(2)];

        sv.set_participating_agents(agents.clone());

        assert_eq!(sv.participating_agents().len(), 2);
        assert_eq!(sv.participating_agents()[0], agents[0]);
        assert_eq!(sv.participating_agents()[1], agents[1]);
    }

    #[test]
    fn set_and_get_executor_pubkey() {
        let mut sv = CommonScenarioValues::default();
        let key = dummy_agent_pubkey_b64(42);

        sv.set_executor_pubkey(key.clone());

        assert_eq!(sv.executor_pubkey(), Some(&key));
    }

    #[test]
    fn set_and_get_smart_agreement_hash() {
        let mut sv = CommonScenarioValues::default();
        let hash = dummy_action_hash_b64(7);

        sv.set_smart_agreement_hash(hash.clone());

        assert_eq!(sv.smart_agreement_hash(), Some(&hash));
    }

    #[test]
    fn set_and_get_session_start_time() {
        let mut sv = CommonScenarioValues::default();
        let now = tokio::time::Instant::now();

        sv.set_session_start_time(now);

        assert_eq!(sv.session_start_time(), Some(now));
    }

    #[test]
    fn set_and_get_network_initialized() {
        let mut sv = CommonScenarioValues::default();

        sv.set_network_initialized(true);
        assert!(sv.network_initialized());

        sv.set_network_initialized(false);
        assert!(!sv.network_initialized());
    }

    #[test]
    fn set_and_get_progenitor_agent_pubkey() {
        let mut sv = CommonScenarioValues::default();
        let key = dummy_agent_pubkey_b64(99);

        sv.set_progenitor_agent_pubkey(key.clone());

        assert_eq!(sv.progenitor_agent_pubkey(), Some(&key));
    }

    #[test]
    fn seen_templates_insert_and_contains() {
        let mut sv = CommonScenarioValues::default();
        let h1 = dummy_action_hash_b64(1);
        let h2 = dummy_action_hash_b64(2);

        sv.seen_templates_mut().insert(h1.clone());

        assert!(sv.seen_templates().contains(&h1));
        assert!(!sv.seen_templates().contains(&h2));
        assert_eq!(sv.seen_templates().len(), 1);

        sv.seen_templates_mut().insert(h2.clone());
        assert_eq!(sv.seen_templates().len(), 2);
    }

    #[test]
    fn seen_templates_dedup_on_reinsert() {
        let mut sv = CommonScenarioValues::default();
        let h = dummy_action_hash_b64(5);

        sv.seen_templates_mut().insert(h.clone());
        sv.seen_templates_mut().insert(h);

        assert_eq!(sv.seen_templates().len(), 1);
    }

    #[test]
    fn seen_transactions_insert_and_contains() {
        let mut sv = CommonScenarioValues::default();
        let h1 = dummy_action_hash_b64(10);
        let h2 = dummy_action_hash_b64(11);

        sv.seen_transactions_mut().insert(h1.clone());

        assert!(sv.seen_transactions().contains(&h1));
        assert!(!sv.seen_transactions().contains(&h2));
        assert_eq!(sv.seen_transactions().len(), 1);

        sv.seen_transactions_mut().insert(h2.clone());
        assert_eq!(sv.seen_transactions().len(), 2);
    }

    #[test]
    fn seen_transactions_dedup_on_reinsert() {
        let mut sv = CommonScenarioValues::default();
        let h = dummy_action_hash_b64(12);

        sv.seen_transactions_mut().insert(h.clone());
        sv.seen_transactions_mut().insert(h);

        assert_eq!(sv.seen_transactions().len(), 1);
    }

    #[test]
    fn set_participating_agents_replaces_previous() {
        let mut sv = CommonScenarioValues::default();
        sv.set_participating_agents(vec![dummy_agent_pubkey_b64(1)]);
        assert_eq!(sv.participating_agents().len(), 1);

        sv.set_participating_agents(vec![
            dummy_agent_pubkey_b64(2),
            dummy_agent_pubkey_b64(3),
            dummy_agent_pubkey_b64(4),
        ]);
        assert_eq!(sv.participating_agents().len(), 3);
        assert_eq!(sv.participating_agents()[0], dummy_agent_pubkey_b64(2));
    }
}
