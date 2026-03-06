//! Scenario setup helpers for Unyt agent provisioning.
//!
//! Contains utilities for generating the progenitor agent key and
//! building the DNA role settings required to install the hApp with
//! the correct progenitor configuration.

use crate::UnytScenarioValues;
use crate::durable_object::DurableObject;
use crate::unyt_agent::UnytAgentExt;
use anyhow::Context;
use holochain_types::{
    app::RoleSettings,
    prelude::{AgentPubKey, DnaModifiersOpt, YamlProperties},
};
use holochain_wind_tunnel_runner::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Builds the DNA role settings map for the "alliance" role.
///
/// Embeds `progenitor_agent_pubkey` into the DNA properties so every
/// agent installs the hApp with the same progenitor configuration.
///
/// # Errors
///
/// Returns an error if the YAML properties cannot be serialized.
pub fn create_role_settings(
    progenitor_agent_pubkey: &AgentPubKey,
) -> anyhow::Result<HashMap<String, RoleSettings>> {
    let dna_properties = serde_yaml::to_value(HashMap::from([(
        "progenitor_agent_pubkey".to_string(),
        progenitor_agent_pubkey.to_string(),
    )]))?;
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

/// Generates a progenitor agent key and publishes it.
///
/// Connects to the Holochain admin websocket, generates a new agent
/// public key, then posts it to the [`DurableObject`] so that all
/// other agents in the run can retrieve it.
///
/// # Errors
///
/// Returns an error if the admin websocket connection, key
/// generation, or Durable Object POST fails.
pub fn generate_progenitor<SV: UnytScenarioValues>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
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

/// Shared agent setup logic for Unyt scenarios.
///
/// Handles conductor startup, hApp installation (with progenitor key
/// embedding), agent discovery, and flag template creation.
///
/// Behaviours listed in `zero_arc_behaviours` are configured with
/// `target_arc_factor(0)` before the conductor starts, and will wait
/// for a full-arc peer to be discovered after setup completes.
pub fn common_agent_setup<SV: UnytScenarioValues>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    happ_path: PathBuf,
    zero_arc_behaviours: &[&str],
) -> HookResult {
    let assigned_behaviour = ctx.assigned_behaviour().to_string();

    // Configure 0-arc for designated behaviours before starting the conductor
    if zero_arc_behaviours.contains(&assigned_behaviour.as_str()) {
        ctx.get_mut()
            .holochain_config_mut()
            .with_target_arc_factor(0);
    }

    start_conductor_and_configure_urls(ctx)?;

    if assigned_behaviour == "initiate" {
        log::info!("Installing app for initiator agent pubkey (Progenitor)");
        let progenitor_agent_pubkey = generate_progenitor(ctx)?;
        let role_settings = create_role_settings(&progenitor_agent_pubkey)?;
        let role_name = String::from("alliance");
        install_app_custom(
            ctx,
            happ_path,
            &role_name,
            Some(progenitor_agent_pubkey),
            Some(role_settings),
        )?;
    } else {
        log::info!("Installing app for participant agent pubkey");
        let progenitor_agent_pubkey = DurableObject::new().get_progenitor_key(ctx)?;
        let role_settings = create_role_settings(&progenitor_agent_pubkey)?;
        let role_name = String::from("alliance");
        install_app_custom(ctx, happ_path, &role_name, None, Some(role_settings))?;
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
    ctx.unyt_create_flag_template()?;

    // Wait for full-arc peer if this agent is 0-arc
    if zero_arc_behaviours.contains(&assigned_behaviour.as_str()) {
        try_wait_until_full_arc_peer_discovered(ctx)?;
    }

    ctx.get_mut()
        .scenario_values
        .set_session_start_time(tokio::time::Instant::now());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use holochain_types::prelude::AgentPubKey;

    /// Builds a deterministic `AgentPubKey` from 32 zero bytes.
    fn dummy_agent_pubkey() -> AgentPubKey {
        AgentPubKey::from_raw_32(vec![0u8; 32])
    }

    #[test]
    fn create_role_settings_contains_alliance_key() {
        let pubkey = dummy_agent_pubkey();
        let settings = create_role_settings(&pubkey).expect("should succeed");

        assert!(
            settings.contains_key("alliance"),
            "role settings must contain the 'alliance' key"
        );
        assert_eq!(settings.len(), 1, "should contain exactly one role");
    }

    #[test]
    fn create_role_settings_is_provisioned_with_properties() {
        let pubkey = dummy_agent_pubkey();
        let settings = create_role_settings(&pubkey).expect("should succeed");
        let alliance = settings.get("alliance").expect("missing alliance role");

        if let RoleSettings::Provisioned {
            membrane_proof,
            modifiers,
        } = alliance
        {
            assert!(membrane_proof.is_none(), "membrane_proof should be None");
            let modifiers = modifiers.as_ref().expect("modifiers should be Some");
            assert!(
                modifiers.network_seed.is_none(),
                "network_seed should be None"
            );
            assert!(
                modifiers.properties.is_some(),
                "properties should contain the progenitor key"
            );
        } else {
            panic!("expected Provisioned variant");
        }
    }

    #[test]
    fn create_role_settings_embeds_pubkey_in_properties() {
        let pubkey = dummy_agent_pubkey();
        let pubkey_str = pubkey.to_string();
        let settings = create_role_settings(&pubkey).expect("should succeed");
        let alliance = settings.get("alliance").expect("missing alliance role");

        if let RoleSettings::Provisioned { modifiers, .. } = alliance {
            let props = modifiers.as_ref().unwrap().properties.as_ref().unwrap();
            let yaml_value: serde_yaml::Value =
                serde_yaml::from_str(&serde_yaml::to_string(props).unwrap()).unwrap();

            let mapping = yaml_value.as_mapping().expect("should be a mapping");
            let stored = mapping
                .get(serde_yaml::Value::String(
                    "progenitor_agent_pubkey".to_string(),
                ))
                .expect("should contain progenitor_agent_pubkey");

            assert_eq!(
                stored.as_str().expect("should be a string"),
                pubkey_str,
                "embedded pubkey must match the input"
            );
        } else {
            panic!("expected Provisioned variant");
        }
    }
}
