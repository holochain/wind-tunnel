use crate::{
    domino_agent::{DominoAgentExt, SpendInput},
    handle_scenario_setup::ScenarioValues,
};
use holochain_wind_tunnel_runner::prelude::*;
use rave_engine::types::Units;
use std::{collections::BTreeMap, str::FromStr, thread, time::Duration};
use zfuel::fuel::ZFuel;

pub fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let reporter = ctx.runner_context().reporter();
    let session_started_at = ctx.get().scenario_values.session_start_time.unwrap();
    let network_initialized = ctx.get().scenario_values.network_initialized;
    // Test 1
    if !network_initialized {
        if ctx.is_network_initialized() {
            log::info!(
                "Network initialized for agent {}",
                ctx.get().cell_id().agent_pubkey()
            );
            reporter.add_custom(
                ReportMetric::new("global_definition_propagation_time")
                    .with_field("at", session_started_at.elapsed().as_secs())
                    .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string()),
            );
            ctx.get_mut().scenario_values.network_initialized = true;
        } else {
            // if the network is not initialized do not proceed with further testing without waiting for it to be initialized
            log::info!(
                "Network not initialized for agent {}, waiting for it to be initialized",
                ctx.get().cell_id().agent_pubkey()
            );
            thread::sleep(Duration::from_secs(2));
            return Ok(());
        }
    }

    // test 2
    // collect agents and start transacting
    const MAX_NUMBER_OF_AGENTS_NEEDED: usize = 10;
    if ctx.get().scenario_values.participating_agents.len() < MAX_NUMBER_OF_AGENTS_NEEDED {
        let code_templates = ctx.domino_get_code_templates_lib()?;
        // collecte unity authors of the code templates
        let mut unique_agents = code_templates
            .iter()
            .map(|template| template.author.clone())
            .collect::<Vec<_>>();

        // remove yourself from the list
        let self_pubkey = ctx.get().cell_id().agent_pubkey().clone().into();
        unique_agents.retain(|agent| agent != &self_pubkey);
        ctx.get_mut().scenario_values.participating_agents = unique_agents
            .into_iter()
            .map(|agent| agent.into())
            .collect();
    }

    // spend with those agents
    let participating_agents = ctx.get().scenario_values.participating_agents.clone();
    let amount = Units::load(BTreeMap::from([("0".to_string(), ZFuel::from_str("1")?)]));
    for agent in participating_agents {
        let _ = ctx.domino_create_spend(SpendInput {
            receiver: agent,
            amount: amount.clone(),
            note: None,
            service_network_definition: None,
        })?;
    }

    thread::sleep(Duration::from_secs(2));

    // todo: check incoming transactions and accept them
    // todo

    Ok(())
}
