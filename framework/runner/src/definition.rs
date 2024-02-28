use std::{collections::HashMap, sync::Arc};

use clap::Parser;

use crate::{cli::WindTunnelScenarioCli, context::{Context, RunnerContext, UserValuesConstraint}};

pub type HookResult = anyhow::Result<()>;

pub type GlobalHookMut<RV> = fn(&mut RunnerContext<RV>) -> HookResult;
pub type GlobalHook<RV> = fn(Arc<RunnerContext<RV>>) -> HookResult;
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
    cli: WindTunnelScenarioCli,
    /// The default duration for this scenario, in seconds.
    ///
    /// This can be overridden when the scenario is run using the `--duration` flag.
    default_duration: Option<u64>,
    /// Global setup hook for this scenario. It will be run once, before any agents are started.
    setup_fn: Option<GlobalHookMut<RV>>,
    /// Setup hook for an agent, which will be run once for each agent as it starts.
    setup_agent_fn: Option<AgentHookMut<RV, V>>,
    /// The agent behaviour for this scenario. There are two ways that this can be used:
    /// - Specify a single behaviour for all agents using [ScenarioDefinitionBuilder::use_agent_behaviour]. This will then start as many identical agents as you request.
    /// - Specify multiple behaviours using [ScenarioDefinitionBuilder::use_named_agent_behaviour]. You then need to tell the runner how many agents you want to run each behaviour.
    agent_behaviour: HashMap<String, AgentHookMut<RV, V>>,
    /// Teardown hook for an agent, which will be run once for each agent when its behaviour is finished.
    ///
    /// If the scenario run is bounded by time, then this hook will be run.
    /// If the scenario is configured to run forever, then this hook will be run on a best effort basis when the test is stopped.
    teardown_agent_fn: Option<AgentHookMut<RV, V>>,
    /// Teardown hook for this scenario. It will be run once, after all agents have finished.
    /// 
    /// If the scenario run is bounded by time, then this hook will be run.
    /// If the scenario is configured to run forever, then this hook will be run on a best effort basis when the test is stopped.
    teardown_fn: Option<GlobalHook<RV>>,
}

/// The result of combining a scenario builder with the input CLI arguments to produce a scenario definition.
pub struct ScenarioDefinition<RV: UserValuesConstraint, V: UserValuesConstraint> {
    pub name: String,
    pub agent_count: usize,
    pub duration: Option<u64>,
    pub setup_fn: Option<GlobalHookMut<RV>>,
    pub setup_agent_fn: Option<AgentHookMut<RV, V>>,
    pub agent_behaviour: HashMap<String, AgentHookMut<RV, V>>,
    pub teardown_agent_fn: Option<AgentHookMut<RV, V>>,
    pub teardown_fn: Option<GlobalHook<RV>>,
}

impl<RV: UserValuesConstraint, V: UserValuesConstraint> ScenarioDefinitionBuilder<RV, V> {
    /// Initialise a new scenario definition from the scenario name and command line arguments.
    /// See the [ScenarioDefinitionBuilder::name] for more information about the name.
    pub fn new(name: &str) -> Self {
        // No that keen on mixing this into a constructor, but in the interest of keeping the boilerplate for tests
        // low this is going here for now.
        let cli = WindTunnelScenarioCli::parse();

        Self {
            name: name.to_string(),
            cli,
            default_duration: None,
            setup_fn: None,
            setup_agent_fn: None,
            agent_behaviour: HashMap::new(),
            teardown_agent_fn: None,
            teardown_fn: None,
        }
    }

    /// Set the default duration [ScenarioDefinitionBuilder::default_duration] for this scenario.
    pub fn with_default_duration(mut self, duration: u64) -> Self {
        self.default_duration = Some(duration);
        self
    }

    /// Set the global setup hook [ScenarioDefinitionBuilder::setup_fn] for this scenario.
    pub fn use_setup(mut self, setup_fn: GlobalHookMut<RV>) -> Self {
        self.setup_fn = Some(setup_fn);
        self
    }

    /// Set the agent setup hook [ScenarioDefinitionBuilder::setup_agent_fn] for this scenario.
    pub fn use_agent_setup(
        mut self,
        setup_agent_fn: AgentHookMut<RV, V>,
    ) -> Self {
        self.setup_agent_fn = Some(setup_agent_fn);
        self
    }

    /// Set the default agent behaviour hook [ScenarioDefinitionBuilder::agent_behaviour] for this scenario.
    pub fn use_agent_behaviour(self, behaviour: AgentHookMut<RV, V>) -> Self {
        self.use_named_agent_behaviour("default", behaviour)
    }

    /// Set a named agent behaviour hook [ScenarioDefinitionBuilder::agent_behaviour] for this scenario.
    pub fn use_named_agent_behaviour(
        mut self,
        name: &str,
        behaviour: AgentHookMut<RV, V>,
    ) -> Self {
        let previous = self.agent_behaviour.insert(name.to_string(), behaviour);

        if previous.is_some() {
            panic!("Behaviour [{}] is already defined", name);
        }

        self
    }

    /// Set the agent teardown hook [ScenarioDefinitionBuilder::teardown_agent_fn] for this scenario.
    pub fn use_agent_teardown(
        mut self,
        teardown_agent_fn: AgentHookMut<RV, V>,
    ) -> Self {
        self.teardown_agent_fn = Some(teardown_agent_fn);
        self
    }

    /// Set the global teardown hook [ScenarioDefinitionBuilder::teardown_fn] for this scenario.
    pub fn use_teardown(mut self, teardown_fn: GlobalHook<RV>) -> Self {
        self.teardown_fn = Some(teardown_fn);
        self
    }

    pub(crate) fn build(self) -> ScenarioDefinition<RV, V> {
        let resolved_duration = if self.cli.soak {
            None
        } else {
            self.cli.duration.or(self.default_duration)
        };

        ScenarioDefinition {
            name: self.name,
            agent_count: self.cli.agents,
            duration: resolved_duration,
            setup_fn: self.setup_fn,
            setup_agent_fn: self.setup_agent_fn,
            agent_behaviour: self.agent_behaviour,
            teardown_agent_fn: self.teardown_agent_fn,
            teardown_fn: self.teardown_fn,
        }
    }
}
