use holochain_types::prelude::{ActionHash, Record};
use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;

#[derive(Debug, Default)]
struct ScenarioValues {
    sample_action_hash: Option<ActionHash>,
}

impl UserValuesConstraint for ScenarioValues {}

fn setup(ctx: &mut RunnerContext<HolochainRunnerContext>) -> HookResult {
    create_and_run_sandbox(ctx)?;
    configure_app_ws_url(ctx)?;
    Ok(())
}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    install_app(ctx, scenario_happ_path!("crud"), &"crud".to_string())?;

    // Just create a single entry and the agent behaviour will read it repeatedly.
    let action_hash: ActionHash = call_zome(
        ctx,
        "crud",
        "create_sample_entry",
        "this is a test entry value",
    )?;

    ctx.get_mut().scenario_values.sample_action_hash = Some(action_hash);

    Ok(())
}

fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let action_hash = ctx
        .get()
        .scenario_values
        .sample_action_hash
        .clone()
        .unwrap();
    let response: Option<Record> = call_zome(ctx, "crud", "get_sample_entry", action_hash)?;

    assert!(response.is_some(), "Expected record to be found");

    Ok(())
}

fn main() -> WindTunnelResult<()> {
    let builder = ScenarioDefinitionBuilder::<
        HolochainRunnerContext,
        HolochainAgentContext<ScenarioValues>,
    >::new_with_init(env!("CARGO_PKG_NAME"))
    .with_default_duration_s(60)
    .use_setup(setup)
    .use_agent_setup(agent_setup)
    .use_agent_behaviour(agent_behaviour)
    .use_agent_teardown(|ctx| {
        uninstall_app(ctx, None).ok();
        Ok(())
    });

    run(builder)?;

    Ok(())
}
