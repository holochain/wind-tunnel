use crate::{domino_agent::DominoAgentExt, handle_scenario_setup::ScenarioValues};
use holochain_wind_tunnel_runner::prelude::*;

pub fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let reporter = ctx.runner_context().reporter();
    let start = ctx.get().scenario_values.session_start_time.unwrap();

    let code_templates = ctx.domino_get_code_templates_lib()?;
    // collecte unity authors of the code templates
    let unique_agents = code_templates
        .iter()
        .map(|template| template.author.clone())
        .collect::<Vec<_>>();

    // TODO: write test
    // if we find more than one report it
    if code_templates.len() > 1 {
        // log::info!("More than one code template found, reporting");
        reporter.add_custom(
            ReportMetric::new("number_of_code_templates_found")
                .with_field("count", code_templates.len() as u64)
                .with_tag(
                    "searching_agent",
                    ctx.get().cell_id().agent_pubkey().to_string(),
                )
                .with_tag("timestamp", start.elapsed().as_secs()),
        );
    }

    Ok(())
}
