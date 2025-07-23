mod behaviour;
mod handle_agent_setup;
mod handle_scenario_setup;
use handle_scenario_setup::ScenarioValues;
use holochain_wind_tunnel_runner::prelude::*;

fn main() -> WindTunnelResult<()> {
    log::info!("Starting domino scenario");
    let builder = ScenarioDefinitionBuilder::<
        HolochainRunnerContext,
        HolochainAgentContext<ScenarioValues>,
    >::new_with_init(env!("CARGO_PKG_NAME"))
    .use_setup(handle_scenario_setup::setup)
    .use_agent_setup(handle_agent_setup::agent_setup)
    .use_named_agent_behaviour("initiate", behaviour::initiate_network::agent_behaviour)
    .use_named_agent_behaviour("participate", behaviour::initiate_network::agent_behaviour)
    .use_agent_teardown(|ctx| {
        uninstall_app(ctx, None).ok();
        Ok(())
    });

    run(builder)?;

    Ok(())
}
