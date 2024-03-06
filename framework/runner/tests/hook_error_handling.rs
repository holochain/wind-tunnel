use std::sync::Arc;
use wind_tunnel_runner::prelude::{
    run, AgentContext, HookResult, RunnerContext, ScenarioDefinitionBuilder, UserValuesConstraint,
    WindTunnelScenarioCli,
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
        connection_string: "test_connection_string".to_string(),
        agents: None,
        behaviour: vec![],
        duration: None,
        soak: false,
        no_progress: true,
        no_metrics: true,
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
