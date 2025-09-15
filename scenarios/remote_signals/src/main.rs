use holochain_types::prelude::*;
use holochain_wind_tunnel_runner::{prelude::*, scenario_happ_path};
use remote_signal_integrity::TimedMessage;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

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
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    run_holochain_conductor(ctx)?;
    configure_admin_ws_url(ctx)?;
    configure_app_ws_url(ctx)?;
    install_app(
        ctx,
        scenario_happ_path!("remote_signal"),
        &"remote_signal".to_string(),
    )?;

    let pending_set = ctx.get().scenario_values.pending_set.clone();
    let reporter: Arc<Reporter> = ctx.runner_context().reporter();
    let client = ctx.get().app_client();

    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            let _rcv_task = tokio::task::spawn(async move {
                client
                    .on_signal(move |signal| {
                        let msg = match signal {
                            Signal::App { signal, .. } => {
                                let msg: Vec<u8> = signal.into_inner().decode().unwrap();
                                let msg: TimedMessage = ExternIO(msg).decode().unwrap();
                                msg
                            }
                            _ => return,
                        };

                        pending_set.lock().unwrap().remove(&msg.to_request());

                        match msg {
                            TimedMessage::TimedRequest { .. } => (),
                            TimedMessage::TimedResponse { requested_at, .. } => {
                                let dispatch_time_s = requested_at.as_micros() as f64 / 1_000_000.0;
                                let receive_time_s =
                                    Timestamp::now().as_micros() as f64 / 1_000_000.0;

                                reporter.add_custom(
                                    ReportMetric::new("remote_signal_round_trip")
                                        .with_field("value", receive_time_s - dispatch_time_s),
                                );
                            }
                        }
                    })
                    .await
                    .expect("signal receiver ended");
            });

            Ok(())
        })?;

    try_wait_for_min_agents(ctx, Duration::from_secs(120))?;

    Ok(())
}

fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let cell_id = ctx.get().cell_id();
    let next_remote_signal_peer = ctx.get_mut().scenario_values.remote_signal_peers.pop();
    let signal_interval = ctx.get().scenario_values.signal_interval;
    let response_timeout = ctx.get().scenario_values.response_timeout;
    let pending_set = ctx.get().scenario_values.pending_set.clone();
    let reporter = ctx.runner_context().reporter();

    let now = Timestamp::now();

    let mut timeout_count = 0;

    pending_set.lock().unwrap().retain(|msg| {
        if now.as_millis() - msg.requested_at().as_millis() > response_timeout.as_millis() as i64 {
            timeout_count += 1;
            false
        } else {
            true
        }
    });

    while timeout_count > 0 {
        reporter.add_custom(ReportMetric::new("remote_signal_timeout").with_field("value", 1));
        timeout_count -= 1;
    }

    let new_peers = match next_remote_signal_peer {
        None => get_peer_list_randomized(ctx)?,
        Some(agent_pub_key) => {
            let msg = TimedMessage::TimedRequest {
                requester: cell_id.agent_pubkey().clone(),
                responder: agent_pub_key.clone(),
                requested_at: Timestamp::now(),
            };
            pending_set.lock().unwrap().insert(msg.clone());
            // Send a remote signal to this agent
            let _: () = call_zome(ctx, "remote_signal", "signal_request", msg)?;

            // Don't hammer with signals
            thread::sleep(signal_interval);

            // Add no new agents, that should only happen when we exhaust the list.
            Vec::with_capacity(0)
        }
    };

    ctx.get_mut()
        .scenario_values
        .remote_signal_peers
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
