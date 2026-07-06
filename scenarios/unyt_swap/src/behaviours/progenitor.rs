use crate::lane;
use crate::values::ScenarioValues;
use holochain_wind_tunnel_runner::prelude::*;
use std::{thread, time::Duration};
use wind_tunnel_unyt_scenario::behaviour::initiate_network;
use wind_tunnel_unyt_scenario::durable_object::DurableObject;
use wind_tunnel_unyt_scenario::unyt_agent::UnytAgentExt;

/// Progenitor behaviour: bootstraps the network via the shared
/// `initiate_network` behaviour, then creates the lane once, using the bridge
/// agent's key fetched from the Durable Object.
pub fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    // Bootstrap the global definition until the network reports ready. Only the
    // base unit (index 0) is created; the lane defines its own units.
    if !ctx.is_network_initialized() {
        return initiate_network::agent_behaviour(ctx, Vec::new());
    }

    // Create the lane once; on later iterations the lane already exists.
    let lanes = ctx.unyt_get_all_lane()?;
    if lanes.is_empty() {
        let bridge_agent_key = DurableObject::new().get_bridge_agent_key(ctx)?;
        let lane_hash = lane::setup_lane(ctx, &bridge_agent_key)?;
        ctx.progenitor_init_alpha_env()?;
        log::info!("[progenitor] lane initialized: {lane_hash}");
    } else {
        log::info!("[progenitor] lane ready, idling");
        thread::sleep(Duration::from_secs(20));
    }

    Ok(())
}
