use super::handle_scenario_setup::ScenarioValues;
use holochain_types::prelude::YamlProperties;
use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;
use std::collections::HashMap;
use std::time::Duration;

pub fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let progenitor_agent_pubkey = ctx.runner_context().get().progenitor_agent_pubkey();
    let prop = serde_yaml::to_value(serde_json::json!({
        "progenitor_agent_pubkey": progenitor_agent_pubkey.to_string(),
    }))?;
    log::info!("DNA properties: {:?}", prop);

    let dna_properties = HashMap::from([("alliance".to_string(), YamlProperties::new(prop))]);

    let assigned_behaviour = ctx.assigned_behaviour().to_string();
    if assigned_behaviour == "initiate" {
        log::info!("Installing app for initiator agent pubkey (Progenitor)");
        install_app(
            ctx,
            scenario_happ_path!("domino"),
            &"alliance".to_string(),
            Some(progenitor_agent_pubkey),
            Some(dna_properties),
        )?;
    } else {
        log::info!("Installing app for participant agent pubkey");
        install_app(
            ctx,
            scenario_happ_path!("domino"),
            &"alliance".to_string(),
            None,
            Some(dna_properties),
        )?;
    }
    try_wait_for_min_agents(ctx, Duration::from_secs(120))?;

    call_zome::<_, String, _>(ctx, "alliance", "init", ())?;

    log::debug!(
        "Agent setup complete for {}, with agent pub key {:?}",
        ctx.agent_name(),
        ctx.get().cell_id().agent_pubkey()
    );

    Ok(())
}
