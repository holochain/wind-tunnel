use holochain_types::app::{AppBundleSource, InstallAppPayload};
use holochain_types::prelude::AppBundle;
use trycp_wind_tunnel_runner::prelude::*;

const CONDUCTOR_CONFIG: &str = include_str!("../../../conductor-config.yaml");

fn agent_setup(ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>) -> HookResult {
    connect_trycp_client(ctx)?;
    reset_trycp_remote(ctx)?;

    let run_id = ctx.runner_context().get_run_id().to_string();
    let client = ctx.get().trycp_client();
    let agent_id = ctx.agent_id().to_string();

    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            client
                .configure_player("test".to_string(), CONDUCTOR_CONFIG.to_string(), None)
                .await?;

            client.startup("test".to_string(), Some("info".to_string()), None).await?;

            println!("Started agent");

            let agent_key = client.generate_agent_pub_key("test".to_string(), None).await?;

            let path = scenario_happ_path!("remote_call");
            let content = tokio::fs::read(path).await?;

            let app_info = client.install_app("test".to_string(), InstallAppPayload {
                source: AppBundleSource::Bundle(AppBundle::decode(&content)?),
                agent_key,
                installed_app_id: Some("remote_call".into()),
                membrane_proofs: Default::default(),
                network_seed: Some(run_id),
            }, None).await?;

            let enable_result = client.enable_app("test".to_string(), app_info.installed_app_id.clone(), None).await?;
            if !enable_result.errors.is_empty() {
                return Err(anyhow::anyhow!("Failed to enable app: {:?}", enable_result.errors));
            }

            println!("Installed app: {:?}", app_info);

            for _ in 0..10 {
                let dump = client.dump_network_metrics("test".to_string(), None, None).await?;
                println!("agent: {}, dump: {:?}", agent_id, dump);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }

            Ok(())
        })?;

    Ok(())
}

fn agent_behaviour(ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>) -> HookResult {
    let client = ctx.get().trycp_client();

    // ctx.runner_context()
    //     .executor()
    //     .execute_in_place(async move {
    //         client
    //             .configure_player("test".to_string(), "".to_string(), None)
    //             .await?;
    //
    //         client.reset(None).await?;
    //
    //         Ok(())
    //     })?;

    Ok(())
}

fn agent_teardown(ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>) -> HookResult {
    disconnect_trycp_client(ctx)?;
    Ok(())
}

fn main() -> WindTunnelResult<()> {
    let builder =
        TryCPScenarioDefinitionBuilder::<TryCPRunnerContext, TryCPAgentContext>::new_with_init(
            env!("CARGO_PKG_NAME"),
        )?
            .into_std()
            .use_agent_setup(agent_setup)
            .use_agent_behaviour(agent_behaviour)
            .use_agent_teardown(agent_teardown);

    run(builder)?;

    Ok(())
}
