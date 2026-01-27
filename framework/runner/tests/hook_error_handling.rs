use std::sync::Arc;
use wind_tunnel_core::prelude::AgentBailError;
use wind_tunnel_runner::prelude::{
    AgentContext, HookResult, ReporterOpt, RunnerContext, ScenarioDefinitionBuilder,
    UserValuesConstraint, WindTunnelScenarioCli, run,
};

#[derive(Default, Debug)]
struct RunnerContextValue {}

impl UserValuesConstraint for RunnerContextValue {}

#[derive(Default, Debug)]
struct AgentContextValue {
    value: i32,
}

impl UserValuesConstraint for AgentContextValue {}

fn sample_cli_cfg() -> WindTunnelScenarioCli {
    WindTunnelScenarioCli {
        connection_string: Some("test_connection_string".to_string()),
        agents: None,
        behaviour: vec![],
        duration: None,
        soak: false,
        no_progress: true,
        reporter: ReporterOpt::Noop,
        run_id: None,
    }
}

#[test]
fn propagate_error_in_setup_hook() {
    fn setup(_tx: &mut RunnerContext<RunnerContextValue>) -> HookResult {
        Err(anyhow::anyhow!("Error in setup hook"))
    }

    let scenario = ScenarioDefinitionBuilder::<RunnerContextValue, AgentContextValue>::new(
        "propagate_error_in_setup_hook",
        sample_cli_cfg(),
    )
    .with_default_duration_s(5)
    .use_setup(setup);

    let result = run(scenario);

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "Error in setup hook");
}

#[test]
fn capture_error_in_agent_setup() {
    fn agent_setup(_ctx: &mut AgentContext<RunnerContextValue, AgentContextValue>) -> HookResult {
        Err(anyhow::anyhow!("Error in agent setup hook"))
    }

    let scenario = ScenarioDefinitionBuilder::<RunnerContextValue, AgentContextValue>::new(
        "capture_error_in_agent_setup",
        sample_cli_cfg(),
    )
    .with_default_duration_s(5)
    .use_agent_setup(agent_setup);

    let result = run(scenario);

    assert!(result.is_ok());
}

#[test]
fn capture_error_in_agent_setup_and_continue() {
    fn agent_behaviour(
        ctx: &mut AgentContext<RunnerContextValue, AgentContextValue>,
    ) -> HookResult {
        if ctx.get().value < 5 {
            ctx.get_mut().value += 1;
        } else {
            // Save time running this test by shutting down once this has run a few times.
            ctx.runner_context().force_stop_scenario();
        }

        Err(anyhow::anyhow!("Error in agent behaviour hook"))
    }

    let scenario = ScenarioDefinitionBuilder::<RunnerContextValue, AgentContextValue>::new(
        "capture_error_in_agent_setup_and_continue",
        sample_cli_cfg(),
    )
    .with_default_duration_s(5)
    .use_agent_behaviour(agent_behaviour);

    let result = run(scenario);

    assert!(result.is_ok());
}

#[test]
fn bail_error_stops_agent_behaviour() {
    fn agent_behaviour_1(
        _ctx: &mut AgentContext<RunnerContextValue, AgentContextValue>,
    ) -> HookResult {
        Err(AgentBailError::default().into())
    }

    fn agent_behaviour_2(
        _ctx: &mut AgentContext<RunnerContextValue, AgentContextValue>,
    ) -> HookResult {
        Ok(())
    }

    let mut cfg = sample_cli_cfg();
    cfg.agents = Some(2);
    cfg.behaviour = vec![("bail".to_string(), 1), ("continue".to_string(), 1)];
    let scenario = ScenarioDefinitionBuilder::<RunnerContextValue, AgentContextValue>::new(
        "bail_error_stops_agent_behaviour",
        cfg,
    )
    .with_default_duration_s(1)
    .use_named_agent_behaviour("bail", agent_behaviour_1)
    .use_named_agent_behaviour("continue", agent_behaviour_2);

    let result = run(scenario);

    assert!(result.is_ok());
    assert_eq!(1, result.unwrap());
}

#[test]
fn capture_error_in_agent_teardown() {
    fn agent_teardown(
        _ctx: &mut AgentContext<RunnerContextValue, AgentContextValue>,
    ) -> HookResult {
        Err(anyhow::anyhow!("Error in agent teardown hook"))
    }

    let scenario = ScenarioDefinitionBuilder::<RunnerContextValue, AgentContextValue>::new(
        "capture_error_in_agent_teardown",
        sample_cli_cfg(),
    )
    .with_default_duration_s(5)
    .use_agent_teardown(agent_teardown);

    let result = run(scenario);

    assert!(result.is_ok());
}

#[test]
fn capture_error_in_teardown() {
    fn teardown(_ctx: Arc<RunnerContext<RunnerContextValue>>) -> HookResult {
        Err(anyhow::anyhow!("Error in teardown hook"))
    }

    let scenario = ScenarioDefinitionBuilder::<RunnerContextValue, AgentContextValue>::new(
        "capture_error_in_teardown",
        sample_cli_cfg(),
    )
    .with_default_duration_s(5)
    .use_teardown(teardown);

    let result = run(scenario);

    assert!(result.is_ok());
}
