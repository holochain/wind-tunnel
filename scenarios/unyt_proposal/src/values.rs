use crate::behaviours::common::ProposalConfig;
use crate::behaviours::ui_refresh::UiRefreshCounters;
use holochain_types::prelude::{ActionHashB64, AgentPubKeyB64};
use holochain_wind_tunnel_runner::prelude::*;
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use wind_tunnel_unyt_scenario::{CommonScenarioValues, UnytScenarioValues};

#[derive(Debug, Default)]
pub struct UnytProposalScenarioValues {
    pub common: CommonScenarioValues,
    /// Tracks proposal created by this agent with their creation time, for round-trip measurement
    pub pending_proposals: HashMap<ActionHashB64, Instant>,
    /// Environment-derived configuration, parsed and validated once in agent setup.
    pub config: Option<ProposalConfig>,
    /// Cumulative counts of UI refresh calls.
    pub ui_refresh_counters: UiRefreshCounters,
}

impl UnytProposalScenarioValues {
    /// The scenario configuration. Panics if accessed before agent setup has populated it.
    pub fn config(&self) -> ProposalConfig {
        self.config
            .expect("proposal config accessed before agent setup populated it")
    }

    /// Mutable access to this agent's cumulative UI-refresh counters.
    pub fn ui_refresh_counters_mut(&mut self) -> &mut UiRefreshCounters {
        &mut self.ui_refresh_counters
    }
}

impl UserValuesConstraint for UnytProposalScenarioValues {}

impl UnytScenarioValues for UnytProposalScenarioValues {
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

pub type ScenarioValues = UnytProposalScenarioValues;
