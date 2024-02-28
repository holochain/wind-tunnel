use std::{sync::Arc, time::Duration};

use wind_tunnel_runner::prelude::*;

#[derive(Default, Debug)]
pub struct HolochainRunnerContext {
    value: usize,
}

impl UserValuesConstraint for HolochainRunnerContext {}

fn setup(ctx: &mut RunnerContext<HolochainRunnerContext>) -> HookResult {
    println!("Setting up the scenario");

    ctx.get_mut().value = 42;

    Ok(())
}

#[derive(Default, Debug)]
pub struct HolochainContext {
    value: String,
}

impl UserValuesConstraint for HolochainContext {}

fn agent_setup(ctx: &mut Context<HolochainRunnerContext, HolochainContext>) -> HookResult {
    ctx.get_mut().value = "Hello, world!".to_string();

    Ok(())
}

fn agent_behaviour(ctx: &mut Context<HolochainRunnerContext, HolochainContext>) -> HookResult {
    println!(
        "Agent behaviour, {}, {}",
        ctx.runner_context().get().value,
        ctx.get().value
    );

    let mut shutdown_listener = ctx.shutdown_listener().clone();
    ctx.runner_context().executor().execute(async {
        loop {
            tokio::select! {
                _ = shutdown_listener.wait_for_shutdown() => {
                    println!("Agent should shutdown");
                    break;
                }
                _ = tokio::time::sleep(Duration::from_secs(1)) => {
                    println!("Agent running");
                }
            }
        }
    });
    
    Ok(())
}

fn agent_teardown(_ctx: &mut Context<HolochainRunnerContext, HolochainContext>) -> HookResult {
    println!("Shutdown hook");

    Ok(())
}

fn teardown(_ctx: Arc<RunnerContext<HolochainRunnerContext>>) -> HookResult {
    println!("Tearing down the scenario");

    Ok(())
}

fn main() -> WindTunnelResult {
    let builder = ScenarioDefinitionBuilder::<HolochainRunnerContext, HolochainContext>::new(env!(
        "CARGO_PKG_NAME"
    ))
    .with_default_duration(10)
    .use_setup(setup)
    .use_agent_setup(agent_setup)
    .use_agent_behaviour(agent_behaviour)
    .use_agent_teardown(agent_teardown)
    .use_teardown(teardown);

    run(builder)?;

    Ok(())
}
