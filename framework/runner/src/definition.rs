use std::collections::HashSet;
use std::{collections::HashMap, sync::Arc};

use crate::cli::ReporterOpt;
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
    capture_env: HashSet<String>,
    setup_fn: Option<GlobalHookMut<RV>>,
    setup_agent_fn: Option<AgentHookMut<RV, V>>,
    agent_behaviour: HashMap<String, AgentHookMut<RV, V>>,
    teardown_agent_fn: Option<AgentHookMut<RV, V>>,
    teardown_fn: Option<GlobalHook<RV>>,
}

pub struct AssignedBehaviour {
    pub(crate) behaviour_name: String,
    pub(crate) agent_count: usize,
}

/// The result of combining a scenario builder with the input CLI arguments to produce a scenario definition.
pub struct ScenarioDefinition<RV: UserValuesConstraint, V: UserValuesConstraint> {
    pub(crate) name: String,
    pub(crate) assigned_behaviours: Vec<AssignedBehaviour>,
    pub(crate) duration_s: Option<u64>,
    pub(crate) connection_string: String,
    pub(crate) capture_env: HashSet<String>,
    pub(crate) no_progress: bool,
    pub(crate) reporter: ReporterOpt,
    pub(crate) setup_fn: Option<GlobalHookMut<RV>>,
    pub(crate) setup_agent_fn: Option<AgentHookMut<RV, V>>,
    pub(crate) agent_behaviour: HashMap<String, AgentHookMut<RV, V>>,
    pub(crate) teardown_agent_fn: Option<AgentHookMut<RV, V>>,
    pub(crate) teardown_fn: Option<GlobalHook<RV>>,
}

impl<RV: UserValuesConstraint, V: UserValuesConstraint> ScenarioDefinition<RV, V> {
    pub(crate) fn assigned_behaviours_flat(&self) -> Vec<String> {
        self.assigned_behaviours
            .iter()
            .flat_map(|b| std::iter::repeat(&b.behaviour_name).take(b.agent_count))
            .cloned()
            .collect()
    }
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
            capture_env: HashSet::with_capacity(0),
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

    pub fn add_capture_env(mut self, key: &str) -> Self {
        self.capture_env.insert(key.to_string());
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

    pub(crate) fn build(self) -> anyhow::Result<ScenarioDefinition<RV, V>> {
        let resolved_duration = if self.cli.soak {
            None
        } else {
            self.cli.duration.or(self.default_duration_s)
        };

        // Priority given to the CLI, then the default value provided by the scenario, then default to 1
        let resolved_agent_count = self.cli.agents.or(self.default_agent_count).unwrap_or(1);

        // Check that the user hasn't requested behaviours that aren't registered in the scenario.
        let registered_behaviours = self
            .agent_behaviour
            .keys()
            .cloned()
            .collect::<HashSet<String>>();
        let requested_behaviours = self
            .cli
            .behaviour
            .iter()
            .map(|(name, _)| name.clone())
            .collect::<HashSet<String>>();
        let unknown_behaviours = requested_behaviours
            .difference(&registered_behaviours)
            .collect::<Vec<&String>>();
        if !unknown_behaviours.is_empty() {
            return Err(anyhow::anyhow!(
                "Unknown behaviours requested: {:?}",
                unknown_behaviours
            ));
        }

        Ok(ScenarioDefinition {
            name: self.name,
            assigned_behaviours: build_assigned_behaviours(&self.cli, resolved_agent_count)?,
            duration_s: resolved_duration,
            connection_string: self.cli.connection_string,
            capture_env: self.capture_env,
            no_progress: self.cli.no_progress,
            reporter: self.cli.reporter,
            setup_fn: self.setup_fn,
            setup_agent_fn: self.setup_agent_fn,
            agent_behaviour: self.agent_behaviour,
            teardown_agent_fn: self.teardown_agent_fn,
            teardown_fn: self.teardown_fn,
        })
    }
}

fn build_assigned_behaviours(
    cli: &WindTunnelScenarioCli,
    resolved_agent_count: usize,
) -> anyhow::Result<Vec<AssignedBehaviour>> {
    let mut resolved_agent_count = resolved_agent_count as i32; // Signed so we can go negative.
    let mut assigned_behaviours = Vec::new();
    for (behaviour_name, agent_count) in &cli.behaviour {
        resolved_agent_count -= *agent_count as i32;
        if resolved_agent_count < 0 {
            return Err(anyhow::anyhow!("The number of agents assigned to behaviours must be less than or equal to the total number of agents"));
        }

        assigned_behaviours.push(AssignedBehaviour {
            behaviour_name: behaviour_name.to_string(),
            agent_count: *agent_count,
        });
    }

    if resolved_agent_count > 0 {
        assigned_behaviours.push(AssignedBehaviour {
            behaviour_name: "default".to_string(),
            agent_count: resolved_agent_count as usize, // Known > 0 here as checked above.
        });
    }

    Ok(assigned_behaviours)
}

#[cfg(test)]
mod tests {
    use crate::cli::ReporterOpt;
    use crate::definition::build_assigned_behaviours;

    #[test]
    pub fn build_assigned_behaviours_default() {
        let assigned = build_assigned_behaviours(
            &crate::cli::WindTunnelScenarioCli {
                connection_string: "".to_string(),
                agents: None,
                behaviour: vec![],
                duration: None,
                soak: false,
                no_progress: true,
                reporter: ReporterOpt::Noop,
            },
            5,
        )
        .unwrap();

        assert_eq!(1, assigned.len());
        assert_eq!("default", assigned[0].behaviour_name);
        assert_eq!(5, assigned[0].agent_count);
    }

    #[test]
    pub fn build_assigned_behaviours_exact() {
        let assigned = build_assigned_behaviours(
            &crate::cli::WindTunnelScenarioCli {
                connection_string: "".to_string(),
                agents: None,
                behaviour: vec![], // Not specified
                duration: None,
                soak: false,
                no_progress: true,
                reporter: ReporterOpt::Noop,
            },
            5,
        )
        .unwrap();

        assert_eq!(1, assigned.len());
        assert_eq!("default", assigned[0].behaviour_name);
        assert_eq!(5, assigned[0].agent_count);
    }

    #[test]
    pub fn build_assigned_behaviours_partial() {
        let assigned = build_assigned_behaviours(
            &crate::cli::WindTunnelScenarioCli {
                connection_string: "".to_string(),
                agents: None,
                behaviour: vec![("login".to_string(), 3)], // 3 of 5
                duration: None,
                soak: false,
                no_progress: true,
                reporter: ReporterOpt::Noop,
            },
            5,
        )
        .unwrap();

        assert_eq!(2, assigned.len());
        assert_eq!("login", assigned[0].behaviour_name);
        assert_eq!(3, assigned[0].agent_count);
        assert_eq!("default", assigned[1].behaviour_name);
        assert_eq!(2, assigned[1].agent_count);
    }

    #[test]
    pub fn build_assigned_behaviours_too_many() {
        let result = build_assigned_behaviours(
            &crate::cli::WindTunnelScenarioCli {
                connection_string: "".to_string(),
                agents: None,
                behaviour: vec![("login".to_string(), 30)], // 30 of 5
                duration: None,
                soak: false,
                no_progress: true,
                reporter: ReporterOpt::Noop,
            },
            5,
        );

        assert!(result.is_err());
    }
}
