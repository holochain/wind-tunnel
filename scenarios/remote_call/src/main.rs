use trycp_wind_tunnel_runner::prelude::*;

fn agent_setup(ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>) -> HookResult {
    connect_trycp_client(ctx)?;

    reset_trycp_remote(ctx)?;

    Ok(())
}

fn agent_behaviour(ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>) -> HookResult {
    let client = ctx.get().trycp_client();

    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            client
                .configure_player("test".to_string(), "".to_string(), None)
                .await?;

            client.reset(None).await?;

            Ok(())
        })?;

    Ok(())
}

fn agent_teardown(ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>) -> HookResult {
    disconnect_trycp_client(ctx)?;
    Ok(())
}

fn main() -> WindTunnelResult<()> {
    let builder =
        TryCPScenarioDefinitionBuilder::<TryCPRunnerContext, TryCPAgentContext>::new_with_init(
            env!("CARGO_PKG_NAME"),
        )?
        .into_std()
        .use_agent_setup(agent_setup)
        .use_agent_behaviour(agent_behaviour)
        .use_agent_teardown(agent_teardown);

    run(builder)?;

    Ok(())
}
