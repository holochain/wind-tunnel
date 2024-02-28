use anyhow::Context;
use holochain_client_instrumented::prelude::AdminWebsocket;
use holochain_wind_tunnel_runner::prelude::*;
use std::{sync::Arc, time::Duration};

fn setup(ctx: &mut RunnerContext<HolochainRunnerContext>) -> HookResult {
    println!("Setting up the scenario");

    let _client = ctx
        .executor()
        .execute_in_place(async move {
            log::info!("Connecting a Holochain admin client");
            AdminWebsocket::connect("ws://localhost:8888".to_string()).await
        })
        .context("Failed to connect the Holochain admin client")?;

    // TODO install an app!

    ctx.get_mut().value = 42;

    Ok(())
}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>,
) -> HookResult {
    ctx.get_mut().value = "Hello, world!".to_string();

    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            log::info!("Connecting a Holochain admin client");
            let mut client = AdminWebsocket::connect("ws://localhost:8888".to_string()).await?;

            let key = client
                .generate_agent_pub_key()
                .await
                .map_err(|e| anyhow::anyhow!("Conductor API error: {:?}", e))?;

            log::info!("Generated agent pub key: {:}", key);

            Ok(())
        })
        .context("Failed to generate agent pub key")?;

    Ok(())
}

fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>,
) -> HookResult {
    println!(
        "Agent behaviour, {}, {}",
        ctx.runner_context().get().value,
        ctx.get().value
    );

    ctx.runner_context().executor().execute_in_place(async {
        tokio::time::sleep(Duration::from_secs(1)).await;
        println!("Agent running");
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

fn main() -> WindTunnelResult {
    let builder = ScenarioDefinitionBuilder::<HolochainRunnerContext, HolochainAgentContext>::new(
        env!("CARGO_PKG_NAME"),
    )
    .with_default_duration(10)
    .use_setup(setup)
    .use_agent_setup(agent_setup)
    .use_agent_behaviour(agent_behaviour)
    .use_agent_teardown(agent_teardown)
    .use_teardown(teardown);

    run(builder)?;

    Ok(())
}
