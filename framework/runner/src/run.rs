use std::sync::Arc;

use crate::{
    context::{Context, RunnerContext, UserValuesConstraint},
    definition::ScenarioDefinitionBuilder,
    executor::Executor,
};

pub fn run<RV: UserValuesConstraint, V: UserValuesConstraint>(
    definition: ScenarioDefinitionBuilder<RV, V>,
) -> anyhow::Result<()> {
    let definition = definition.build();

    println!("Running scenario: {}", definition.name);

    let mut runner_context = RunnerContext::new(Executor::default());

    if let Some(setup_fn) = definition.setup_fn {
        setup_fn(&mut runner_context)?;
    }

    let runner_context = Arc::new(runner_context);

    for _ in 0..definition.cli.agents {
        let runner_context = runner_context.clone();

        let setup_agent_fn = definition.setup_agent_fn;
        let agent_behaviour_fn = definition.agent_behaviour.clone();

        std::thread::spawn(move || {
            let mut context = Context::new(runner_context);
            if let Some(setup_agent_fn) = setup_agent_fn {
                setup_agent_fn(&mut context).unwrap();
            }

            if let Some(behaviour) = agent_behaviour_fn.get("default") {
                behaviour(&mut context).unwrap();
            }
        });
    }

    let mut context = Context::new(runner_context.clone());
    if let Some(setup_agent_fn) = definition.setup_agent_fn {
        setup_agent_fn(&mut context)?;
    }

    if let Some(behaviour) = definition.agent_behaviour.get("default") {
        behaviour(&mut context)?;
    }

    Ok(())
}
