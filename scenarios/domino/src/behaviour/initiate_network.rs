use std::{thread, time::Duration};

use crate::{domino_agent::DominoAgentExt, handle_scenario_setup::ScenarioValues};
use holochain_wind_tunnel_runner::prelude::*;

pub fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    // check if network is initialized, if not initialize it
    if !ctx.is_network_initialized() {
        // todo: create system code templates
        // todo: set global configuration
    } else {
        // else just pause since there is nothing else for this agent to do,
        // since the network is initialized
        thread::sleep(Duration::from_secs(1));
    }
    Ok(())
}
