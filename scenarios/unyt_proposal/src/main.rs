mod behaviours;
mod values;

use self::behaviours::common::ProposalConfig;
use self::values::ScenarioValues;
use holochain_wind_tunnel_runner::happ_path;
use holochain_wind_tunnel_runner::prelude::*;
use rave_engine::types::UnitDefinition;
use rave_engine::types::UnytType;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use wind_tunnel_unyt_scenario::{ArcType, UnytScenarioValues, unyt_agent::UnytAgentExt};

const NETWORK_INIT_TIMEOUT: Duration = Duration::from_secs(180);
const ZERO_USER: &str = "zero_user";

/// The arc type each named behaviour runs with.
fn arc_type_for(behaviour: &str) -> ArcType {
    match behaviour {
        ZERO_USER => ArcType::Zero,
        _ => ArcType::Full,
    }
}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    // Parse and validate the environment configuration first.
    ctx.get_mut().scenario_values.config = Some(ProposalConfig::from_env()?);

    wind_tunnel_unyt_scenario::setup::common_agent_setup(ctx, happ_path!("unyt"), &[ZERO_USER])?;

    // Wait for network initialization
    // The progenitor initializes the network in its behaviour, so it must not wait here.
    if ctx.assigned_behaviour() != "initiate" {
        // Other behaviors should wait for network initialization.
        let agent_key = ctx.get().cell_id().agent_pubkey().to_string();
        let waiting_since = Instant::now();
        while !ctx.is_network_initialized() {
            if ctx.shutdown_listener().should_shutdown() {
                anyhow::bail!("Shutdown while waiting for network init");
            }
            if waiting_since.elapsed() > NETWORK_INIT_TIMEOUT {
                anyhow::bail!(
                    "Network was not initialized within {} s",
                    NETWORK_INIT_TIMEOUT.as_secs()
                );
            }
            log::info!("[{agent_key}] network not initialized, waiting");
            thread::sleep(Duration::from_secs(2));
        }

        // The clock is started by `common_agent_setup` right after this agent joins the
        // network, so this measures how long the global definition took to reach it.
        let session_started_at = ctx
            .get()
            .scenario_values
            .session_start_time()
            .ok_or_else(|| anyhow::anyhow!("`session_started_at` not set"))?;
        let arc_type = arc_type_for(ctx.assigned_behaviour());
        log::info!("[{arc_type}-arc] Network initialized for agent {agent_key}");
        ctx.runner_context().reporter().add_custom(
            ReportMetric::new("global_definition_propagation_time")
                .with_field("value", session_started_at.elapsed().as_secs())
                .with_tag("agent", agent_key)
                .with_tag("arc", arc_type.as_tag()),
        );
        ctx.get_mut().scenario_values.set_network_initialized(true);
    }

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
    .use_named_agent_behaviour("user", |ctx| {
        self::behaviours::user::agent_behaviour(ctx, ArcType::Full)
    })
    .use_named_agent_behaviour("zero_user", |ctx| {
        self::behaviours::user::agent_behaviour(ctx, ArcType::Zero)
    })
    .use_agent_teardown(wind_tunnel_unyt_scenario::behaviour::teardown::agent_teardown)
    .add_capture_env("UNYT_DURABLE_OBJECTS_URL")
    .add_capture_env("UNYT_PROPOSAL_WEIGHTS")
    .add_capture_env("UNYT_COMMITMENT_ACCEPT_PCT")
    .add_capture_env("UNYT_COUNTER_ADJUSTMENT_PCT")
    .add_capture_env("UNYT_MAX_NEGOTIATION_ROUNDS")
    .add_capture_env("UNYT_SPEND_FRACTION_PCT")
    .add_capture_env("MIN_AGENTS");

    run(builder)?;

    Ok(())
}
