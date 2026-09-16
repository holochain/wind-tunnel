use clap::Parser;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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

#[derive(Debug, Default)]
struct RuntimeBoundAgentContext;

impl UserValuesConstraint for RuntimeBoundAgentContext {}

impl Drop for RuntimeBoundAgentContext {
    fn drop(&mut self) {
        let _task = tokio::spawn(async {});
    }
}

fn sample_cli_cfg() -> WindTunnelScenarioCli {
    WindTunnelScenarioCli {
        connection_string: Some("test_connection_string".to_string()),
        agents: None,
        behaviour: vec![],
        duration: None,
        soak: false,
        no_progress: true,
        fail_on_agent_panic: false,
        reporter: ReporterOpt::Noop,
        run_id: None,
    }
}

#[test]
fn accepts_fail_on_agent_panic_flag() {
    let result = WindTunnelScenarioCli::try_parse_from(["scenario", "--fail-on-agent-panic"]);

    assert!(result.is_ok());
}

fn panic_in_agent(_ctx: &mut AgentContext<RunnerContextValue, AgentContextValue>) -> HookResult {
    panic!("agent panic for test");
}

static TEARDOWN_CALLED_AFTER_AGENT_PANIC: AtomicBool = AtomicBool::new(false);

fn mark_teardown_after_agent_panic(_ctx: Arc<RunnerContext<RunnerContextValue>>) -> HookResult {
    TEARDOWN_CALLED_AFTER_AGENT_PANIC.store(true, Ordering::SeqCst);
    Ok(())
}

#[test]
fn agent_panic_is_logged_by_default() {
    let scenario = ScenarioDefinitionBuilder::<RunnerContextValue, AgentContextValue>::new(
        "agent_panic_is_logged_by_default",
        sample_cli_cfg(),
    )
    .with_default_duration_s(1)
    .use_agent_behaviour(panic_in_agent);

    let result = run(scenario);

    assert!(result.is_ok());
}

#[test]
fn fail_on_agent_panic_returns_join_error() {
    let mut cfg = sample_cli_cfg();
    cfg.fail_on_agent_panic = true;
    let scenario = ScenarioDefinitionBuilder::<RunnerContextValue, AgentContextValue>::new(
        "fail_on_agent_panic_returns_join_error",
        cfg,
    )
    .with_default_duration_s(1)
    .use_agent_behaviour(panic_in_agent);

    let error = run(scenario).expect_err("agent panic should fail strict scenarios");

    assert!(
        error
            .to_string()
            .contains("Could not join thread for test agent 0: agent panic for test"),
        "unexpected error: {error:?}"
    );
}

#[test]
fn fail_on_agent_panic_runs_teardown_before_returning() {
    TEARDOWN_CALLED_AFTER_AGENT_PANIC.store(false, Ordering::SeqCst);

    let mut cfg = sample_cli_cfg();
    cfg.fail_on_agent_panic = true;
    let scenario = ScenarioDefinitionBuilder::<RunnerContextValue, AgentContextValue>::new(
        "fail_on_agent_panic_runs_teardown_before_returning",
        cfg,
    )
    .with_default_duration_s(1)
    .use_agent_behaviour(panic_in_agent)
    .use_teardown(mark_teardown_after_agent_panic);

    let _result = run(scenario);

    assert!(TEARDOWN_CALLED_AFTER_AGENT_PANIC.load(Ordering::SeqCst));
}

#[test]
fn agent_context_is_dropped_within_runtime_context() {
    fn stop_scenario(
        ctx: &mut AgentContext<RunnerContextValue, RuntimeBoundAgentContext>,
    ) -> HookResult {
        ctx.runner_context().force_stop_scenario();
        Ok(())
    }

    let mut cfg = sample_cli_cfg();
    cfg.fail_on_agent_panic = true;
    let scenario = ScenarioDefinitionBuilder::<RunnerContextValue, RuntimeBoundAgentContext>::new(
        "agent_context_is_dropped_within_runtime_context",
        cfg,
    )
    .use_agent_setup(stop_scenario);

    let result = run(scenario);

    assert!(result.is_ok(), "unexpected error: {result:?}");
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
