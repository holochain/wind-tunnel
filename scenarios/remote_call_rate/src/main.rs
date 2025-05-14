use anyhow::Context;
use holochain_types::prelude::AgentPubKey;
use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;
use rand::seq::SliceRandom;
use rand::thread_rng;
use remote_call_integrity::TimedResponse;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub struct ScenarioValues {
    remote_call_peers: Vec<AgentPubKey>,
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
        scenario_happ_path!("remote_call"),
        &"remote_call".to_string(),
    )?;
    try_wait_for_min_agents(ctx, Duration::from_secs(120))?;

    Ok(())
}

fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let admin_ws_url = ctx.runner_context().get_connection_string().to_string();
    let cell_id = ctx.get().cell_id();
    let next_remote_call_peer = ctx.get_mut().scenario_values.remote_call_peers.pop();
    let reporter = ctx.runner_context().reporter();
    let new_peers = match next_remote_call_peer {
        None => {
            ctx.runner_context().executor().execute_in_place(async {
                let admin_client = AdminWebsocket::connect(admin_ws_url, reporter.clone()).await?;
                // No more agents available to call, get a new list.
                // This is also the initial condition.
                let agent_infos_encoded = admin_client
                    .agent_info(None)
                    .await
                    .context("Failed to get agent info")?;
                let mut agent_infos = Vec::new();
                for info in agent_infos_encoded {
                    let a = kitsune2_api::AgentInfoSigned::decode(
                        &kitsune2_core::Ed25519Verifier,
                        info.as_bytes(),
                    )?;
                    agent_infos.push(AgentPubKey::from_raw_32(a.agent.to_vec()))
                }
                let mut new_peer_list = agent_infos
                    .into_iter()
                    .filter(|k| k != cell_id.agent_pubkey()) // Don't call ourselves!
                    .collect::<Vec<_>>();
                new_peer_list.shuffle(&mut thread_rng());
                Ok(new_peer_list)
            })?
        }
        Some(agent_pub_key) => {
            // Send a remote call to this agent
            let start = Instant::now();
            let response: TimedResponse = call_zome(
                ctx,
                "remote_call",
                "call_echo_timestamp",
                agent_pub_key.clone(),
            )
            .with_context(|| format!("Failed to make remote call to: {:?}", agent_pub_key))?;
            let round_trip_time_s = start.elapsed();

            let dispatch_time_s = response.request_value.as_micros() as f64 / 1e6;
            let receive_time_s = response.value.as_micros() as f64 / 1e6;

            reporter.add_custom(
                ReportMetric::new("remote_call_dispatch")
                    .with_tag("agent", agent_pub_key.to_string())
                    .with_field("value", receive_time_s - dispatch_time_s),
            );
            reporter.add_custom(
                ReportMetric::new("remote_call_round_trip")
                    .with_tag("agent", agent_pub_key.to_string())
                    .with_field("value", round_trip_time_s.as_secs_f64()),
            );

            // Add no new agents, that should only happen when we exhaust the list.
            Vec::with_capacity(0)
        }
    };

    ctx.get_mut()
        .scenario_values
        .remote_call_peers
        .extend(new_peers);

    Ok(())
}

fn main() -> WindTunnelResult<()> {
    let builder = ScenarioDefinitionBuilder::<
        HolochainRunnerContext,
        HolochainAgentContext<ScenarioValues>,
    >::new_with_init(env!("CARGO_PKG_NAME"))
    .use_setup(setup)
    .use_agent_setup(agent_setup)
    .use_agent_behaviour(agent_behaviour)
    .use_agent_teardown(|ctx| {
        uninstall_app(ctx, None).ok();
        Ok(())
    });

    run(builder)?;

    Ok(())
}
