use std::{collections::HashMap, sync::Arc};

use crate::init::init;
use crate::{
    cli::WindTunnelScenarioCli,
    context::{AgentContext, RunnerContext, UserValuesConstraint},
};

/// The result type that is required to be returned from all hooks.
pub type HookResult = anyhow::Result<()>;

pub type GlobalHookMut<RV> = fn(&mut RunnerContext<RV>) -> HookResult;
pub type GlobalHook<RV> = fn(Arc<RunnerContext<RV>>) -> HookResult;
pub type AgentHookMut<RV, V> = fn(&mut AgentContext<RV, V>) -> HookResult;

/// The builder for a scenario definition.
///
/// This must be used at the start of a test to define the scenario that you want to run.
pub struct ScenarioDefinitionBuilder<RV: UserValuesConstraint, V: UserValuesConstraint> {
    name: String,
    #[doc(hidden)]
    cli: WindTunnelScenarioCli,
    default_agent_count: Option<usize>,
    default_duration_s: Option<u64>,
    setup_fn: Option<GlobalHookMut<RV>>,
    setup_agent_fn: Option<AgentHookMut<RV, V>>,
    agent_behaviour: HashMap<String, AgentHookMut<RV, V>>,
    teardown_agent_fn: Option<AgentHookMut<RV, V>>,
    teardown_fn: Option<GlobalHook<RV>>,
}

/// The result of combining a scenario builder with the input CLI arguments to produce a scenario definition.
pub struct ScenarioDefinition<RV: UserValuesConstraint, V: UserValuesConstraint> {
    pub name: String,
    pub agent_count: usize,
    pub duration_s: Option<u64>,
    pub connection_string: String,
    pub no_progress: bool,
    pub setup_fn: Option<GlobalHookMut<RV>>,
    pub setup_agent_fn: Option<AgentHookMut<RV, V>>,
    pub agent_behaviour: HashMap<String, AgentHookMut<RV, V>>,
    pub teardown_agent_fn: Option<AgentHookMut<RV, V>>,
    pub teardown_fn: Option<GlobalHook<RV>>,
}

impl<RV: UserValuesConstraint, V: UserValuesConstraint> ScenarioDefinitionBuilder<RV, V> {
    /// Initialise a new scenario definition from the scenario name and command line arguments.
    ///
    /// Calling this constructor will also initialise the runner CLI and set up logging by calling [init].
    /// This is a shortcut for:
    /// ```rust,no_run
    /// use wind_tunnel_runner::prelude::{init, ScenarioDefinitionBuilder, UserValuesConstraint};
    ///
    /// #[derive(Debug, Default)]
    /// struct Values {}
    /// impl UserValuesConstraint for Values {}
    ///
    /// let cli = init();
    /// let scenario = ScenarioDefinitionBuilder::<Values, Values>::new("my-scenario", cli);
    /// ```
    ///
    /// The name of the scenario should be unique within the test suite. The recommended value is `env!("CARGO_PKG_NAME")`.
    pub fn new_with_init(name: &str) -> Self {
        let cli = init();
        ScenarioDefinitionBuilder::new(name, cli)
    }

    /// Create a scenario definition without initialising the runner.
    ///
    /// This is intended for testing or scenarios where you want to avoid initialising the CLI.
    pub fn new(name: &str, cli: WindTunnelScenarioCli) -> Self {
        Self {
            name: name.to_string(),
            cli,
            default_agent_count: None,
            default_duration_s: None,
            setup_fn: None,
            setup_agent_fn: None,
            agent_behaviour: HashMap::new(),
            teardown_agent_fn: None,
            teardown_fn: None,
        }
    }

    /// Set the default number of agents that should be spawned for this scenario.
    ///
    /// This can be overridden when the scenario is run using the `--agents` flag.
    pub fn with_default_agent_count(mut self, count: usize) -> Self {
        self.default_agent_count = Some(count);
        self
    }

    /// Sets the default duration for this scenario, in seconds.
    ///
    /// This can be overridden when the scenario is run using the `--duration` flag.
    pub fn with_default_duration_s(mut self, duration: u64) -> Self {
        self.default_duration_s = Some(duration);
        self
    }

    /// Sets the global setup hook for this scenario. It will be run once, before any agents are started.
    pub fn use_setup(mut self, setup_fn: GlobalHookMut<RV>) -> Self {
        self.setup_fn = Some(setup_fn);
        self
    }

    /// Sets the setup hook for an agent. It will be run once for each agent, as it starts.
    pub fn use_agent_setup(mut self, setup_agent_fn: AgentHookMut<RV, V>) -> Self {
        self.setup_agent_fn = Some(setup_agent_fn);
        self
    }

    /// Sets the default agent behaviour for this scenario. There are two ways that this can be used:
    ///
    /// This should be used when you want to run agents with the same behaviour.
    pub fn use_agent_behaviour(self, behaviour: AgentHookMut<RV, V>) -> Self {
        self.use_named_agent_behaviour("default", behaviour)
    }

    /// Adds a named agent behaviour hook for this scenario.
    ///
    /// The names must be unique!
    ///
    /// This should be used when you want to run agents with different behaviours. Otherwise, use [ScenarioDefinitionBuilder::use_agent_behaviour].
    pub fn use_named_agent_behaviour(mut self, name: &str, behaviour: AgentHookMut<RV, V>) -> Self {
        let previous = self.agent_behaviour.insert(name.to_string(), behaviour);

        if previous.is_some() {
            panic!("Behaviour [{}] is already defined", name);
        }

        self
    }

    /// Sets the teardown hook for an agent, which will be run once for each agent when its behaviour is finished.
    ///
    /// If the scenario run is bounded by time, then this hook will be run.
    /// If the scenario is configured to run forever, then this hook will be run on a best effort basis when the scenario is stopped.
    pub fn use_agent_teardown(mut self, teardown_agent_fn: AgentHookMut<RV, V>) -> Self {
        self.teardown_agent_fn = Some(teardown_agent_fn);
        self
    }

    /// Sets the teardown hook for this scenario. It will be run once, after all agents have finished.
    ///
    /// If the scenario run is bounded by time, then this hook will be run.
    /// If the scenario is configured to run forever, then this hook will be run on a best effort basis when the scenario is stopped.
    pub fn use_teardown(mut self, teardown_fn: GlobalHook<RV>) -> Self {
        self.teardown_fn = Some(teardown_fn);
        self
    }

    pub(crate) fn build(self) -> ScenarioDefinition<RV, V> {
        let resolved_duration = if self.cli.soak {
            None
        } else {
            self.cli.duration.or(self.default_duration_s)
        };

        ScenarioDefinition {
            name: self.name,
            // Priority given to the CLI, then the default value provided by the scenario, then default to 1
            agent_count: self.cli.agents.or(self.default_agent_count).unwrap_or(1),
            duration_s: resolved_duration,
            connection_string: self.cli.connection_string,
            no_progress: self.cli.no_progress,
            setup_fn: self.setup_fn,
            setup_agent_fn: self.setup_agent_fn,
            agent_behaviour: self.agent_behaviour,
            teardown_agent_fn: self.teardown_agent_fn,
            teardown_fn: self.teardown_fn,
        }
    }
}
