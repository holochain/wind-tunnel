use anyhow::Context;
use holochain_types::prelude::AgentPubKey;
use rand::seq::SliceRandom;
use rand::thread_rng;
use remote_call_integrity::TimedResponse;
use std::time::{Duration, Instant};
use trycp_wind_tunnel_runner::embed_conductor_config;
use trycp_wind_tunnel_runner::prelude::*;

embed_conductor_config!();

#[derive(Debug, Default)]
pub struct ScenarioValues {
    remote_call_peers: Vec<AgentPubKey>,
}

impl UserValuesConstraint for ScenarioValues {}

fn agent_setup(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<ScenarioValues>>,
) -> HookResult {
    connect_trycp_client(ctx)?;
    reset_trycp_remote(ctx)?;

    let client = ctx.get().trycp_client();
    let agent_name = ctx.agent_name().to_string();

    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            client
                .configure_player(agent_name.clone(), conductor_config().to_string(), None)
                .await?;

            client.startup(agent_name.clone(), None).await?;

            Ok(())
        })?;

    install_app(
        ctx,
        scenario_happ_path!("remote_call"),
        &"remote_call".to_string(),
    )?;
    try_wait_for_min_peers(ctx, Duration::from_secs(120))?;

    Ok(())
}

fn agent_behaviour(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<ScenarioValues>>,
) -> HookResult {
    let client = ctx.get().trycp_client();

    let agent_name = ctx.agent_name().to_string();
    let app_port = ctx.get().app_port();
    let cell_id = ctx.get().cell_id();
    let next_remote_call_peer = ctx.get_mut().scenario_values.remote_call_peers.pop();
    let reporter = ctx.runner_context().reporter();

    let new_peers = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move {
            match next_remote_call_peer {
                None => {
                    // No more agents available to call, get a new list.
                    // This is also the initial condition.
                    let mut new_peer_list = client
                        .agent_info(agent_name, None, None)
                        .await
                        .context("Failed to get agent info")?
                        .into_iter()
                        .map(|info| AgentPubKey::from_raw_36(info.agent.0.clone()))
                        .filter(|k| k != cell_id.agent_pubkey()) // Don't call ourselves!
                        .collect::<Vec<_>>();
                    new_peer_list.shuffle(&mut thread_rng());
                    Ok(new_peer_list)
                }
                Some(agent_pub_key) => {
                    // Send a remote call to this agent
                    let start = Instant::now();
                    let response = client
                        .call_zome(
                            app_port,
                            cell_id,
                            "remote_call",
                            "call_echo_timestamp",
                            agent_pub_key.clone(),
                            // Better to keep this higher than the Kitsune timeout so that when this fails we get a
                            // clear error back, rather than timing out here.
                            Some(Duration::from_secs(80)),
                        )
                        .await
                        .with_context(|| {
                            format!("Failed to make remote call to: {:?}", agent_pub_key)
                        })?;
                    let round_trip_time_s = start.elapsed();

                    let response: TimedResponse = response
                        .decode()
                        .map_err(|e| anyhow::anyhow!("Decoding failure: {:?}", e))?;

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
                    Ok(Vec::with_capacity(0))
                }
            }
        })?;

    ctx.get_mut()
        .scenario_values
        .remote_call_peers
        .extend(new_peers);

    Ok(())
}

fn agent_teardown(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<ScenarioValues>>,
) -> HookResult {
    if let Err(e) = dump_logs(ctx) {
        log::warn!("Failed to dump logs: {:?}", e);
    }

    // Best effort to remove data and cleanup.
    // You should comment out this line if you want to examine the result of the scenario run!
    let _ = reset_trycp_remote(ctx);

    // Alternatively, you can just shut down the remote conductor instead of shutting it down and removing data.
    // shutdown_remote(ctx)?;

    disconnect_trycp_client(ctx)?;

    Ok(())
}

fn main() -> WindTunnelResult<()> {
    let builder = TryCPScenarioDefinitionBuilder::<
        TryCPRunnerContext,
        TryCPAgentContext<ScenarioValues>,
    >::new_with_init(env!("CARGO_PKG_NAME"))?
    .into_std()
    .use_agent_setup(agent_setup)
    .use_agent_behaviour(agent_behaviour)
    .use_agent_teardown(agent_teardown);

    let agents_at_completion = run(builder)?;

    println!("Finished with {} agents", agents_at_completion);

    Ok(())
}
