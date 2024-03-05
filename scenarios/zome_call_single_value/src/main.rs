use anyhow::Context;
use holochain_types::prelude::{ExternIO};
use holochain_wind_tunnel_runner::prelude::*;
use std::path::Path;
use std::sync::Arc;
use holochain_wind_tunnel_runner::scenario_happ_path;

fn setup(ctx: &mut RunnerContext<HolochainRunnerContext>) -> HookResult {
    configure_app_ws_url(ctx)?;
    Ok(())
}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>,
) -> HookResult {
    install_app(
        ctx,
        scenario_happ_path!("return_single_value"),
        &"return_single_value".to_string(),
    )?;

    Ok(())
}

fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>,
) -> HookResult {
    let cell_id = ctx.get().cell_id();
    let mut app_agent_client = ctx.get().app_agent_client();
    ctx.runner_context().executor().execute_in_place(async {
        app_agent_client
            .call_zome(
                cell_id.into(),
                "return_single_value".into(),
                "get_value".into(),
                ExternIO::encode(()).context("Encoding failure")?,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Conductor API error: {:?}", e))?;

        Ok(())
    })?;

    Ok(())
}

fn agent_teardown(
    _ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>,
) -> HookResult {
    println!("Shutdown hook");

    Ok(())
}

fn teardown(_ctx: Arc<RunnerContext<HolochainRunnerContext>>) -> HookResult {
    println!("Tearing down the scenario");

    Ok(())
}

fn main() -> WindTunnelResult<()> {
    let builder = ScenarioDefinitionBuilder::<HolochainRunnerContext, HolochainAgentContext>::new(
        env!("CARGO_PKG_NAME"),
    )
    .with_default_duration_s(60)
    .use_setup(setup)
    .use_agent_setup(agent_setup)
    .use_agent_behaviour(agent_behaviour)
    .use_agent_teardown(agent_teardown)
    .use_teardown(teardown);

    run(builder)?;

    Ok(())
}
