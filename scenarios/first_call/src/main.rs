use holochain_types::prelude::ActionHash;
use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;
use std::path::Path;

#[derive(Debug, Default)]
struct ScenarioValues {
    admin_client: Option<AdminWebsocket>,
}

impl UserValuesConstraint for ScenarioValues {}

fn setup(ctx: &mut RunnerContext<HolochainRunnerContext>) -> HookResult {
    configure_app_ws_url(ctx)?;
    Ok(())
}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
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

fn agent_behaviour_local(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    install_app(ctx, scenario_happ_path!("crud"), &"crud".into())?;

    let _: ActionHash = call_zome(ctx, "crud", "create_sample_entry", "a value")?;

    uninstall_app(ctx)?;

    Ok(())
}

fn uninstall_app(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let admin_client = ctx.get().scenario_values.admin_client.as_ref().unwrap();
    let installed_app_id = ctx.get().installed_app_id();
    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            admin_client
                .uninstall_app(installed_app_id)
                .await
                .map_err(handle_api_err)?;
            Ok(())
        })
        // Discard the error here, the behaviour may uninstall the app. The state will depend when
        // the shutdown signal is received.
        .ok();

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
    .use_named_agent_behaviour("local", agent_behaviour_local)
    .use_agent_teardown(uninstall_app);

    run(builder)?;

    Ok(())
}
