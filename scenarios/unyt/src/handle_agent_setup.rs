use super::handle_scenario_setup::ScenarioValues;
use crate::durable_object::DurableObject;
use crate::unyt_agent::UnytAgentExt;
use anyhow::Context;
use holochain_types::prelude::{AgentPubKey, YamlProperties};
use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_bytes;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::Instant;

pub fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    start_conductor_and_configure_urls(ctx)?;

    let assigned_behaviour = ctx.assigned_behaviour().to_string();
    // if ctx.agent_name().contains("agent-0") {
    if assigned_behaviour == "initiate" {
        log::info!("Installing app for initiator agent pubkey (Progenitor)");
        let progenitor_agent_pubkey = generate_progenitor(ctx)?;
        let prop = serde_yaml::to_value(serde_json::json!({
            "progenitor_agent_pubkey": progenitor_agent_pubkey.to_string(),
        }))?;
        log::info!("DNA properties: {:?}", prop);
        let dna_properties = HashMap::from([("alliance".to_string(), YamlProperties::new(prop))]);
        custom_install_app_from_bytes(
            ctx,
            scenario_happ_bytes!("unyt"),
            &"alliance".to_string(),
            Some(progenitor_agent_pubkey),
            Some(dna_properties),
        )?;
    } else {
        log::info!("Installing app for participant agent pubkey");
        let progenitor_agent_pubkey = DurableObject::new().get_progenitor_key(ctx)?;
        let prop = serde_yaml::to_value(serde_json::json!({
            "progenitor_agent_pubkey": progenitor_agent_pubkey.to_string(),
        }))?;
        log::info!("DNA properties: {:?}", prop);
        let dna_properties = HashMap::from([("alliance".to_string(), YamlProperties::new(prop))]);
        custom_install_app_from_bytes(
            ctx,
            scenario_happ_bytes!("unyt"),
            &"alliance".to_string(),
            None,
            Some(dna_properties),
        )?;
    }
    try_wait_for_min_agents(ctx, Duration::from_secs(120))?;

    ctx.unyt_init()?;

    log::info!(
        "Agent setup complete for {}, with agent pub key {:?}, dna hash {:?}",
        ctx.agent_name(),
        ctx.get().cell_id().agent_pubkey(),
        ctx.get().cell_id().dna_hash()
    );

    // Every agent creates a code template to flag that they have joined the network
    let _ = ctx.unyt_create_flag_template()?;

    // Note: get all code template and check the author of them as possible agents to transact with
    // I think we should generate the list based on the behaviour of the agent
    // so in some cases we should only have one agent to transact with
    // and in other cases we should have all agents to transact with
    ctx.get_mut().scenario_values.session_start_time = Some(Instant::now());
    Ok(())
}

fn generate_progenitor(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> Result<AgentPubKey, anyhow::Error> {
    // Generate a progenitor agent pubkey
    let admin_ws_url = ctx.get().admin_ws_url();
    let reporter = ctx.runner_context().reporter();
    let progenitor_agent_pubkey = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move {
            log::debug!("Connecting a Holochain admin client: {}", admin_ws_url);
            let admin_client = AdminWebsocket::connect(admin_ws_url, reporter)
                .await
                .context("Unable to connect admin client")?;
            let agent_pubkey = admin_client
                .generate_agent_pub_key()
                .await
                .map_err(handle_api_err)?;
            Ok(agent_pubkey)
        })
        .context("Failed to set up app port")?;
    log::info!(
        "Generated progenitor agent pubkey: {:?}",
        progenitor_agent_pubkey
    );

    // Create DurableObject instance and post the progenitor key
    let durable_object = DurableObject::new();

    // Use a unique run_id - you might want to get this from scenario configuration
    let run_id = ctx.runner_context().get_run_id().to_string();
    let progenitor_key_str = progenitor_agent_pubkey.to_string();

    // Post the progenitor key
    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            durable_object
                .post_progenitor_key(&run_id, &progenitor_key_str)
                .await
        })
        .context("Failed to post progenitor key to DurableObject")?;

    Ok(progenitor_agent_pubkey)
}
