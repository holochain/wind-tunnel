use std::time::Duration;
use trycp_wind_tunnel_runner::embed_conductor_config;
use trycp_wind_tunnel_runner::prelude::*;

embed_conductor_config!();

mod remote_call_rate {
    include!("../../remote_call_rate/src/common.rs");
}

mod remote_signals {
    include!("../../remote_signals/src/common.rs");
}

mod trycp_write_validated {
    include!("../../trycp_write_validated/src/common.rs");
}

#[derive(Debug, Default)]
pub struct ScenarioValues {
    remote_call_rate: remote_call_rate::ScenarioValues,
    remote_signals: remote_signals::ScenarioValues,
    trycp_write_validated: trycp_write_validated::ScenarioValues,
}

impl UserValuesConstraint for ScenarioValues {}

impl AsMut<remote_call_rate::ScenarioValues> for ScenarioValues {
    fn as_mut(&mut self) -> &mut remote_call_rate::ScenarioValues {
        &mut self.remote_call_rate
    }
}

impl AsMut<remote_signals::ScenarioValues> for ScenarioValues {
    fn as_mut(&mut self) -> &mut remote_signals::ScenarioValues {
        &mut self.remote_signals
    }
}

impl AsMut<trycp_write_validated::ScenarioValues> for ScenarioValues {
    fn as_mut(&mut self) -> &mut trycp_write_validated::ScenarioValues {
        &mut self.trycp_write_validated
    }
}

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
                .configure_player(agent_name.clone(), conductor_config().to_string(), None)
                .await?;

            client.startup(agent_name.clone(), None).await?;

            Ok(())
        })?;

    remote_call_rate::agent_setup_post_startup_pre_install_hook(ctx)?;
    remote_signals::agent_setup_post_startup_pre_install_hook(ctx)?;
    trycp_write_validated::agent_setup_post_startup_pre_install_hook(ctx)?;

    install_app(ctx, scenario_happ_path!("frank1"), &"frank1".to_string())?;
    try_wait_for_min_peers(ctx, Duration::from_secs(120))?;

    Ok(())
}

fn agent_behaviour(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<ScenarioValues>>,
) -> HookResult {
    remote_call_rate::agent_behaviour_hook(ctx)?;
    remote_signals::agent_behaviour_hook(ctx)?;
    trycp_write_validated::agent_behaviour_hook(ctx)?;

    // Don't just hammer
    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            Ok(())
        })?;

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