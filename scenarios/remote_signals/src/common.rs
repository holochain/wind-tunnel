use anyhow::Context;
use holochain_types::prelude::*;
use rand::seq::SliceRandom;
use rand::thread_rng;
use remote_signal_integrity::TimedMessage;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use trycp_wind_tunnel_runner::prelude::*;

#[derive(Debug)]
pub struct ScenarioValues {
    pub signal_interval: Duration,
    pub response_timeout: Duration,
    pub remote_signal_peers: Vec<AgentPubKey>,
    pub pending_set: Arc<Mutex<HashSet<TimedMessage>>>,
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

impl AsMut<ScenarioValues> for ScenarioValues {
    fn as_mut(&mut self) -> &mut ScenarioValues {
        self
    }
}

impl UserValuesConstraint for ScenarioValues {}

pub fn agent_setup_post_startup_pre_install_hook<Sv>(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<Sv>>,
) -> HookResult
where
    Sv: UserValuesConstraint + AsMut<ScenarioValues>,
{
    let client = ctx.get().trycp_client();
    let reporter = ctx.runner_context().reporter();
    let pending_set = ctx.get_mut().scenario_values.as_mut().pending_set.clone();

    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            let _rcv_task = tokio::task::spawn(async move {
                loop {
                    let msg: Signal = ExternIO(
                        client
                            .recv_signal()
                            .await
                            .expect("signal receiver ended")
                            .data,
                    )
                    .decode()
                    .unwrap();
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
                        TimedMessage::TimedResponse { requested_at, .. } => {
                            let dispatch_time_s = requested_at.as_micros() as f64 / 1_000_000.0;
                            let receive_time_s = Timestamp::now().as_micros() as f64 / 1_000_000.0;

                            reporter.add_custom(
                                ReportMetric::new("remote_signal_round_trip")
                                    .with_field("value", receive_time_s - dispatch_time_s),
                            );
                        }
                    }
                }
            });

            Ok(())
        })?;

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
    let next_remote_signal_peer = ctx
        .get_mut()
        .scenario_values
        .as_mut()
        .remote_signal_peers
        .pop();
    let signal_interval = ctx.get_mut().scenario_values.as_mut().signal_interval;
    let response_timeout = ctx.get_mut().scenario_values.as_mut().response_timeout;
    let pending_set = ctx.get_mut().scenario_values.as_mut().pending_set.clone();
    let reporter = ctx.runner_context().reporter();

    let new_peers = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move {
            let now = Timestamp::now();

            pending_set.lock().unwrap().retain(|msg| {
                if now.as_millis() - msg.requested_at().as_millis()
                    > response_timeout.as_millis() as i64
                {
                    reporter.add_custom(
                        ReportMetric::new("remote_signal_timeout").with_field("value", 1),
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
        .as_mut()
        .remote_signal_peers
        .extend(new_peers);

    Ok(())
}
