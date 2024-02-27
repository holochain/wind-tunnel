use wind_tunnel_runner::prelude::*;

#[derive(Default)]
pub struct HolochainRunnerContext {
    value: usize,
}

impl UserValuesConstraint for HolochainRunnerContext {}

fn setup(ctx: &mut RunnerContext<HolochainRunnerContext>) -> HookResult {
    println!("Setting up the scenario");

    ctx.get_mut().value = 42;

    Ok(())
}

#[derive(Default)]
pub struct HolochainContext {
    value: String,
}

impl UserValuesConstraint for HolochainContext {}

fn agent_setup(ctx: &mut Context<HolochainRunnerContext, HolochainContext>) -> HookResult {
    println!("Setting up the agent");

    ctx.get_mut().value = "Hello, world!".to_string();

    Ok(())
}

fn agent_behaviour(ctx: &mut Context<HolochainRunnerContext, HolochainContext>) -> HookResult {
    println!(
        "Agent behaviour, {}, {}",
        ctx.runner_context().get().value,
        ctx.get().value
    );

    Ok(())
}

fn main() -> WindTunnelResult {
    let builder = ScenarioDefinitionBuilder::<HolochainRunnerContext, HolochainContext>::new(env!(
        "CARGO_PKG_NAME"
    ))
    .use_setup(setup)
    .use_agent_setup(agent_setup)
    .use_agent_behaviour(agent_behaviour);

    run(builder)?;

    Ok(())
}
