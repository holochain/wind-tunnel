use holochain_types::prelude::AgentActivity;
use holochain_types::prelude::{ActionHash, AgentPubKey};
use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;
use kitsune2_api::AgentInfoSigned;
use kitsune2_core::Ed25519Verifier;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub struct ScenarioValues {
    write_peer: Option<AgentPubKey>,
}

impl UserValuesConstraint for ScenarioValues {}

fn setup(ctx: &mut RunnerContext<HolochainRunnerContext>) -> HookResult {
    configure_app_ws_url(ctx)?;
    Ok(())
}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    install_app(
        ctx,
        scenario_happ_path!("agent_activity"),
        &"agent_activity".to_string(),
    )?;

    try_wait_for_min_agents(ctx, Duration::from_secs(120))?;

    Ok(())
}

fn agent_behaviour_write(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let _: ActionHash = call_zome(
        ctx,
        "agent_activity",
        "create_sample_entry",
        "this is a test entry value",
    )?;
    Ok(())
}

fn agent_behaviour_get_agent_activity(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let reporter = ctx.runner_context().reporter();
    let saved_write_peer = ctx.get_mut().scenario_values.write_peer.clone();

    match saved_write_peer {
        Some(write_peer) => {
            let now = Instant::now();
            let activity: AgentActivity = call_zome(
                ctx,
                "agent_activity",
                "get_agent_activity_full",
                write_peer.clone(),
            )?;
            let elapsed = now.elapsed();

            reporter.add_custom(
                ReportMetric::new("write_get_agent_activity")
                    .with_tag("agent", write_peer.to_string())
                    .with_field(
                        "highest_observed_action_seq",
                        activity.highest_observed.map_or(0, |v| v.action_seq),
                    )
                    .with_field("value", elapsed.as_secs_f64()),
            );
        }
        _ => {
            let client = ctx.get().app_client();
            let cell_id = ctx.get().cell_id();
            let new_peers: Vec<AgentPubKey> =
                ctx.runner_context()
                    .executor()
                    .execute_in_place(async move {
                        let agent_infos_encoded: Vec<String> = client.agent_info(None).await?;

                        let mut agent_infos = Vec::new();
                        for info in agent_infos_encoded {
                            let a = AgentInfoSigned::decode(&Ed25519Verifier, info.as_bytes())?;
                            agent_infos.push(AgentPubKey::from_k2_agent(&a.agent))
                        }
                        let peer_list = agent_infos
                            .into_iter()
                            .filter(|k| k != cell_id.agent_pubkey()) // Don't include ourselves!
                            .collect::<Vec<_>>();

                        Ok(peer_list)
                    })?;

            ctx.get_mut().scenario_values.write_peer = new_peers.first().map(|p| p.clone());
        }
    }

    Ok(())
}

fn main() -> WindTunnelResult<()> {
    let builder = ScenarioDefinitionBuilder::<
        HolochainRunnerContext,
        HolochainAgentContext<ScenarioValues>,
    >::new_with_init(env!("CARGO_PKG_NAME"))
    .with_default_duration_s(60)
    .use_setup(setup)
    .use_agent_setup(agent_setup)
    .use_named_agent_behaviour("write", agent_behaviour_write)
    .use_named_agent_behaviour("get_agent_activity", agent_behaviour_get_agent_activity)
    .use_agent_teardown(|ctx| {
        uninstall_app(ctx, None).ok();
        Ok(())
    });

    run(builder)?;

    Ok(())
}
