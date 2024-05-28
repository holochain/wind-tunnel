use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;
use std::path::{Path, PathBuf};
use std::time::Instant;

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
    let start = Instant::now();
    install_app(ctx, happ_path, &happ_name.to_string())?;
    let install_time_s = start.elapsed().as_secs_f64();

    uninstall_app(ctx, None)?;

    let metric = ReportMetric::new("app_install")
        .with_tag("happ", happ_name.to_string())
        .with_field("value", install_time_s);
    ctx.runner_context().reporter().clone().add_custom(metric);

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
    .use_named_agent_behaviour("minimal", agent_behaviour_minimal)
    .use_named_agent_behaviour("large", agent_behaviour_large)
    .use_agent_teardown(|ctx| {
        uninstall_app(ctx, None).ok();
        Ok(())
    });

    run(builder)?;

    Ok(())
}
