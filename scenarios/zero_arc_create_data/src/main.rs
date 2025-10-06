use holochain_types::prelude::ActionHash;
use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;

#[derive(Debug, Default)]
struct ScenarioValues {
    call_count: u32,
}

impl UserValuesConstraint for ScenarioValues {}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    start_conductor_and_configure_urls(ctx)?;
    install_app(ctx, scenario_happ_path!("crud"), &"crud".to_string())?;

    Ok(())
}

fn additional_agents_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let full_arc_config = ctx
        .get()
        .holochain_config()
        .expect("Failed to build HolochainConfig");

    // zero arc config
    let mut zero_arc_config = full_arc_config.clone();
    zero_arc_config.conductor_config.network.target_arc_factor = 0;

    // start 4 (more) full arc conductors
    for i in 0..4 {
        let setup = run_holochain_conductor_with_config(
            ctx.runner_context().executor(),
            HolochainConductorSetupConfig {
                holochain_config: full_arc_config.clone(),
                agent_name: format!("{agent_name}_full_{i}", agent_name = ctx.agent_name()),
                run_id: ctx.runner_context().get_run_id().to_string(),
            },
        )?;
        // get app ws url
        let app_ws_url = get_app_ws_url(
            ctx.runner_context().executor(),
            ctx.runner_context().reporter(),
            setup.admin_ws_url,
        )?;
        // install app
        install_app_at(
            ctx.runner_context().executor(),
            InstallAppConfig {
                app_path: scenario_happ_path!("crud"),
                role_name: "crud".to_string(),
                admin_ws_url: setup.admin_ws_url,
                app_ws_url,
                installed_app_id: format!("crud_full_{i}"),
                reporter: ctx.runner_context().reporter(),
                run_id: ctx.runner_context().get_run_id().to_string(),
            },
        )?;
    }

    // start 5 zero arc conductors
    for i in 0..5 {
        let setup = run_holochain_conductor_with_config(
            ctx.runner_context().executor(),
            HolochainConductorSetupConfig {
                holochain_config: zero_arc_config.clone(),
                agent_name: format!("{agent_name}_zero_{i}", agent_name = ctx.agent_name()),
                run_id: ctx.runner_context().get_run_id().to_string(),
            },
        )?;
        // get app ws url
        let app_ws_url = get_app_ws_url(
            ctx.runner_context().executor(),
            ctx.runner_context().reporter(),
            setup.admin_ws_url,
        )?;
        // install app
        install_app_at(
            ctx.runner_context().executor(),
            InstallAppConfig {
                app_path: scenario_happ_path!("crud"),
                role_name: "crud".to_string(),
                admin_ws_url: setup.admin_ws_url,
                app_ws_url,
                installed_app_id: format!("crud_zero_{i}"),
                reporter: ctx.runner_context().reporter(),
                run_id: ctx.runner_context().get_run_id().to_string(),
            },
        )?;
    }

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
    .use_agent_setup(agent_setup)
    .use_agent_behaviour(agent_behaviour)
    .use_additional_agents_setup(additional_agents_setup)
    .use_agent_teardown(|ctx| {
        uninstall_app(ctx, None).ok();
        Ok(())
    });

    run(builder)?;

    Ok(())
}
