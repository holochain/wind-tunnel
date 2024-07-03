use anyhow::Context;
use countersigning_integrity::Signals;
use holochain_types::prelude::{AgentPubKey, PreflightResponse};
use holochain_types::signal::Signal;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::ops::Add;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use trycp_wind_tunnel_runner::prelude::*;

const CONDUCTOR_CONFIG: &str = include_str!("../../../conductor-config.yaml");

#[derive(Debug, Default)]
pub struct ScenarioValues {
    initiate_with_peers: Vec<AgentPubKey>,
    session_attempts: Arc<AtomicUsize>,
    session_successes: Arc<AtomicUsize>,
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
                .configure_player(agent_name.clone(), CONDUCTOR_CONFIG.to_string(), None)
                .await?;

            client
                .startup(agent_name.clone(), Some("warn".to_string()), None)
                .await?;

            Ok(())
        })?;

    install_app(
        ctx,
        scenario_happ_path!("countersigning"),
        &"countersigning".to_string(),
    )?;
    try_wait_for_min_peers(ctx, Duration::from_secs(120))?;

    let client = ctx.get().trycp_client();
    let app_port = ctx.get().app_port();
    let cell_id = ctx.get().cell_id();
    let assigned_behaviour = ctx.assigned_behaviour().to_string();
    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            if assigned_behaviour == "initiate" {
                // As an initiator we just need to call a zome so that `init` will run.
                client
                    .call_zome(
                        app_port,
                        cell_id,
                        "countersigning",
                        "initiator_hello",
                        (),
                        None,
                    )
                    .await?;
            } else if assigned_behaviour == "participate" {
                // As a participant we need to advertise our role by publishing a link to our agent key
                client
                    .call_zome(
                        app_port,
                        cell_id,
                        "countersigning",
                        "participant_hello",
                        (),
                        None,
                    )
                    .await?;
            } else {
                return Err(anyhow::anyhow!(
                    "Unknown assigned behaviour: {}",
                    assigned_behaviour
                ));
            }

            Ok(())
        })?;

    log::debug!("Agent setup complete for: {}", ctx.agent_name().to_string());

    Ok(())
}

fn agent_behaviour_initiate(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<ScenarioValues>>,
) -> HookResult {
    let client = ctx.get().trycp_client();

    let app_port = ctx.get().app_port();
    let cell_id = ctx.get().cell_id();
    let initiate_with_peers = ctx.get_mut().scenario_values.initiate_with_peers.pop();
    let reporter = ctx.runner_context().reporter();

    let agent_name = ctx.agent_name().to_string();
    let initiated = ctx.get().scenario_values.session_attempts.clone();
    let initiated_success = ctx.get().scenario_values.session_successes.clone();

    let new_peers = ctx
        .runner_context()
        .executor()
        .execute_in_place(
            async move {
                match initiate_with_peers {
                    None => {
                        // No more agents available to call, get a new list.
                        // This is also the initial condition.
                        let response = client
                            .call_zome(
                                app_port,
                                cell_id.clone(),
                                "countersigning",
                                "list_participants",
                                (),
                                None,
                            )
                            .await
                            .context("Failed to list participants")?;
                        let mut new_peer_list = response.decode::<Vec<AgentPubKey>>().map_err(|e| anyhow::anyhow!("Decoding failure: {:?}", e))?;
                        new_peer_list.shuffle(&mut thread_rng());
                        Ok(new_peer_list)
                    }
                    Some(agent_pub_key) => {
                        log::debug!("Initiating a countersigning session with agent {:?}", agent_pub_key);

                        let start = Instant::now();
                        let initiated = initiated.fetch_add(1, std::sync::atomic::Ordering::Acquire);
                        reporter.add_custom(
                            ReportMetric::new("countersigning_session_initiated")
                                .with_tag("agent", agent_name.clone())
                                .with_field("value", initiated as i64),
                        );

                        // Start a countersigning session with the next agent in the list.
                        let response = client
                            .call_zome(
                                app_port,
                                cell_id.clone(),
                                "countersigning",
                                "start_two_party",
                                agent_pub_key.clone(),
                                // This should be fairly quick, can increase this if it causes problems
                                None,
                            )
                            .await
                            .with_context(|| {
                                format!("Failed to start a new countersigning session: {:?}", agent_pub_key)
                            })?;

                        let my_preflight_response: PreflightResponse = response.decode().map_err(|e| anyhow::anyhow!("Decoding failure: {:?}", e))?;

                        let session_timeout = Instant::now().add(Duration::from_millis((my_preflight_response.request.session_times.end.as_millis() - my_preflight_response.request.session_times.start.as_millis()) as u64));
                        loop {
                            // Now listen for a signal from the remote with their acceptance
                            let signal = tokio::time::timeout_at(session_timeout, client.recv_signal()).await.context("Agent did not respond to the countersigning request in time, abandoning")?;

                            match signal {
                                Some(signal) => {
                                    let signal = match rmp_serde::decode::from_slice::<Signal>(&signal.data).map_err(|e| anyhow::anyhow!("Decoding failure, appears to not be a signal: {:?}", e))? {
                                        Signal::App {
                                            signal,
                                            ..
                                        } => signal,
                                        _ => {
                                            log::debug!("Received a signal that is not an app signal, listening for other signals.");
                                            continue;
                                        }
                                    };

                                    let other_response = match signal.clone().into_inner().decode::<Signals>() {
                                        Ok(Signals::Response(response)) => response,
                                        Ok(_) => {
                                            log::debug!("Received a signal that is not a response, listening for other signals.");
                                            continue;
                                        }
                                        Err(_) => {
                                            // Must be resilient to unexpected signals, somebody else might try to initiate with us while we're already
                                            // working with another peer.
                                            log::debug!("Got an unexpected signal, will try again. {:?}", signal);
                                            continue;
                                        }
                                    };

                                    log::debug!("The other party [{:?}] has accepted the countersigning session.", agent_pub_key);

                                    client.call_zome(
                                        app_port,
                                        cell_id.clone(),
                                        "countersigning",
                                        "commit_two_party",
                                        vec![my_preflight_response, other_response],
                                        None,
                                    ).await.context("Initiator failed to commit countersigned entry")?;
                                    let elapsed = start.elapsed();

                                    log::debug!("Completed the countersigning session with agent {:?}", agent_pub_key);

                                    let initiated_success = initiated_success.fetch_add(1, std::sync::atomic::Ordering::Acquire);
                                    reporter.add_custom(
                                        ReportMetric::new("countersigning_session_initiated_success")
                                            .with_tag("agent", agent_name.clone())
                                            .with_field("value", initiated_success as i64),
                                    );
                                    reporter.add_custom(
                                        ReportMetric::new("countersigning_session_initiated_duration")
                                            .with_tag("agent", agent_name)
                                            .with_field("value", elapsed.as_secs_f64()),
                                    );

                                    break;
                                }
                                None => {
                                    log::warn!("No signal received, problem with the remote? Will try again.");
                                }
                            }
                        }

                        // Add no new agents, that should only happen when we exhaust the list.
                        Ok(Vec::with_capacity(0))
                    }
                }
            }
        )?;

    ctx.get_mut()
        .scenario_values
        .initiate_with_peers
        .extend(new_peers);

    Ok(())
}

fn agent_behaviour_participate(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<ScenarioValues>>,
) -> HookResult {
    let client = ctx.get().trycp_client();

    let app_port = ctx.get().app_port();
    let cell_id = ctx.get().cell_id();
    let reporter = ctx.runner_context().reporter();

    let agent_name = ctx.agent_name().to_string();
    let accepted = ctx.get().scenario_values.session_attempts.clone();
    let accepted_success = ctx.get().scenario_values.session_successes.clone();

    ctx.runner_context().executor().execute_in_place(
        async move {
            loop {
                log::debug!("Waiting for a countersigning session to be initiated.");
                let signal = client.recv_signal().await;

                log::debug!("Received a signal.");

                match signal {
                    Some(signal) => {
                        let signal = match rmp_serde::decode::from_slice::<Signal>(&signal.data).map_err(|e| anyhow::anyhow!("Decoding failure, appears to not be a signal: {:?}", e))? {
                            Signal::App {
                                signal,
                                ..
                            } => signal,
                            _ => {
                                log::debug!("Received a signal that is not an app signal, listening for other signals.");
                                continue;
                            }
                        };

                        let request = match signal.clone().into_inner().decode::<Signals>() {
                            Ok(Signals::AcceptedRequest(request)) => request,
                            Ok(_) => {
                                log::debug!("Received a signal that is not an accepted request, listening for other signals.");
                                continue;
                            }
                            Err(e) => {
                                // Must be resilient to unexpected signals, somebody else might try to initiate with us while we're already
                                // working with another peer.
                                log::debug!("Got an unexpected signal, will try again. {:?}: {:?}", signal, e);
                                continue;
                            }
                        };

                        let start = Instant::now();
                        let accepted = accepted.fetch_add(1, std::sync::atomic::Ordering::Acquire);
                        reporter.add_custom(
                            ReportMetric::new("countersigning_session_accepted")
                                .with_tag("agent", agent_name.clone())
                                .with_field("value", accepted as i64),
                        );

                        log::debug!("Another party has initiated a countersigning session.");

                        let response = client.call_zome(
                            app_port,
                            cell_id.clone(),
                            "countersigning",
                            "accept_two_party",
                            request.preflight_request,
                            None,
                        ).await?;

                        log::debug!("Accepted the incoming session, proceeding to commit.");

                        let my_accept_response: PreflightResponse = response.decode().map_err(|e| anyhow::anyhow!("Decoding failure: {:?}", e))?;

                        client.call_zome(
                            app_port,
                            cell_id.clone(),
                            "countersigning",
                            "commit_two_party",
                            vec![request.preflight_response, my_accept_response],
                            None,
                        ).await.context("Participant failed to commit countersigned entry")?;
                        let elapsed = start.elapsed();

                        log::debug!("Completed the countersigning session with the initiating party.");

                        let accepted_success = accepted_success.fetch_add(1, std::sync::atomic::Ordering::Acquire);
                        reporter.add_custom(
                            ReportMetric::new("countersigning_session_accepted_success")
                                .with_tag("agent", agent_name.clone())
                                .with_field("value", accepted_success as i64),
                        );
                        reporter.add_custom(
                            ReportMetric::new("countersigning_session_accepted_duration")
                                .with_tag("agent", agent_name)
                                .with_field("value", elapsed.as_secs_f64()),
                        );

                        break;
                    }
                    None => {
                        log::warn!("No signal received, problem with the remote? Will try again.");
                    }
                }
            }

            Ok(())
        }
    )?;

    Ok(())
}

fn agent_teardown(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<ScenarioValues>>,
) -> HookResult {
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
    .use_named_agent_behaviour("initiate", agent_behaviour_initiate)
    .use_named_agent_behaviour("participate", agent_behaviour_participate)
    .use_agent_teardown(agent_teardown);

    let agents_at_completion = run(builder)?;

    println!("Finished with {} agents", agents_at_completion);

    Ok(())
}
