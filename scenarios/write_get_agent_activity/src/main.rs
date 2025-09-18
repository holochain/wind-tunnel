use holochain_types::prelude::ActionHash;
use holochain_types::prelude::AgentActivity;
use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;
use std::time::Instant;

fn setup(ctx: &mut RunnerContext<HolochainRunnerContext>) -> HookResult {
    configure_app_ws_url(ctx)?;
    Ok(())
}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>,
) -> HookResult {
    install_app(
        ctx,
        scenario_happ_path!("agent_activity"),
        &"agent_activity".to_string(),
    )?;

    Ok(())
}

fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>,
) -> HookResult {
    let reporter = ctx.runner_context().reporter();

    // Create an entry
    let _: ActionHash = call_zome(
        ctx,
        "agent_activity",
        "create_sample_entry",
        "this is a test entry value",
    )?;

    // Get my own agent activity
    let now = Instant::now();
    let activity: AgentActivity = call_zome(
        ctx,
        "agent_activity",
        "get_agent_activity_full",
        ctx.get().cell_id().agent_pubkey(),
    )?;
    let elapsed = now.elapsed();

    reporter.add_custom(
        ReportMetric::new("write_get_agent_activity")
            .with_field(
                "highest_observed_action_seq",
                activity.highest_observed.map_or(0, |v| v.action_seq),
            )
            .with_field("value", elapsed.as_secs_f64()),
    );

    Ok(())
}

fn main() -> WindTunnelResult<()> {
    let builder =
        ScenarioDefinitionBuilder::<HolochainRunnerContext, HolochainAgentContext>::new_with_init(
            env!("CARGO_PKG_NAME"),
        )
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
