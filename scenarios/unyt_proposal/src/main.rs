//! Negotiated Unyt transactions with explicit full-arc and zero-arc workload roles.
//!
//! `initiate` initializes the network on a full arc. `full_propose` and
//! `zero_propose` share the proposer workload; `full_respond` and `zero_respond`
//! share the responder workload. Zero-arc roles rely on full-arc storage peers.

mod behaviours;
mod values;

use holochain_wind_tunnel_runner::happ_path;
use holochain_wind_tunnel_runner::prelude::*;
use rave_engine::types::UnitDefinition;
use rave_engine::types::UnytType;
use wind_tunnel_unyt_scenario::telemetry::record_open_connections_if_necessary;

use self::values::ScenarioValues;

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    wind_tunnel_unyt_scenario::setup::common_agent_setup(
        ctx,
        happ_path!("unyt"),
        &["zero_propose", "zero_respond"],
    )
}

fn record_open_connections(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let previous = ctx.get().scenario_values.open_connections_last_recorded;
    let next = record_open_connections_if_necessary(ctx, previous)?;
    ctx.get_mut().scenario_values.open_connections_last_recorded = next;
    Ok(())
}

fn main() -> WindTunnelResult<()> {
    log::info!("Starting Unyt Proposal scenario");
    let builder = ScenarioDefinitionBuilder::<
        HolochainRunnerContext,
        HolochainAgentContext<ScenarioValues>,
    >::new_with_init(env!("CARGO_PKG_NAME"))
    .use_agent_setup(agent_setup)
    .use_named_agent_behaviour("initiate", |ctx| {
        record_open_connections(ctx)?;
        wind_tunnel_unyt_scenario::behaviour::initiate_network::agent_behaviour(
            ctx,
            // Define two service units globally, so proposals can be bidirectional. The happ
            // adds these to the global definition's `service_units` at indices
            // ("0", "1"). Unit 0 mirrors the base unit the happ would create by default.
            // Unit 1 lets a proposal send value, which is what makes
            // the acceptor create a receipt, exercising the full commit -> accept -> receipt
            // flow.
            vec![
                UnitDefinition {
                    unit_type: UnytType::default(),
                    unit_symbol: "ZF".to_string(),
                    unit_name: "Fuel".to_string(),
                    unit_description: "Base Unit".to_string(),
                    unit_color: "#02b4b3".to_string(),
                },
                UnitDefinition {
                    unit_type: UnytType::default(),
                    unit_symbol: "SU1".to_string(),
                    unit_name: "Service Unit 1".to_string(),
                    unit_description: "Secondary unit for bidirectional proposals".to_string(),
                    unit_color: "#02b4b3".to_string(),
                },
            ],
        )
    })
    .use_named_agent_behaviour("full_propose", |ctx| {
        record_open_connections(ctx)?;
        self::behaviours::propose::agent_behaviour(ctx)
    })
    .use_named_agent_behaviour("zero_propose", |ctx| {
        record_open_connections(ctx)?;
        self::behaviours::propose::agent_behaviour(ctx)
    })
    .use_named_agent_behaviour("full_respond", |ctx| {
        record_open_connections(ctx)?;
        self::behaviours::respond::agent_behaviour(ctx)
    })
    .use_named_agent_behaviour("zero_respond", |ctx| {
        record_open_connections(ctx)?;
        self::behaviours::respond::agent_behaviour(ctx)
    })
    .use_agent_teardown(wind_tunnel_unyt_scenario::behaviour::teardown::agent_teardown)
    .add_capture_env("UNYT_DURABLE_OBJECTS_URL")
    .add_capture_env("UNYT_PROPOSER_WEIGHTS")
    .add_capture_env("UNYT_RESPONDER_WEIGHTS")
    .add_capture_env("UNYT_COMMITMENT_ACCEPT_PCT")
    .add_capture_env("UNYT_COUNTER_ADJUSTMENT_PCT")
    .add_capture_env("UNYT_MAX_NEGOTIATION_ROUNDS")
    .add_capture_env("MIN_AGENTS");

    run(builder)?;

    Ok(())
}
