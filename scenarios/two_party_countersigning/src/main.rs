use anyhow::Context;
use countersigning_integrity::{AcceptedRequest, Signals};
use holochain_types::prelude::{AgentPubKey, CellId, EntryHash, ExternIO, PreflightResponse};
use holochain_types::signal::{Signal, SystemSignal};
use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::ops::Add;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;

#[derive(Debug, Default)]
pub struct ScenarioValues {
    signal_tx: Option<tokio::sync::broadcast::Sender<Signal>>,
    initiate_with_peers: Vec<AgentPubKey>,
    session_attempts: Arc<AtomicUsize>,
    session_successes: Arc<AtomicUsize>,
    session_failures: Arc<AtomicUsize>,
}

impl UserValuesConstraint for ScenarioValues {}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    start_conductor_and_configure_urls(ctx)?;
    install_app(
        ctx,
        scenario_happ_path!("countersigning"),
        &"countersigning".to_string(),
    )?;
    try_wait_for_min_agents(ctx, Duration::from_secs(120))?;

    let (signal_tx, _) = tokio::sync::broadcast::channel::<Signal>(100);
    let client = ctx.get().app_client();
    ctx.runner_context().executor().execute_in_place({
        let signal_tx = signal_tx.clone();
        async move {
            client
                .on_signal(move |signal| {
                    if let Err(e) = signal_tx.send(signal) {
                        log::info!("Failed to relay signal: {e:?}");
                    };
                })
                .await?;

            Ok(())
        }
    })?;
    ctx.get_mut().scenario_values.signal_tx = Some(signal_tx);

    let assigned_behaviour = ctx.assigned_behaviour().to_string();
    if assigned_behaviour == "initiate" {
        // As an initiator we just need to call a zome so that `init` will run.
        call_zome::<_, String, _>(ctx, "countersigning", "initiator_hello", ())?;
    } else if assigned_behaviour == "participate" {
        // As a participant we need to advertise our role by publishing a link to our agent key
        call_zome::<_, (), _>(ctx, "countersigning", "participant_hello", ())?;
    } else {
        return Err(anyhow::anyhow!(
            "Unknown assigned behaviour: {assigned_behaviour}"
        ));
    }

    log::debug!(
        "Agent setup complete for {}, with agent pub key {:?}",
        ctx.agent_name(),
        ctx.get().cell_id().agent_pubkey()
    );

    Ok(())
}

fn agent_behaviour_initiate(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let client = ctx.get().app_client();
    let signal_rx = ctx
        .get()
        .scenario_values
        .signal_tx
        .as_ref()
        .unwrap()
        .subscribe();

    let cell_id = ctx.get().cell_id();
    let initiate_with_peers = ctx.get_mut().scenario_values.initiate_with_peers.pop();
    let reporter = ctx.runner_context().reporter();

    let initiated = ctx.get().scenario_values.session_attempts.clone();
    let initiated_success = ctx.get().scenario_values.session_successes.clone();
    let initiated_failure = ctx.get().scenario_values.session_failures.clone();

    let new_peers = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move {
            match initiate_with_peers {
                None => {
                    // No more agents available to call, get a new list.
                    // This is also the initial condition.
                    let response = client
                        .call_zome(
                            ZomeCallTarget::CellId(cell_id),
                            "countersigning",
                            "list_participants",
                            ExternIO::encode(()).context("Failed to encode empty payload")?,
                        )
                        .await
                        .context("Failed to list participants")?;

                    let mut new_peer_list: Vec<AgentPubKey> = response
                        .decode()
                        .context("Failed to decode agent list response")?;

                    new_peer_list.shuffle(&mut thread_rng());

                    // Pause to let Holochain receive more agent links if none are found yet.
                    if new_peer_list.is_empty() {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }

                    Ok(new_peer_list)
                }
                Some(agent_pub_key) => {
                    log::debug!("Initiating a countersigning session with agent {agent_pub_key:?}");

                    let start = Instant::now();
                    let initiated = initiated.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    reporter.add_custom(
                        ReportMetric::new("countersigning_session_initiated")
                            .with_tag("agent", cell_id.agent_pubkey().to_string())
                            .with_field("value", initiated as u64),
                    );

                    // Start a countersigning session with the next agent in the list.
                    let response = client
                        .call_zome(
                            ZomeCallTarget::CellId(cell_id.clone()),
                            "countersigning",
                            "start_two_party",
                            ExternIO::encode(agent_pub_key.clone())
                                .context("Failed to encode agent pub key")?,
                        )
                        .await
                        .with_context(|| {
                            format!(
                                "Failed to start a new countersigning session: {agent_pub_key:?}"
                            )
                        })?;
                    let my_preflight_response: PreflightResponse = response
                        .decode()
                        .context("Failed to decode preflight response")?;

                    let session_times = my_preflight_response.request.session_times.clone();
                    let session_timeout = Instant::now().add(Duration::from_millis(
                        (session_times.end.as_millis() - session_times.start.as_millis()) as u64,
                    ));

                    match run_initiated_session(
                        client.clone(),
                        signal_rx,
                        my_preflight_response,
                        session_timeout,
                        agent_pub_key.clone(),
                        cell_id.clone(),
                    )
                    .await
                    {
                        Ok(retry_count) => {
                            let elapsed = start.elapsed();

                            log::debug!(
                                "Completed countersigning session with agent {agent_pub_key:?}"
                            );

                            let initiated_success = initiated_success
                                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            reporter.add_custom(
                                ReportMetric::new("countersigning_session_initiated_success")
                                    .with_tag("agent", cell_id.agent_pubkey().to_string())
                                    .with_tag("retries", retry_count as u64)
                                    .with_field("value", (initiated_success + 1) as u64),
                            );
                            reporter.add_custom(
                                ReportMetric::new("countersigning_session_initiated_duration")
                                    .with_tag("agent", cell_id.agent_pubkey().to_string())
                                    .with_tag("failed", false)
                                    .with_field("value", elapsed.as_secs_f64()),
                            );
                        }
                        Err(e) => {
                            let elapsed = start.elapsed();

                            log::warn!(
                                "Failed countersigning session with agent {agent_pub_key:?}: {e:?}"
                            );

                            let initiated_failure = initiated_failure
                                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            reporter.add_custom(
                                ReportMetric::new("countersigning_session_initiated_failure")
                                    .with_tag("agent", cell_id.agent_pubkey().to_string())
                                    .with_field("value", (initiated_failure + 1) as u64),
                            );
                            reporter.add_custom(
                                ReportMetric::new("countersigning_session_initiated_duration")
                                    .with_tag("agent", cell_id.agent_pubkey().to_string())
                                    .with_tag("failed", true)
                                    .with_field("value", elapsed.as_secs_f64()),
                            );
                        }
                    }

                    // Add no new agents, that should only happen when we exhaust the list.
                    Ok(Vec::with_capacity(0))
                }
            }
        })?;

    ctx.get_mut()
        .scenario_values
        .initiate_with_peers
        .extend(new_peers);

    Ok(())
}

async fn run_initiated_session(
    client: AppWebsocket,
    mut signal_rx: tokio::sync::broadcast::Receiver<Signal>,
    my_preflight_response: PreflightResponse,
    session_timeout: Instant,
    agent_pub_key: AgentPubKey,
    cell_id: CellId,
) -> anyhow::Result<usize> {
    loop {
        // Now listen for a signal from the remote with their acceptance
        let signal = tokio::time::timeout_at(session_timeout, signal_rx.recv()).await
            .with_context(|| format!("Agent [{agent_pub_key:?}] did not respond to the countersigning request in time, abandoning"))?
            .context("Failed to receive signal")?;

        let signal = match signal {
            Signal::App { signal, .. } => signal,
            _ => {
                log::debug!(
                    "Received a signal that is not an app signal, listening for other signals."
                );
                continue;
            }
        };

        let other_response = match signal.clone().into_inner().decode::<Signals>() {
            Ok(Signals::Response(response))
                if response.request().fingerprint()?
                    == my_preflight_response.request().fingerprint()? =>
            {
                response
            }
            Ok(_) => {
                log::debug!("Received a signal that is not a response for this countersigning session, listening for other signals.");
                continue;
            }
            Err(_) => {
                // We shouldn't really be getting signals that don't decode but choosing to
                // filter them out here.
                log::debug!("Got an unexpected signal, will try again. {signal:?}");
                continue;
            }
        };

        log::debug!("The other party [{agent_pub_key:?}] has accepted the countersigning session.");

        return complete_session(
            client.clone(),
            signal_rx,
            cell_id.clone(),
            my_preflight_response,
            other_response,
            session_timeout,
        )
        .await;
    }
}

fn agent_behaviour_participate(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let client = ctx.get().app_client();
    let mut signal_rx = ctx
        .get()
        .scenario_values
        .signal_tx
        .as_ref()
        .unwrap()
        .subscribe();

    let cell_id = ctx.get().cell_id();
    let reporter = ctx.runner_context().reporter();

    let accepted = ctx.get().scenario_values.session_attempts.clone();
    let accepted_success = ctx.get().scenario_values.session_successes.clone();
    let accepted_failure = ctx.get().scenario_values.session_failures.clone();

    ctx.runner_context().executor().execute_in_place(
        async move {
            loop {
                log::debug!("Waiting for a countersigning session to be initiated.");
                let signal = signal_rx.recv().await.context("Failed to receive signal")?;

                log::debug!("Received a signal.");

                let signal = match signal {
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
                        log::debug!("Got an unexpected signal, will try again. {signal:?}: {e:?}");
                        continue;
                    }
                };

                let start = Instant::now();
                let accepted = accepted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                reporter.add_custom(
                    ReportMetric::new("countersigning_session_accepted")
                        .with_tag("agent", cell_id.agent_pubkey().to_string())
                        .with_field("value", accepted as u64),
                );

                // Figure out the session end time, so we can stop waiting for the session to complete when
                // retrying or listening for signals.
                let session_times = request.preflight_request.session_times.clone();
                let session_timeout = Instant::now().add(Duration::from_millis(
                    (session_times.end.as_millis() - session_times.start.as_millis()) as u64,
                ));

                match run_accepted_session(client, signal_rx, request, session_timeout, cell_id.clone()).await {
                    Ok(retry_count) => {
                        let elapsed = start.elapsed();

                        log::debug!("Completed countersigning session with the initiating party.");

                        let accepted_success = accepted_success.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        reporter.add_custom(
                            ReportMetric::new("countersigning_session_accepted_success")
                                .with_tag("agent", cell_id.agent_pubkey().to_string())
                                .with_tag("retries", retry_count as u64)
                                .with_field("value", (accepted_success + 1) as u64),
                        );
                        reporter.add_custom(
                            ReportMetric::new("countersigning_session_accepted_duration")
                                .with_tag("agent", cell_id.agent_pubkey().to_string())
                                .with_tag("failed", false)
                                .with_field("value", elapsed.as_secs_f64()),
                        );
                    },
                    Err(e) => {
                        let elapsed = start.elapsed();

                        log::warn!("Failed countersigning session with the initiating party: {e:?}");

                        // If we got a fatal error rather than a successful session, wait for the session to expire before trying again
                        tokio::time::sleep_until(session_timeout).await;

                        let accepted_failure = accepted_failure.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        reporter.add_custom(
                            ReportMetric::new("countersigning_session_accepted_failure")
                                .with_tag("agent", cell_id.agent_pubkey().to_string())
                                .with_field("value", (accepted_failure + 1) as u64),
                        );
                        reporter.add_custom(
                            ReportMetric::new("countersigning_session_accepted_duration")
                                .with_tag("agent", cell_id.agent_pubkey().to_string())
                                .with_tag("failed", true)
                                .with_field("value", elapsed.as_secs_f64()),
                        );
                    }
                };

                break;
            }

            Ok(())
        }
    )?;

    Ok(())
}

async fn run_accepted_session(
    client: AppWebsocket,
    signal_rx: tokio::sync::broadcast::Receiver<Signal>,
    request: AcceptedRequest,
    session_timeout: Instant,
    cell_id: CellId,
) -> anyhow::Result<usize> {
    log::debug!("Another party has initiated a countersigning session.");

    let response = client
        .call_zome(
            ZomeCallTarget::CellId(cell_id.clone()),
            "countersigning",
            "accept_two_party",
            ExternIO::encode(request.preflight_request)
                .context("Failed to encode preflight request")?,
        )
        .await?;

    log::debug!("Accepted the incoming session, proceeding to commit.");

    let my_accept_response: PreflightResponse = response
        .decode()
        .map_err(|e| anyhow::anyhow!("Decoding failure: {e:?}"))?;

    complete_session(
        client.clone(),
        signal_rx,
        cell_id.clone(),
        request.preflight_response,
        my_accept_response,
        session_timeout,
    )
    .await
}

async fn complete_session(
    client: AppWebsocket,
    signal_rx: tokio::sync::broadcast::Receiver<Signal>,
    cell_id: CellId,
    initiate_preflight_response: PreflightResponse,
    participate_preflight_response: PreflightResponse,
    session_timeout: Instant,
) -> anyhow::Result<usize> {
    let mut retry_count = 0;
    for i in 0.. {
        let r = client
            .call_zome(
                ZomeCallTarget::CellId(cell_id.clone()),
                "countersigning",
                "commit_two_party",
                ExternIO::encode(vec![
                    initiate_preflight_response.clone(),
                    participate_preflight_response.clone(),
                ])
                .context("Failed to encode preflight responses")?,
            )
            .await
            .context("Failed to commit countersigned entry");

        match r {
            Ok(_) => {
                break;
            }
            Err(e) => {
                if Instant::now() > session_timeout {
                    // We haven't been able to commit our entry by the end of the countersigning
                    // session time, so we should abandon the attempt. This is safe because we
                    // haven't published a signature.
                    return Err(e).context(format!(
                        "Abandoning commit attempt because the session timed out on attempt {i}"
                    ));
                } else if e
                    .chain()
                    .any(|e| e.to_string().contains("DepMissingFromDht"))
                {
                    // Skip logging this message, it's what we're expecting to take some time in this retry loop
                } else if e.chain().any(|e| {
                    e.to_string()
                        .contains("countersigning session that has already expired")
                }) {
                    return Err(e).context(format!("Failed because the session expired on attempt {} and with {:?} expected time remaining", i, session_timeout - Instant::now()));
                } else {
                    log::warn!("[{i}] Failed to commit countersigned entry, will retry. {e:?}");
                }

                retry_count = i;
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }
    }

    // Wait for the session to complete before recording the time taken and the successful result.
    // This also prevents a new session starting while our chain is locked!
    match await_countersigning_success(
        signal_rx,
        initiate_preflight_response.request.app_entry_hash,
    )
    .await
    {
        Ok(_) => {}
        Err(e) => {
            return Err(e).with_context(|| {
                format!(
                    "Session between [{:?}] did not complete within the session time",
                    participate_preflight_response.request.signing_agents
                )
            });
        }
    }

    log::debug!("Completed countersigning session with retry count: {retry_count}");

    Ok(retry_count)
}

async fn await_countersigning_success(
    mut signal_rx: tokio::sync::broadcast::Receiver<Signal>,
    session_entry_hash: EntryHash,
) -> HookResult {
    loop {
        // Note that we don't expect the session timeout here. We will wait for Holochain to
        // make a decision and not assume that the session is resolved at the end time.
        let signal = signal_rx.recv().await.context("Failed to receive signal")?;
        match signal {
            Signal::System(SystemSignal::SuccessfulCountersigning(eh))
                if eh == session_entry_hash =>
            {
                log::debug!("Countersigning session completed successfully.");
                break;
            }
            Signal::System(SystemSignal::SuccessfulCountersigning(_)) => {
                // This shouldn't happen because only one countersigning session can be active at a time. There's a bug if this log message shows up.
                log::error!("Received a successful countersigning signal for a different session, listening for other signals.");
                continue;
            }
            Signal::System(SystemSignal::AbandonedCountersigning(eh))
                if eh == session_entry_hash =>
            {
                return Err(anyhow::anyhow!("Countersigning session was abandoned"));
            }
            Signal::System(SystemSignal::AbandonedCountersigning(_)) => {
                // This shouldn't happen because only one countersigning session can be active at a time. There's a bug if this log message shows up.
                log::error!("Received an abandoned countersigning signal for a different session, listening for other signals.");
                continue;
            }
            // Note that this might include other initiations. Since we will ignore the signal here, the initiator will have to wait for the timeout.
            signal => {
                log::debug!("Received a signal that is not a successful countersigning signal, listening for other signals. {signal:?}");
                continue;
            }
        };
    }

    Ok(())
}

fn main() -> WindTunnelResult<()> {
    let builder = ScenarioDefinitionBuilder::<
        HolochainRunnerContext,
        HolochainAgentContext<ScenarioValues>,
    >::new_with_init(env!("CARGO_PKG_NAME"))
    .use_build_info(conductor_build_info)
    .use_agent_setup(agent_setup)
    .use_named_agent_behaviour("initiate", agent_behaviour_initiate)
    .use_named_agent_behaviour("participate", agent_behaviour_participate)
    .use_agent_teardown(|ctx| {
        uninstall_app(ctx, None).ok();
        Ok(())
    });

    run(builder)?;

    Ok(())
}
