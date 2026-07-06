use holochain_wind_tunnel_runner::happ_path;
use holochain_wind_tunnel_runner::prelude::*;
use wind_tunnel_unyt_scenario::ArcType;
use wind_tunnel_unyt_scenario::CommonScenarioValues;

pub type ScenarioValues = CommonScenarioValues;

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    wind_tunnel_unyt_scenario::setup::common_agent_setup(ctx, happ_path!("unyt"), &[])
}

fn main() -> WindTunnelResult<()> {
    log::info!("Starting Unyt Chain Transaction scenario");
    let builder = ScenarioDefinitionBuilder::<
        HolochainRunnerContext,
        HolochainAgentContext<ScenarioValues>,
    >::new_with_init(env!("CARGO_PKG_NAME"))
    .use_agent_setup(agent_setup)
    .use_named_agent_behaviour("initiate", |ctx| {
        wind_tunnel_unyt_scenario::behaviour::initiate_network::agent_behaviour(ctx, Vec::new())
    })
    .use_named_agent_behaviour("spend", |ctx| {
        wind_tunnel_unyt_scenario::behaviour::spend::agent_behaviour(ctx, ArcType::Full)
    })
    .use_named_agent_behaviour("smart_agreements", |ctx| {
        wind_tunnel_unyt_scenario::behaviour::smart_agreements::agent_behaviour(ctx, ArcType::Full)
    })
    .use_agent_teardown(wind_tunnel_unyt_scenario::behaviour::teardown::agent_teardown)
    .add_capture_env("UNYT_NUMBER_OF_LINKS_TO_PROCESS")
    .add_capture_env("UNYT_DURABLE_OBJECTS_URL");

    run(builder)?;

    Ok(())
}
