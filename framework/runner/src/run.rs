use std::sync::Arc;

use anyhow::Context;
use wind_tunnel_instruments::ReportConfig;

use crate::{
    context::{AgentContext, RunnerContext, UserValuesConstraint},
    definition::ScenarioDefinitionBuilder,
    executor::Executor,
    shutdown::{start_shutdown_listener, ShutdownSignalError},
};
use crate::monitor::start_monitor;

pub fn run<RV: UserValuesConstraint, V: UserValuesConstraint>(
    definition: ScenarioDefinitionBuilder<RV, V>,
) -> anyhow::Result<()> {
    let definition = definition.build();

    println!("Running scenario: {}", definition.name);

    let runtime = tokio::runtime::Runtime::new().context("Failed to create Tokio runtime")?;
    let shutdown_handle = start_shutdown_listener(&runtime)?;
    let executor = Arc::new(Executor::new(runtime, shutdown_handle.clone()));
    let reporter = Arc::new(ReportConfig::default().enable_summary().init());
    let mut runner_context = RunnerContext::new(executor, reporter, shutdown_handle.clone());

    if let Some(setup_fn) = definition.setup_fn {
        setup_fn(&mut runner_context)?;
    }

    // After the setup has run, if this is a time bounded scenario we need to set a timer to shut down the test
    if let Some(duration) = definition.duration {
        let shutdown_handle = shutdown_handle.clone();
        runner_context.executor().spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(duration)).await;
            shutdown_handle.shutdown();
        });
    }

    let runner_context = Arc::new(runner_context);
    let runner_context_for_teardown = runner_context.clone();

    // Ready to start spawning agents so start the resource monitor to report high usage by agents
    // which might lead to a misleading outcome.
    start_monitor(shutdown_handle.new_listener());

    let mut handles = Vec::new();
    for agent_index in 0..definition.agent_count {
        // Read access to the runner context for each agent
        let runner_context = runner_context.clone();

        let setup_agent_fn = definition.setup_agent_fn;
        let agent_behaviour_fn = definition.agent_behaviour.clone();
        let teardown_agent_fn = definition.teardown_agent_fn;

        // For us to check if the agent should shutdown between behaviour cycles
        let mut cycle_shutdown_receiver = shutdown_handle.new_listener();
        // For the behaviour implementation to listen for shutdown and respond appropriately
        let delegated_shutdown_listener = shutdown_handle.new_listener();

        let agent_id = format!("agent-{}", agent_index);

        handles.push(
            std::thread::Builder::new()
                .name(agent_id.clone())
                .spawn(move || {
                    let mut context =
                        AgentContext::new(agent_id, runner_context, delegated_shutdown_listener);
                    if let Some(setup_agent_fn) = setup_agent_fn {
                        setup_agent_fn(&mut context).unwrap();
                    }

                    if let Some(behaviour) = agent_behaviour_fn.get("default") {
                        loop {
                            if cycle_shutdown_receiver.should_shutdown() {
                                println!("Stopping agent {}", agent_index);
                                break;
                            }

                            match behaviour(&mut context) {
                                Ok(()) => {}
                                Err(e) if e.is::<ShutdownSignalError>() => {
                                    // Do nothing, this is expected if the agent is being shutdown.
                                    // The check at the top of the loop will catch this and break out.
                                }
                                Err(e) => {
                                    log::error!("Agent behaviour failed: {:?}", e);
                                }
                            }
                        }
                    }

                    if let Some(teardown_agent_fn) = teardown_agent_fn {
                        teardown_agent_fn(&mut context).unwrap();
                    }
                })
                .expect("Failed to spawn thread for test agent"),
        );
    }

    for handle in handles {
        handle.join().map_err(|e| {
            anyhow::anyhow!("Error joining thread for test agent: {:?}", e)
        })?;
    }

    if let Some(teardown_fn) = definition.teardown_fn {
        teardown_fn(runner_context_for_teardown.clone())?;
    }

    runner_context_for_teardown.reporter().finalize();

    Ok(())
}
