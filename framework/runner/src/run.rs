use std::sync::Arc;

use anyhow::Context;

use crate::{
    context::{AgentContext, RunnerContext, UserValuesConstraint},
    definition::ScenarioDefinitionBuilder,
    executor::Executor,
    shutdown::{start_shutdown_listener, ShutdownSignalError},
};

pub fn run<RV: UserValuesConstraint, V: UserValuesConstraint>(
    definition: ScenarioDefinitionBuilder<RV, V>,
) -> anyhow::Result<()> {
    let definition = definition.build();

    println!("Running scenario: {}", definition.name);

    let runtime = tokio::runtime::Runtime::new().context("Failed to create Tokio runtime")?;
    let shutdown_handle = start_shutdown_listener(&runtime)?;
    let executor = Arc::new(Executor::new(runtime, shutdown_handle.clone()));
    let mut runner_context = RunnerContext::new(executor, shutdown_handle.clone());

    if let Some(setup_fn) = definition.setup_fn {
        setup_fn(&mut runner_context)?;
    }

    // After the setup has run, if this is a time bounded scenario we need to set a timer to shutdown the test
    if let Some(duration) = definition.duration {
        let shutdown_handle = shutdown_handle.clone();
        runner_context.executor().spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(duration)).await;
            shutdown_handle.shutdown();
        });
    }

    let runner_context = Arc::new(runner_context);
    let runner_context_for_teardown = runner_context.clone();

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

        handles.push(
            std::thread::Builder::new()
                .name(format!("agent-{}", agent_index))
                .spawn(move || {
                    let mut context =
                        AgentContext::new(runner_context, delegated_shutdown_listener);
                    if let Some(setup_agent_fn) = setup_agent_fn {
                        setup_agent_fn(&mut context).unwrap();
                    }

                    if let Some(behaviour) = agent_behaviour_fn.get("default") {
                        loop {
                            if cycle_shutdown_receiver.should_shutdown() {
                                println!("Stopping agent {}", agent_index);
                                break;
                            }

                            println!("Running agent behaviour");
                            match behaviour(&mut context) {
                                Ok(()) => {}
                                Err(e) if e.is::<ShutdownSignalError>() => {
                                    // Do nothing, this is expected if the agent is being shutdown
                                }
                                Err(e) => {
                                    log::error!("Agent behaviour failed: {:?}", e);
                                    break;
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
        handle.join().expect("Error joining thread for test agent");
    }

    if let Some(teardown_fn) = definition.teardown_fn {
        teardown_fn(runner_context_for_teardown)?;
    }

    Ok(())
}
