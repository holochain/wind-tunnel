use super::ScenarioValues;
use crate::durable_object::DurableObject;
use crate::unyt_agent::UnytAgentExt;
use anyhow::Context;
use holochain_types::{
    app::RoleSettings,
    prelude::{AgentPubKey, DnaModifiersOpt, YamlProperties},
};
use holochain_wind_tunnel_runner::happ_path;
use holochain_wind_tunnel_runner::prelude::*;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::Instant;

fn create_role_settings(
    progenitor_agent_pubkey: &AgentPubKey,
) -> anyhow::Result<HashMap<String, RoleSettings>> {
    let dna_properties = serde_yaml::to_value(serde_json::json!({
        "progenitor_agent_pubkey": progenitor_agent_pubkey.to_string(),
    }))?;
    log::info!("DNA properties: {:?}", dna_properties);
    let role_settings = HashMap::from([(
        "alliance".to_string(),
        RoleSettings::Provisioned {
            membrane_proof: None,
            modifiers: Some(DnaModifiersOpt {
                network_seed: None,
                properties: Some(YamlProperties::new(dna_properties)),
            }),
        },
    )]);

    Ok(role_settings)
}

pub fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    start_conductor_and_configure_urls(ctx)?;

    let assigned_behaviour = ctx.assigned_behaviour().to_string();
    if assigned_behaviour == "initiate" {
        log::info!("Installing app for initiator agent pubkey (Progenitor)");
        let progenitor_agent_pubkey = generate_progenitor(ctx)?;
        let role_settings = create_role_settings(&progenitor_agent_pubkey)?;
        install_app_custom(
            ctx,
            happ_path!("unyt"),
            &"alliance".to_string(),
            Some(progenitor_agent_pubkey),
            Some(role_settings),
        )?;
    } else {
        log::info!("Installing app for participant agent pubkey");
        let progenitor_agent_pubkey = DurableObject::new().get_progenitor_key(ctx)?;
        let role_settings = create_role_settings(&progenitor_agent_pubkey)?;
        install_app_custom(
            ctx,
            happ_path!("unyt"),
            &"alliance".to_string(),
            None,
            Some(role_settings),
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
    let admin_ws_url = ctx.get().admin_ws_url();
    let reporter = ctx.runner_context().reporter();
    let progenitor_agent_pubkey = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move {
            log::debug!("Connecting a Holochain admin client: {}", admin_ws_url);
            let admin_client = AdminWebsocket::connect(admin_ws_url, None, reporter)
                .await
                .context("Unable to connect admin client")?;
            let agent_pubkey = admin_client.generate_agent_pub_key().await?;
            Ok(agent_pubkey)
        })
        .context("Failed to generate progenitor agent pubkey")?;
    log::info!(
        "Generated progenitor agent pubkey: {:?}",
        progenitor_agent_pubkey
    );

    // Post the progenitor key to the DurableObject so other agents can fetch it
    let durable_object = DurableObject::new();
    let run_id = ctx.runner_context().get_run_id().to_string();
    let progenitor_key_str = progenitor_agent_pubkey.to_string();
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
