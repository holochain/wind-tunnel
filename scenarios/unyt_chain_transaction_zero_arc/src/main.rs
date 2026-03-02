use holochain_wind_tunnel_runner::happ_path;
use holochain_wind_tunnel_runner::prelude::*;
use wind_tunnel_unyt_scenario::{ArcType, CommonScenarioValues};

pub type ScenarioValues = CommonScenarioValues;

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    wind_tunnel_unyt_scenario::setup::common_agent_setup(
        ctx,
        happ_path!("unyt"),
        &["zero_spend", "zero_smart_agreements", "zero_observer"],
    )
}

fn main() -> WindTunnelResult<()> {
    log::info!("Starting Unyt Chain Transaction Zero Arc scenario");
    let builder = ScenarioDefinitionBuilder::<
        HolochainRunnerContext,
        HolochainAgentContext<ScenarioValues>,
    >::new_with_init(env!("CARGO_PKG_NAME"))
    .use_agent_setup(agent_setup)
    .use_named_agent_behaviour(
        "initiate",
        wind_tunnel_unyt_scenario::initiate_network::agent_behaviour,
    )
    .use_named_agent_behaviour("zero_spend", |ctx| {
        wind_tunnel_unyt_scenario::behaviour::spend::agent_behaviour(ctx, Some(ArcType::Zero))
    })
    .use_named_agent_behaviour("zero_smart_agreements", |ctx| {
        wind_tunnel_unyt_scenario::behaviour::smart_agreements::agent_behaviour(
            ctx,
            Some(ArcType::Zero),
        )
    })
    .use_named_agent_behaviour("full_observer", |ctx| {
        wind_tunnel_unyt_scenario::behaviour::observer::agent_behaviour(ctx, ArcType::Full)
    })
    .use_named_agent_behaviour("zero_observer", |ctx| {
        wind_tunnel_unyt_scenario::behaviour::observer::agent_behaviour(ctx, ArcType::Zero)
    })
    .use_agent_teardown(wind_tunnel_unyt_scenario::behaviour::teardown::agent_teardown)
    .add_capture_env("NUMBER_OF_LINKS_TO_PROCESS")
    .add_capture_env("MIN_AGENTS");

    run(builder)?;

    Ok(())
}
