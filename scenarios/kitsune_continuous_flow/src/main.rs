use kitsune_wind_tunnel_runner::prelude::*;
use rand::Rng;
use std::time::Duration;

fn agent_setup(ctx: &mut AgentContext<KitsuneRunnerContext, KitsuneAgentContext>) -> HookResult {
    create_chatter(ctx)?;
    join_chatter_network(ctx)
}

fn behavior(
    ctx: &mut AgentContext<KitsuneRunnerContext, KitsuneAgentContext>,
) -> anyhow::Result<()> {
    // Determine number of messages per interval.
    // Configurable with env var `NUM_MESSAGES`.
    // Defaults to 3.
    let number_of_messages = std::env::var("NUM_MESSAGES")
        .unwrap_or("3".to_string())
        .parse()
        .expect("NUM_MESSAGES must be a number < 256");
    // Create messages.
    let mut messages = Vec::with_capacity(number_of_messages as usize);
    let timestamp = std::time::UNIX_EPOCH
        .elapsed()
        .expect("time went backwards")
        .as_millis();
    for i in 0..number_of_messages {
        let message = format!("message_{}_{}_{}", ctx.agent_index(), timestamp, i);
        messages.push(message);
    }
    // Say messages.
    say(ctx, messages)?;
    // Wait a random amount of time between 10 and 1000 ms.
    let mut rng = rand::thread_rng();
    let interval = rng.gen_range(10..1000);
    ctx.runner_context().executor().execute_in_place(async {
        tokio::time::sleep(Duration::from_millis(interval)).await;
        Ok(())
    })
}

fn main() -> WindTunnelResult<()> {
    let builder =
        KitsuneScenarioDefinitionBuilder::<KitsuneRunnerContext, KitsuneAgentContext>::new_with_init(
            "kitsune_continuous_flow",
        )?.into_std()
        .use_agent_setup(agent_setup)
        .use_agent_behaviour(behavior)
        .with_default_duration_s(30);
    run(builder)?;
    Ok(())
}
