use crate::handle_scenario_setup::ScenarioValues;
use holochain_wind_tunnel_runner::prelude::*;

pub fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    Ok(())
}
