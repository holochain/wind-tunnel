use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

use crate::cli::ReporterOpt;
use crate::monitor::start_monitor;
use crate::progress::start_progress;
use crate::{
    context::{AgentContext, RunnerContext, UserValuesConstraint},
    definition::ScenarioDefinitionBuilder,
    executor::Executor,
    shutdown::start_shutdown_listener,
};
use anyhow::Context;
use log::debug;
use wind_tunnel_core::prelude::{AgentBailError, ShutdownHandle, ShutdownSignalError};
use wind_tunnel_instruments::ReportConfig;
use wind_tunnel_summary_model::{RunSummaryInitArgs, append_run_summary};

/// Environment variable name to set a custom run summary file path
const RUN_SUMMARY_PATH_ENV: &str = "RUN_SUMMARY_PATH";
/// Default path for the run summary file
const DEFAULT_RUN_SUMMARY_PATH: &str = "run_summary.jsonl";

pub fn run<RV: UserValuesConstraint, V: UserValuesConstraint>(
    definition: ScenarioDefinitionBuilder<RV, V>,
) -> anyhow::Result<usize> {
    let definition = definition.build()?;

    println!("#RunId: [{}]", definition.run_id);

    // Create the summary for the run. This is used to link the run with the report and build a
    // summary from the metrics after the run has completed.
    let mut summary = wind_tunnel_summary_model::RunSummary::new(RunSummaryInitArgs {
        run_id: definition.run_id.clone(),
        scenario_name: definition.name.clone(),
        started_at: chrono::Utc::now().timestamp(),
        peer_count: definition
            .assigned_behaviours
            .iter()
            .map(|b| b.agent_count)
            .sum(),
        wind_tunnel_version: option_env!("CARGO_PKG_VERSION")
            .unwrap_or("unknown")
            .to_string(),
    })
    .with_run_duration(definition.duration_s)
    .with_assigned_behaviours(
        definition
            .assigned_behaviours
            .iter()
            .map(|b| (b.behaviour_name.clone(), b.agent_count))
            .collect(),
    );

    for capture in &definition.capture_env {
        if let Ok(value) = std::env::var(capture) {
            summary.add_env(capture.clone(), value);
        }
    }

    log::info!("Running scenario: {}", definition.name);

    let runtime = tokio::runtime::Runtime::new().context("Failed to create Tokio runtime")?;

    let shutdown_handle = start_shutdown_listener(&runtime)?;
    let report_shutdown_handle = ShutdownHandle::default();

    let reporter = {
        let _h = runtime.handle().enter();
        let mut report_config =
            ReportConfig::new(definition.run_id.clone(), definition.name.clone());

        match definition.reporter {
            ReporterOpt::InMemory => {
                report_config = report_config.enable_in_memory();
            }
            ReporterOpt::InMemoryWithCustomMetrics => {
                report_config = report_config.enable_in_memory_with_custom_metrics();
            }
            ReporterOpt::InfluxFile => {
                let dir = PathBuf::from(
                    std::env::var("WT_METRICS_DIR")
                        .context("Missing environment variable WT_METRICS_DIR".to_string())?,
                );
                report_config = report_config.enable_influx_file(dir);
            }
            ReporterOpt::Noop => {
                log::info!("No reporter enabled");
            }
        }

        Arc::new(
            report_config.init_reporter(runtime.handle(), report_shutdown_handle.new_listener())?,
        )
    };
    let executor = Arc::new(Executor::new(runtime, shutdown_handle.clone()));
    let mut runner_context = RunnerContext::new(
        executor,
        reporter,
        shutdown_handle.clone(),
        definition.run_id.clone(),
        definition.connection_string.clone(),
    );

    if let Some(setup_fn) = &definition.setup_fn {
        setup_fn(&mut runner_context)?;
    }

    let runner_context = Arc::new(runner_context);
    let runner_context_for_teardown = runner_context.clone();

    // Get build info and add to summary
    if let Some(build_info_fn) = definition.build_info_fn {
        match build_info_fn(runner_context.clone()) {
            Ok(Some(build_info)) => {
                summary.set_build_info(build_info);
            }
            Err(e) => {
                log::warn!("build_info_fn failed: {e}");
            }
            _ => {}
        }
    }

    // After the setup has run, and if this is a time bounded scenario, then we need to take additional actions
    if let Some(duration) = definition.duration_s {
        if !definition.no_progress {
            // If the scenario is time bounded then start the progress monitor to show the user how long is left
            start_progress(
                Duration::from_secs(duration),
                shutdown_handle.new_listener(),
            );
        }

        // Set a timer to shut down the test after the duration has elapsed
        let shutdown_handle = shutdown_handle.clone();
        runner_context.executor().spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(duration)).await;
            shutdown_handle.shutdown();
        });
    }

    // Ready to start spawning agents so start the resource monitor to report high usage by agents
    // which might lead to a misleading outcome.
    start_monitor(shutdown_handle.new_listener());

    let assigned_behaviours = definition.assigned_behaviours_flat();

    let agents_run_to_completion = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for (agent_index, assigned_behaviour) in assigned_behaviours.iter().enumerate() {
        // Read access to the runner context for each agent
        let runner_context = runner_context.clone();

        let setup_agent_fn = definition.setup_agent_fn;
        let agent_behaviour_fn = definition.agent_behaviour.get(assigned_behaviour).cloned();
        let teardown_agent_fn = definition.teardown_agent_fn;
        let websocket_config = definition.websocket_config.clone();

        let agents_run_to_completion = agents_run_to_completion.clone();

        // For us to check if the agent should shut down between behaviour cycles
        let mut cycle_shutdown_receiver = shutdown_handle.new_listener();
        // For the behaviour implementation to listen for shutdown and respond appropriately
        let delegated_shutdown_listener = shutdown_handle.new_listener();

        let assigned_behaviour = assigned_behaviour.clone();
        let agent_name = format!("agent-{agent_index}");

        handles.push(
            std::thread::Builder::new()
                .name(agent_name.clone())
                .spawn(move || {
                    // TODO synchronize these setups so that the scenario waits for all of them to complete before proceeding.
                    let mut context = AgentContext::new(
                        agent_index,
                        agent_name.clone(),
                        assigned_behaviour.clone(),
                        runner_context,
                        delegated_shutdown_listener,
                    );
                    // Apply websocket config if provided
                    if let Some(websocket_config) = websocket_config {
                        context = context.with_websocket_config(websocket_config);
                    }
                    if let Some(Err(e)) = setup_agent_fn.map(|f| f(&mut context)) {
                        log::error!("Agent setup failed for agent {agent_name}: {e:?}");

                        if let Some(Err(e)) = teardown_agent_fn.map(|f| f(&mut context)) {
                            log::error!("Agent teardown failed for agent {agent_name}: {e:?}");
                        }

                        return;
                    }

                    log::info!("Agent {agent_name} starting behaviour [{assigned_behaviour}]");

                    // TODO implement warmup
                    let mut behaviour_ran_to_complete = true;
                    if let Some(behaviour) = agent_behaviour_fn {
                        loop {
                            if cycle_shutdown_receiver.should_shutdown() {
                                log::debug!("Stopping agent {agent_name}");
                                break;
                            }

                            match behaviour(&mut context) {
                                Ok(()) => {}
                                Err(e) if e.is::<ShutdownSignalError>() => {
                                    // Do nothing, this is expected if the agent is being shutdown.
                                    // The check at the top of the loop will catch this and break out.
                                }
                                Err(e) if e.is::<AgentBailError>() => {
                                    // A single agent has failed, we don't want to stop the whole
                                    // scenario so warn and exit the loop.
                                    log::warn!("Agent {agent_name} bailed: {e:?}");
                                    behaviour_ran_to_complete = false;
                                    break;
                                }
                                Err(e) => {
                                    log::error!(
                                        "Agent behaviour [{assigned_behaviour}] failed for agent {agent_name}: {e:?}"
                                    );
                                }
                            }
                        }
                    }

                    if let Some(Err(e)) = teardown_agent_fn.map(|f| f(&mut context)) {
                        log::error!("Agent teardown failed for agent {agent_name}: {e:?}");
                    }

                    if behaviour_ran_to_complete {
                        agents_run_to_completion.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .expect("Failed to spawn thread for test agent"),
        );
    }

    for (index, handle) in handles.into_iter().enumerate() {
        if let Err(e) = handle.join() {
            log::error!("Could not join thread for test agent {index}: {e:?}")
        }
    }

    if let Some(teardown_fn) = definition.teardown_fn {
        // Don't crash the runner if the teardown fails. We still want the reporting and runner
        // shutdown to happen cleanly. The hook is documented as 'best effort'
        if let Err(e) = teardown_fn(runner_context_for_teardown.clone()) {
            log::error!("Teardown failed: {e:?}");
        }
    }

    // Manually shutdown the reporting once all the teardown steps are complete, this doesn't
    // respond to Ctrl+C like the user-provided code does.
    report_shutdown_handle.shutdown();
    // Then wait for the reporting to finish
    runner_context_for_teardown.reporter().finalize();

    summary.set_peer_end_count(agents_run_to_completion.load(std::sync::atomic::Ordering::Acquire));

    // append run summary
    let summary_path = std::env::var(RUN_SUMMARY_PATH_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_RUN_SUMMARY_PATH));
    debug!("Appending run summary to {}", summary_path.display());
    if let Err(e) = append_run_summary(summary, summary_path) {
        log::error!("Failed to append run summary: {e:?}");
    }

    println!("#RunId: [{}]", definition.run_id);

    Ok(agents_run_to_completion.load(std::sync::atomic::Ordering::Acquire))
}
