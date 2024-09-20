use anyhow::Context;
use holochain_types::prelude::AgentPubKey;
use rand::seq::SliceRandom;
use rand::thread_rng;
use remote_call_integrity::TimedResponse;
use std::time::{Duration, Instant};
use trycp_wind_tunnel_runner::prelude::*;

#[derive(Debug, Default)]
pub struct ScenarioValues {
    pub remote_call_peers: Vec<AgentPubKey>,
}

impl UserValuesConstraint for ScenarioValues {}

impl AsMut<ScenarioValues> for ScenarioValues {
    fn as_mut(&mut self) -> &mut ScenarioValues {
        self
    }
}

pub fn agent_setup_post_startup_pre_install_hook<Sv>(
    _ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<Sv>>,
) -> HookResult
where
    Sv: UserValuesConstraint + AsMut<ScenarioValues>,
{
    Ok(())
}

pub fn agent_behaviour_hook<Sv>(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<Sv>>,
) -> HookResult
where
    Sv: UserValuesConstraint + AsMut<ScenarioValues>,
{
    let client = ctx.get().trycp_client();

    let agent_name = ctx.agent_name().to_string();
    let app_port = ctx.get().app_port();
    let cell_id = ctx.get().cell_id();
    let next_remote_call_peer = ctx
        .get_mut()
        .scenario_values
        .as_mut()
        .remote_call_peers
        .pop();
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

                    let dispatch_time_s = response.request_value.as_micros() as f64 / 1_000_000.0;
                    let receive_time_s = response.value.as_micros() as f64 / 1_000_000.0;

                    reporter.add_custom(
                        ReportMetric::new("remote_call_dispatch")
                            .with_field("value", receive_time_s - dispatch_time_s),
                    );
                    reporter.add_custom(
                        ReportMetric::new("remote_call_round_trip")
                            .with_field("value", round_trip_time_s.as_secs_f64()),
                    );

                    // Add no new agents, that should only happen when we exhaust the list.
                    Ok(Vec::with_capacity(0))
                }
            }
        })?;

    ctx.get_mut()
        .scenario_values
        .as_mut()
        .remote_call_peers
        .extend(new_peers);

    Ok(())
}
