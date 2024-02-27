use std::collections::HashMap;

use crate::context::{Context, RunnerContext, UserValuesConstraint};

pub type HookResult = anyhow::Result<()>;

pub type GlobalHookMut<RV> = fn(&mut RunnerContext<RV>) -> HookResult;
pub type AgentHookMut<RV, V> = fn(&mut Context<RV, V>) -> HookResult;

/// The builder for a scenario definition.
///
/// This must be used at the start of a test to define the scenario that you want to run.
pub struct ScenarioDefinitionBuilder<RV: UserValuesConstraint, V: UserValuesConstraint> {
    /// The name of the scenario, which should be unique within the test suite.
    ///
    /// Recommended value is `env!("CARGO_PKG_NAME")`.
    name: String,
    /// This value is initialised for you and you cannot change it.
    #[doc(hidden)]
    cli: wind_tunnel_cli::WindTunnelCli,
    /// Global setup hook for this scenario. It will be run once, before any agents are started.
    setup_fn: Option<GlobalHookMut<RV>>,
    /// Setup hook for an agent, which will be run once for each agent as it starts.
    ///
    /// This is optional and if not provided, you can still set up an agent as part of the [ScenarioDefinitionBuilder::agent_behaviour].
    /// The difference with this hook is that it is syncronised so that you can set up all agents before any of them start running their behaviour.
    setup_agent_fn: Option<AgentHookMut<RV, V>>,
    /// The agent behaviour for this scenario. There are two ways that this can be used:
    /// - Specify a single behaviour for all agents using [ScenarioDefinitionBuilder::use_agent_behaviour]. This will then start as many identical agents as you request.
    /// - Specify multiple behaviours using [ScenarioDefinitionBuilder::use_named_agent_behaviour]. You then need to tell the runner how many agents you want to run each behaviour.
    agent_behaviour: HashMap<String, AgentHookMut<RV, V>>,
}

pub struct ScenarioDefinition<RV: UserValuesConstraint, V: UserValuesConstraint> {
    pub name: String,
    pub cli: wind_tunnel_cli::WindTunnelCli,
    pub setup_fn: Option<GlobalHookMut<RV>>,
    pub setup_agent_fn: Option<AgentHookMut<RV, V>>,
    pub agent_behaviour: HashMap<String, AgentHookMut<RV, V>>,
}

impl<RV: UserValuesConstraint, V: UserValuesConstraint> ScenarioDefinitionBuilder<RV, V> {
    /// Initialise a new scenario definition from the scenario name and command line arguments.
    /// See the [ScenarioDefinitionBuilder::name] for more information about the name.
    pub fn new(name: &str) -> Self {
        let cli = wind_tunnel_cli::init();

        Self {
            name: name.to_string(),
            cli,
            setup_fn: None,
            setup_agent_fn: None,
            agent_behaviour: HashMap::new(),
        }
    }

    /// Set the global setup hook [ScenarioDefinitionBuilder::setup_fn] for this scenario.
    pub fn use_setup(mut self, setup_fn: fn(&mut RunnerContext<RV>) -> HookResult) -> Self {
        self.setup_fn = Some(setup_fn);
        self
    }

    /// Set the agent setup hook [ScenarioDefinitionBuilder::setup_agent_fn] for this scenario.
    pub fn use_agent_setup(
        mut self,
        setup_agent_fn: fn(&mut Context<RV, V>) -> HookResult,
    ) -> Self {
        self.setup_agent_fn = Some(setup_agent_fn);
        self
    }

    /// Set the default agent behaviour hook [ScenarioDefinitionBuilder::agent_behaviour] for this scenario.
    pub fn use_agent_behaviour(self, behaviour: fn(&mut Context<RV, V>) -> HookResult) -> Self {
        self.use_named_agent_behaviour("default", behaviour)
    }

    /// Set a named agent behaviour hook [ScenarioDefinitionBuilder::agent_behaviour] for this scenario.
    pub fn use_named_agent_behaviour(
        mut self,
        name: &str,
        behaviour: fn(&mut Context<RV, V>) -> HookResult,
    ) -> Self {
        let previous = self.agent_behaviour.insert(name.to_string(), behaviour);

        if previous.is_some() {
            panic!("Behaviour [{}] is already defined", name);
        }

        self
    }

    pub(crate) fn build(self) -> ScenarioDefinition<RV, V> {
        ScenarioDefinition {
            name: self.name,
            cli: self.cli,
            setup_fn: self.setup_fn,
            setup_agent_fn: self.setup_agent_fn,
            agent_behaviour: self.agent_behaviour.clone(),
        }
    }
}
