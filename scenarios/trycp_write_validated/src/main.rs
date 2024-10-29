use holochain_types::prelude::ActionHash;
use std::time::Duration;
use trycp_wind_tunnel_runner::embed_conductor_config;
use trycp_wind_tunnel_runner::prelude::*;
use validated_integrity::UpdateSampleEntryInput;

embed_conductor_config!();

fn agent_setup(ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>) -> HookResult {
    connect_trycp_client(ctx)?;
    reset_trycp_remote(ctx)?;

    let client = ctx.get().trycp_client();
    let agent_name = ctx.agent_name().to_string();

    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            client
                .configure_player(agent_name.clone(), conductor_config(), None)
                .await?;

            client.startup(agent_name.clone(), None).await?;

            Ok(())
        })?;

    install_app(
        ctx,
        scenario_happ_path!("validated"),
        &"validated".to_string(),
    )?;
    try_wait_for_min_peers(ctx, Duration::from_secs(120))?;

    Ok(())
}

fn agent_behaviour(ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>) -> HookResult {
    let action_hash: ActionHash = call_zome(
        ctx,
        "validated",
        "create_sample_entry",
        "this is a test entry value",
        Some(Duration::from_secs(80)),
    )?;

    let _: ActionHash = call_zome(
        ctx,
        "validated",
        "update_sample_entry",
        UpdateSampleEntryInput {
            original: action_hash,
            new_value: "the old string was a bit boring".to_string(),
        },
        Some(Duration::from_secs(80)),
    )?;

    Ok(())
}

fn agent_teardown(ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>) -> HookResult {
    if let Err(e) = dump_logs(ctx) {
        log::warn!("Failed to dump logs: {:?}", e);
    }

    // Best effort to remove data and cleanup.
    // You should comment out this line if you want to examine the result of the scenario run!
    let _ = reset_trycp_remote(ctx);

    // Alternatively, you can just shut down the remote conductor instead of shutting it down and removing data.
    // shutdown_remote(ctx)?;

    disconnect_trycp_client(ctx)?;

    Ok(())
}

fn main() -> WindTunnelResult<()> {
    let builder =
        TryCPScenarioDefinitionBuilder::<TryCPRunnerContext, TryCPAgentContext>::new_with_init(
            env!("CARGO_PKG_NAME"),
        )?
        .into_std()
        .with_default_duration_s(180)
        .use_agent_setup(agent_setup)
        .use_agent_behaviour(agent_behaviour)
        .use_agent_teardown(agent_teardown);

    run_with_required_agents(builder, 1)?;

    Ok(())
}
