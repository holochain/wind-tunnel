use kitsune_wind_tunnel_runner::prelude::*;
use std::time::Duration;

fn agent_setup(ctx: &mut AgentContext<KitsuneRunnerContext, KitsuneAgentContext>) -> HookResult {
    create_chatter(ctx)?;
    join_chatter_space(ctx)
}

fn behavior(
    ctx: &mut AgentContext<KitsuneRunnerContext, KitsuneAgentContext>,
) -> anyhow::Result<()> {
    let message = format!(
        "message_{}_{}",
        ctx.agent_index(),
        std::time::UNIX_EPOCH
            .elapsed()
            .expect("time went backwards")
            .as_millis()
    );
    say(ctx, &message)?;
    ctx.runner_context().executor().execute_in_place(async {
        tokio::time::sleep(Duration::from_secs(5)).await;
        Ok(())
    })
}

fn main() -> WindTunnelResult<()> {
    let builder =
        KitsuneScenarioDefinitionBuilder::<KitsuneRunnerContext, KitsuneAgentContext>::new_with_init(
            "kitsune",
        )?.into_std()
        .use_agent_setup(agent_setup)
        .use_agent_behaviour(behavior);
    run(builder)?;
    Ok(())
}
