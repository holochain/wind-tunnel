use anyhow::Context;
use holochain_types::prelude::AgentPubKey;
use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;
use remote_call_integrity::TimedResponse;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub struct ScenarioValues {
    remote_call_peers: Vec<AgentPubKey>,
}

impl UserValuesConstraint for ScenarioValues {}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    start_conductor_and_configure_urls(ctx)?;
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
    let next_remote_call_peer = ctx.get_mut().scenario_values.remote_call_peers.pop();
    let reporter = ctx.runner_context().reporter();
    let new_peers = match next_remote_call_peer {
        None => get_peer_list_randomized(ctx)?,
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
    .use_agent_setup(agent_setup)
    .use_agent_behaviour(agent_behaviour)
    .use_agent_teardown(|ctx| {
        uninstall_app(ctx, None).ok();
        Ok(())
    });

    run(builder)?;

    Ok(())
}
