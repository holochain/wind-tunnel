use std::sync::Arc;

use crate::{
    context::{Context, RunnerContext, UserValuesConstraint},
    definition::ScenarioDefinitionBuilder,
    executor::Executor, shutdown::start_shutdown_listener,
};

pub fn run<RV: UserValuesConstraint, V: UserValuesConstraint>(
    definition: ScenarioDefinitionBuilder<RV, V>,
) -> anyhow::Result<()> {
    let definition = definition.build();

    println!("Running scenario: {}", definition.name);

    let mut runner_context = RunnerContext::new(Executor::default());
    let shutdown_listener = start_shutdown_listener(runner_context.executor())?;

    if let Some(setup_fn) = definition.setup_fn {
        setup_fn(&mut runner_context)?;
    }

    let runner_context = Arc::new(runner_context);
    let runner_context_for_teardown = runner_context.clone();

    let mut handles = Vec::new();
    for agent_index in 0..definition.cli.agents {
        // Read access to the runner context for each agent
        let runner_context = runner_context.clone();

        let setup_agent_fn = definition.setup_agent_fn;
        let agent_behaviour_fn = definition.agent_behaviour.clone();
        let teardown_agent_fn = definition.teardown_agent_fn.clone();

        // For us to check if the agent should shutdown between behaviour cycles
        let mut cycle_shutdown_receiver = shutdown_listener.new_listener();
        // For the behaviour implementation to listen for shutdown and respond appropriately
        let delegated_shutdown_listener = shutdown_listener.new_listener();

        handles.push(std::thread::spawn(move || {
            let mut context = Context::new(runner_context, delegated_shutdown_listener);
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
                    behaviour(&mut context).unwrap();
                }
            }

            if let Some(teardown_agent_fn) = teardown_agent_fn {
                teardown_agent_fn(&mut context).unwrap();
            }
        }));
    }

    for handle in handles {
        handle.join().expect("Error joining thread for test agent");
    }

    if let Some(teardown_fn) = definition.teardown_fn {
        teardown_fn(runner_context_for_teardown)?;
    }

    Ok(())
}
