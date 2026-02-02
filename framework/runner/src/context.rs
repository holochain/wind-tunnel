use crate::executor::Executor;
use holochain_client_instrumented::prelude::WebsocketConfig;
use std::{fmt::Debug, sync::Arc};
use wind_tunnel_core::prelude::{DelegatedShutdownListener, ShutdownHandle};
use wind_tunnel_instruments::Reporter;

pub trait UserValuesConstraint: Default + Debug + Send + Sync + 'static {}

/// The context created by the runner for a scenario run. This context is visible to all agents
/// so it is read-only from within agent hooks but can be modified by global hooks.
///
/// This type has a generic parameter for the user-defined state that can be stored in the context.
#[derive(Debug)]
pub struct RunnerContext<RV: UserValuesConstraint> {
    executor: Arc<Executor>,
    reporter: Arc<Reporter>,
    shutdown_handle: ShutdownHandle,
    run_id: String,
    connection_string: Option<String>,
    value: RV,
}

impl<RV: UserValuesConstraint> RunnerContext<RV> {
    pub(crate) fn new(
        executor: Arc<Executor>,
        reporter: Arc<Reporter>,
        shutdown_handle: ShutdownHandle,
        run_id: String,
        connection_string: Option<String>,
    ) -> Self {
        Self {
            executor,
            reporter,
            shutdown_handle,
            run_id,
            connection_string,
            value: Default::default(),
        }
    }

    /// A handle to the [Executor] for the runner.
    ///
    /// This is used to run async code within hooks.
    pub fn executor(&self) -> &Arc<Executor> {
        &self.executor
    }

    /// A handle to the reporter for the runner.
    ///
    /// This is used to record data in-memory. You shouldn't need to access it directly.
    /// This should be passed to an instrumented client so that it can report data to the runner.
    pub fn reporter(&self) -> Arc<Reporter> {
        self.reporter.clone()
    }

    /// Get a new shutdown listener that will be triggered when the runner is shutdown.
    ///
    /// This is provided in case you are doing something unexpected and need to hook into the shutdown process.
    /// In general, please consider using [Executor::execute_in_place] which automatically handles shutdown.
    pub fn new_shutdown_listener(&self) -> DelegatedShutdownListener {
        self.shutdown_handle.new_listener()
    }

    /// The unique identifier for the scenario run.
    pub fn get_run_id(&self) -> &str {
        &self.run_id
    }

    /// Connection string for the target service of the scenario, supplied by the user via the CLI.
    pub fn get_connection_string(&self) -> Option<&str> {
        self.connection_string.as_deref()
    }

    /// Get mutable access to the user-defined state for the runner.
    pub fn get_mut(&mut self) -> &mut RV {
        &mut self.value
    }

    /// Get the user-defined state for the runner.
    pub fn get(&self) -> &RV {
        &self.value
    }

    /// Force stop the scenario.
    ///
    /// This will trigger shutdown of all agents and the runner. It is primarily exposed for testing
    /// but if you need to stop the scenario from within a hook, you can use this. It is a better
    /// alternative to using a panic if you really need the scenario to stop.
    pub fn force_stop_scenario(&self) {
        self.shutdown_handle.shutdown();
    }
}

/// The context available to an agent during a scenario run. One context is created for each agent
/// so it is safe to store agent-specific state in this context.
///
/// The context holds a reference to the [RunnerContext] so that the agent can read shared state
/// for the scenario.
///
/// This type is generic over two parameters, one for the [AgentContext] and one for the [RunnerContext].
/// These are used to store user-defined state for the agent and the runner respectively.
#[derive(Debug)]
pub struct AgentContext<RV: UserValuesConstraint, V: UserValuesConstraint> {
    agent_index: usize,
    agent_name: String,
    assigned_behaviour: String,
    runner_context: Arc<RunnerContext<RV>>,
    shutdown_listener: DelegatedShutdownListener,
    value: V,
    websocket_config: Option<Arc<WebsocketConfig>>,
}

impl<RV: UserValuesConstraint, V: UserValuesConstraint> AgentContext<RV, V> {
    pub(crate) fn new(
        agent_index: usize,
        agent_name: String,
        assigned_behaviour: String,
        runner_context: Arc<RunnerContext<RV>>,
        shutdown_listener: DelegatedShutdownListener,
    ) -> Self {
        Self {
            agent_index,
            agent_name,
            assigned_behaviour,
            runner_context,
            shutdown_listener,
            value: Default::default(),
            websocket_config: None,
        }
    }

    /// Construct a new [`AgentContext``] with the given websocket configuration.
    pub fn with_websocket_config(mut self, config: Arc<WebsocketConfig>) -> Self {
        self.websocket_config = Some(config);
        self
    }

    /// The index of the agent within the scenario.
    pub fn agent_index(&self) -> usize {
        self.agent_index
    }

    /// A value generated by the runner that you can use to identify yourself when making requests.
    ///
    /// This value is unique within the runner but *not* unique across multiple runners.
    pub fn agent_name(&self) -> &str {
        &self.agent_name
    }

    /// The user-supplied value from [crate::prelude::ScenarioDefinitionBuilder::use_named_agent_behaviour].
    ///
    /// From within the behaviour, you know which behaviour you are assigned to. This is useful for
    /// the setup and teardown hooks where you may need to adjust the actions your agent takes
    /// based on the behaviour you are assigned.
    pub fn assigned_behaviour(&self) -> &str {
        &self.assigned_behaviour
    }

    /// A handle to the runner context for the scenario.
    pub fn runner_context(&self) -> &Arc<RunnerContext<RV>> {
        &self.runner_context
    }

    /// Get the shutdown listener which will be triggered when the runner is shutdown.
    ///
    /// This is provided in case you are doing something unexpected and need to hook into the shutdown process.
    /// In general, please consider using [Executor::execute_in_place] which automatically handles shutdown.
    pub fn shutdown_listener(&mut self) -> &mut DelegatedShutdownListener {
        &mut self.shutdown_listener
    }

    /// Get the websocket configuration for the agent, if one was set.
    pub fn websocket_config(&self) -> Option<Arc<WebsocketConfig>> {
        self.websocket_config.clone()
    }

    /// Get mutable access to the user-defined state for the agent.
    pub fn get_mut(&mut self) -> &mut V {
        &mut self.value
    }

    /// Get the user-defined state for the agent.
    pub fn get(&self) -> &V {
        &self.value
    }
}
