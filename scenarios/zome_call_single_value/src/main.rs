use anyhow::Context;
use holochain_client_instrumented::prelude::{
    AdminWebsocket, AppAgentWebsocket, AuthorizeSigningCredentialsPayload,
    ClientAgentSigner,
};
use holochain_conductor_api::{AppStatusFilter, CellInfo};
use holochain_types::prelude::{
    AppBundleSource, ExternIO, InstallAppPayload,
};
use holochain_wind_tunnel_runner::prelude::*;
use std::path::Path;
use std::{sync::Arc};

fn setup(ctx: &mut RunnerContext<HolochainRunnerContext>) -> HookResult {
    configure_app_ws_url(ctx)?;
    Ok(())
}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>,
) -> HookResult {
    let admin_ws_url = ctx.runner_context().get_connection_string().to_string();

    // TODO break this function down a bit?
    let app_ws_url = ctx.runner_context().get().app_ws_url();
    let agent_id = ctx.agent_id().to_string();
    let reporter = ctx.runner_context().reporter();
    let (installed_app_id, app_agent_client) = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move {
            log::info!("Connecting a Holochain admin client: {}", admin_ws_url);
            let mut client =
                AdminWebsocket::connect(admin_ws_url, reporter.clone()).await?;

            // TODO kills the test if it fails, that is not intentional. The error should be reported but not unwrapped
            let key = client
                .generate_agent_pub_key()
                .await
                .map_err(|e| anyhow::anyhow!("Conductor API error: {:?}", e))?;

            log::info!("Generated agent pub key: {:}", key);

            let installed_app_id = format!("{}-app", agent_id).to_string();
            client
                .install_app(InstallAppPayload {
                    source: AppBundleSource::Path(
                        Path::new(env!("CARGO_MANIFEST_DIR"))
                            .join("../../happs")
                            .join(env!("CARGO_PKG_NAME"))
                            .join("return_single_value.happ"),
                    ),
                    agent_key: key,
                    installed_app_id: Some(installed_app_id.clone()),
                    membrane_proofs: Default::default(),
                    network_seed: None,
                })
                .await
                .map_err(|e| anyhow::anyhow!("Conductor API error: {:?}", e))?;

            client
                .enable_app(installed_app_id.clone())
                .await
                .map_err(|e| anyhow::anyhow!("Conductor API error: {:?}", e))?;

            let app_info = client
                .list_apps(Some(AppStatusFilter::Running))
                .await
                .map_err(|e| anyhow::anyhow!("Conductor API error: {:?}", e))?;
            let app_info = app_info
                .iter()
                .find(|app| app.installed_app_id == installed_app_id)
                .context("Cannot find the app which was just installed")?;
            let cell_id = match app_info
                .cell_info
                .get("return_single_value")
                .unwrap()
                .first()
                .unwrap()
            {
                CellInfo::Provisioned(c) => c.cell_id.clone(),
                _ => anyhow::bail!("Cell not provisioned"),
            };

            let credentials = client
                .authorize_signing_credentials(AuthorizeSigningCredentialsPayload {
                    cell_id: cell_id.clone(),
                    functions: None,
                })
                .await
                .map_err(|e| anyhow::anyhow!("Conductor API error: {:?}", e))?;

            let mut signer = ClientAgentSigner::default();
            signer.add_credentials(cell_id, credentials);

            let app_agent_client = AppAgentWebsocket::connect(
                app_ws_url,
                installed_app_id.clone(),
                signer.into(),
                reporter,
            )
            .await?;

            Ok((installed_app_id, app_agent_client))
        })
        .context("Failed to install app")?;

    ctx.get_mut().app_agent_client = Some(app_agent_client);
    ctx.get_mut().installed_app_id = Some(installed_app_id);

    Ok(())
}

fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>,
) -> HookResult {
    let mut app_agent_client = ctx.get().app_agent_client.clone().unwrap();
    let installed_app_id = ctx.get().installed_app_id.clone().unwrap();
    ctx.runner_context().executor().execute_in_place(async {
        let app_info = app_agent_client
            .app_info(installed_app_id)
            .await
            .map_err(|e| anyhow::anyhow!("Conductor API error: {:?}", e))?
            .context("AppInfo not found")?;
        let cell_id = match app_info
            .cell_info
            .get("return_single_value")
            .unwrap()
            .first()
            .unwrap()
        {
            CellInfo::Provisioned(c) => c.cell_id.clone(),
            _ => anyhow::bail!("Cell not provisioned"),
        };

        app_agent_client
            .call_zome(
                cell_id.into(),
                "return_single_value".into(),
                "get_value".into(),
                ExternIO::encode(()).context("Encoding failure")?,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Conductor API error: {:?}", e))?;

        Ok(())
    })?;

    Ok(())
}

fn agent_teardown(
    _ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>,
) -> HookResult {
    println!("Shutdown hook");

    Ok(())
}

fn teardown(_ctx: Arc<RunnerContext<HolochainRunnerContext>>) -> HookResult {
    println!("Tearing down the scenario");

    Ok(())
}

fn main() -> WindTunnelResult<()> {
    let builder = ScenarioDefinitionBuilder::<HolochainRunnerContext, HolochainAgentContext>::new(
        env!("CARGO_PKG_NAME"),
    )
    .with_default_duration_s(60)
    .use_setup(setup)
    .use_agent_setup(agent_setup)
    .use_agent_behaviour(agent_behaviour)
    .use_agent_teardown(agent_teardown)
    .use_teardown(teardown);

    run(builder)?;

    Ok(())
}
