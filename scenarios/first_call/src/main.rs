use holochain_types::prelude::ActionHash;
use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;

#[derive(Debug, Default)]
struct ScenarioValues {
    admin_client: Option<AdminWebsocket>,
}

impl UserValuesConstraint for ScenarioValues {}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    start_conductor_and_configure_urls(ctx)?;
    let admin_url = ctx.get().admin_ws_url();
    let reporter = ctx.runner_context().reporter();
    let admin_client = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move {
            let admin_client = AdminWebsocket::connect(admin_url, reporter).await?;
            Ok(admin_client)
        })?;

    ctx.get_mut().scenario_values.admin_client = Some(admin_client);

    Ok(())
}

fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    // Will log a warning on the first run, but makes it easier to run the scenario multiple times
    uninstall_app(ctx, None)?;
    install_app(ctx, scenario_happ_path!("crud"), &"crud".into())?;
    let _: ActionHash = call_zome(ctx, "crud", "create_sample_entry", "a value")?;

    Ok(())
}

fn main() -> WindTunnelResult<()> {
    let builder = ScenarioDefinitionBuilder::<
        HolochainRunnerContext,
        HolochainAgentContext<ScenarioValues>,
    >::new_with_init(env!("CARGO_PKG_NAME"))
    .with_default_duration_s(180)
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
