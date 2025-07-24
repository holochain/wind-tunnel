use crate::{domino_agent::DominoAgentExt, handle_scenario_setup::ScenarioValues};
use holochain_wind_tunnel_runner::prelude::*;
use std::{thread, time::Duration};

pub fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let reporter = ctx.runner_context().reporter();
    let session_started_at = ctx.get().scenario_values.session_start_time.unwrap();
    let network_initialized = ctx.get().scenario_values.network_initialized;
    if !network_initialized {
        if ctx.is_network_initialized() {
            log::info!(
                "Network initialized for agent {}",
                ctx.get().cell_id().agent_pubkey()
            );
            reporter.add_custom(
                ReportMetric::new("network_initialized")
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

    // todo
    // let code_templates = ctx.domino_get_code_templates_lib()?;
    // // collecte unity authors of the code templates
    // let unique_agents = code_templates
    //     .iter()
    //     .map(|template| template.author.clone())
    //     .collect::<Vec<_>>();

    // // TODO: write test
    // // if we find more than one report it
    // if code_templates.len() > 1 {
    //     // log::info!("More than one code template found, reporting");
    //     reporter.add_custom(
    //         ReportMetric::new("number_of_code_templates_found")
    //             .with_field("count", code_templates.len() as u64)
    //             .with_tag(
    //                 "searching_agent",
    //                 ctx.get().cell_id().agent_pubkey().to_string(),
    //             )
    //             .with_tag("timestamp", start.elapsed().as_secs()),
    //     );
    // }

    Ok(())
}
