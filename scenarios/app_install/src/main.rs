use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;
use std::path::PathBuf;

#[derive(Debug, Default)]
struct ScenarioValues {
    admin_client: Option<AdminWebsocket>,
}

impl UserValuesConstraint for ScenarioValues {}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    run_holochain_conductor(ctx)?;
    configure_app_ws_url(ctx)?;
    let admin_url = ctx.runner_context().get_connection_string();
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

fn agent_behaviour_minimal(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    install_app_behaviour(ctx, scenario_happ_path!("callback"), "callback")?;

    Ok(())
}

fn agent_behaviour_large(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    install_app_behaviour(ctx, scenario_happ_path!("large"), "large")?;
    Ok(())
}

fn install_app_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    happ_path: PathBuf,
    happ_name: &str,
) -> HookResult {
    // Will log a warning on the first run, but makes it easier to run the scenario multiple times
    uninstall_app(ctx, None).ok();
    install_app(ctx, happ_path, &happ_name.to_string())?;

    Ok(())
}

fn main() -> WindTunnelResult<()> {
    let builder = ScenarioDefinitionBuilder::<
        HolochainRunnerContext,
        HolochainAgentContext<ScenarioValues>,
    >::new_with_init(env!("CARGO_PKG_NAME"))
    .with_default_duration_s(120)
    .use_agent_setup(agent_setup)
    .use_named_agent_behaviour("minimal", agent_behaviour_minimal)
    .use_named_agent_behaviour("large", agent_behaviour_large)
    .use_agent_teardown(|ctx| {
        uninstall_app(ctx, None).ok();
        Ok(())
    });

    run(builder)?;

    Ok(())
}
