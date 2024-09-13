use anyhow::Context;
use holochain_types::prelude::*;
use rand::seq::SliceRandom;
use rand::thread_rng;
use remote_signal_integrity::TimedMessage;
use std::time::Duration;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use trycp_wind_tunnel_runner::embed_conductor_config;
use trycp_wind_tunnel_runner::prelude::*;

embed_conductor_config!();

#[derive(Debug)]
pub struct ScenarioValues {
    signal_interval: Duration,
    response_timeout: Duration,
    remote_signal_peers: Vec<AgentPubKey>,
    pending_set: Arc<Mutex<HashSet<TimedMessage>>>,
}

fn env_dur(n: &'static str, d: u64) -> Duration {
    match std::env::var(n) {
        Ok(n) => Duration::from_millis(n.parse::<u64>().unwrap()),
        _ => Duration::from_millis(d),
    }
}

impl Default for ScenarioValues {
    fn default() -> Self {
        let signal_interval = env_dur("SIGNAL_INTERVAL_MS", 1_000);
        let response_timeout = env_dur("RESPONSE_TIMEOUT_MS", 20_000);

        Self {
            signal_interval,
            response_timeout,
            remote_signal_peers: Vec::new(),
            pending_set: Arc::new(Mutex::new(HashSet::new())),
        }
    }
}

impl UserValuesConstraint for ScenarioValues {}

fn agent_setup(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<ScenarioValues>>,
) -> HookResult {
    connect_trycp_client(ctx)?;
    reset_trycp_remote(ctx)?;

    let pending_set = ctx.get().scenario_values.pending_set.clone();

    let client = ctx.get().trycp_client();
    let agent_name = ctx.agent_name().to_string();
    let reporter = ctx.runner_context().reporter();

    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            client
                .configure_player(agent_name.clone(), conductor_config().to_string(), None)
                .await?;

            client.startup(agent_name.clone(), None).await?;

            let _rcv_task = tokio::task::spawn(async move {
                loop {
                    match client.recv_signal().await {
                        None => panic!("signal receiver ended"),
                        Some(signal) => {
                            let msg: Signal = ExternIO(signal.data).decode().unwrap();
                            let msg = match msg {
                                Signal::App { signal, .. } => {
                                    let msg: Vec<u8> = signal.into_inner().decode().unwrap();
                                    let msg: TimedMessage = ExternIO(msg).decode().unwrap();
                                    msg
                                }
                                _ => continue,
                            };

                            pending_set.lock().unwrap().remove(&msg.to_request());

                            match msg {
                                TimedMessage::TimedRequest { .. } => (),
                                TimedMessage::TimedResponse {
                                    requested_at,
                                    ..
                                } => {
                                    let dispatch_time_s = requested_at.as_micros() as f64 / 1_000_000.0;
                                    let receive_time_s = Timestamp::now().as_micros() as f64 / 1_000_000.0;

                                    reporter.add_custom(
                                        ReportMetric::new("remote_signal_round_trip")
                                            .with_field("value", receive_time_s - dispatch_time_s),
                                    );
                                }
                            }
                        }
                    }
                }
            });

            Ok(())
        })?;

    install_app(
        ctx,
        scenario_happ_path!("remote_signal"),
        &"remote_signal".to_string(),
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
    let next_remote_signal_peer = ctx.get_mut().scenario_values.remote_signal_peers.pop();
    let signal_interval = ctx.get().scenario_values.signal_interval;
    let response_timeout = ctx.get().scenario_values.response_timeout;
    let pending_set = ctx.get().scenario_values.pending_set.clone();
    let reporter = ctx.runner_context().reporter();

    let new_peers = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move {
            let now = Timestamp::now();

            pending_set.lock().unwrap().retain(|msg| {
                if now.as_millis() - msg.requested_at().as_millis() > response_timeout.as_millis() as i64 {
                    reporter.add_custom(
                        ReportMetric::new("remote_signal_timeout")
                            .with_field("value", 1),
                    );
                    false
                } else {
                    true
                }
            });

            match next_remote_signal_peer {
                None => {
                    // No more agents available to signal, get a new list.
                    // This is also the initial condition.
                    let mut new_peer_list = client
                        .agent_info(agent_name, None, None)
                        .await
                        .context("Failed to get agent info")?
                        .into_iter()
                        .map(|info| AgentPubKey::from_raw_36(info.agent.0.clone()))
                        .filter(|k| k != cell_id.agent_pubkey()) // Don't signal ourselves!
                        .collect::<Vec<_>>();
                    new_peer_list.shuffle(&mut thread_rng());
                    Ok(new_peer_list)
                }
                Some(agent_pub_key) => {
                    let msg = TimedMessage::TimedRequest {
                        requester: cell_id.agent_pubkey().clone(),
                        responder: agent_pub_key.clone(),
                        requested_at: Timestamp::now(),
                    };
                    pending_set.lock().unwrap().insert(msg.clone());
                    // Send a remote signal to this agent
                    client
                        .call_zome(
                            app_port,
                            cell_id.clone(),
                            "remote_signal",
                            "signal_request",
                            msg,
                            // Better to keep this higher than the Kitsune timeout so that when this fails we get a
                            // clear error back, rather than timing out here.
                            Some(Duration::from_secs(80)),
                        )
                        .await
                        .with_context(|| {
                            format!("Failed to make remote signal to: {:?}", agent_pub_key)
                        })?;

                    // Don't hammer with signals
                    tokio::time::sleep(signal_interval).await;

                    // Add no new agents, that should only happen when we exhaust the list.
                    Ok(Vec::with_capacity(0))
                }
            }
        })?;

    ctx.get_mut()
        .scenario_values
        .remote_signal_peers
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
