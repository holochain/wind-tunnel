use holochain_types::prelude::ActionHash;
use holochain_wind_tunnel_runner::happ_path;
use holochain_wind_tunnel_runner::prelude::*;

#[derive(Debug, Default)]
struct ScenarioValues {
    call_count: u32,
}

impl UserValuesConstraint for ScenarioValues {}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    start_conductor_and_configure_urls(ctx)?;
    install_app(ctx, happ_path!("crud"), &"crud".to_string())?;

    Ok(())
}

fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let _: ActionHash = call_zome(
        ctx,
        "crud",
        "create_sample_entry",
        "this is a test entry value",
    )?;

    let response: u32 = call_zome(ctx, "crud", "chain_query_count_len", ())?;

    let values = &mut ctx.get_mut().scenario_values;
    values.call_count += 1;

    // Minimal check that we're querying the right content and getting the expected result from the
    // calculation in this zome function.
    assert_eq!(
        values.call_count * 26,
        response,
        "Expected call count to match response"
    );

    Ok(())
}

fn main() -> WindTunnelResult<()> {
    let builder = ScenarioDefinitionBuilder::<
        HolochainRunnerContext,
        HolochainAgentContext<ScenarioValues>,
    >::new_with_init(env!("CARGO_PKG_NAME"))
    .with_default_duration_s(60)
    .use_build_info(conductor_build_info)
    .use_agent_setup(agent_setup)
    .use_agent_behaviour(agent_behaviour)
    .use_agent_teardown(|ctx| {
        uninstall_app(ctx, None).ok();
        Ok(())
    });

    run(builder)?;

    Ok(())
}
