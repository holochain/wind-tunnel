use crate::common::{get_cell_id_for_role_name, installed_app_id_for_agent};
use crate::context::HolochainAgentContext;
use crate::runner_context::HolochainRunnerContext;
use anyhow::Context;
use holochain_client_instrumented::prelude::{
    handle_api_err, AdminWebsocket, AppWebsocket, AuthorizeSigningCredentialsPayload,
    ClientAgentSigner,
};
use holochain_types::prelude::*;
use holochain_types::prelude::{AppBundleSource, InstallAppPayload, RoleName};
use std::collections::HashMap;
use std::path::PathBuf;
use wind_tunnel_runner::prelude::{AgentContext, UserValuesConstraint, WindTunnelResult};

pub async fn build_happ(
    path: PathBuf,
    properties: HashMap<String, YamlProperties>,
) -> WindTunnelResult<AppBundleSource> {
    let mut source = AppBundleSource::Path(path);
    use mr_bundle::Bundle;
    let bundle: mr_bundle::Bundle<AppManifest> = match source {
        AppBundleSource::Bytes(bundle) => Bundle::decode(&bundle).unwrap(),
        AppBundleSource::Path(path) => Bundle::read_from_file(&path).await.unwrap(),
    };
    let AppManifest::V1(mut manifest) = bundle.manifest().clone();
    for role_manifest in &mut manifest.roles {
        let properties = properties.get(&role_manifest.name);
        role_manifest.dna.modifiers.properties = properties.cloned();
    }
    source = AppBundleSource::Bytes(
        bundle
            .update_manifest(AppManifest::V1(manifest))
            .unwrap()
            .encode()
            .unwrap(),
    );
    Ok(source)
}

pub async fn build_happ_from_bytes(
    bytes: &[u8],
    properties: HashMap<String, YamlProperties>,
) -> WindTunnelResult<AppBundleSource> {
    use mr_bundle::Bundle;
    let bundle: mr_bundle::Bundle<AppManifest> = Bundle::decode(bytes)?;
    let AppManifest::V1(mut manifest) = bundle.manifest().clone();
    for role_manifest in &mut manifest.roles {
        let properties = properties.get(&role_manifest.name);
        role_manifest.dna.modifiers.properties = properties.cloned();
    }
    let source = AppBundleSource::Bytes(
        bundle
            .update_manifest(AppManifest::V1(manifest))?
            .encode()?,
    );
    Ok(source)
}

pub fn custom_install_app<SV>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    app_path: PathBuf,
    role_name: &RoleName,
    agent_pubkey: Option<AgentPubKey>,
    dna_properties: Option<HashMap<String, YamlProperties>>,
) -> WindTunnelResult<()>
where
    SV: UserValuesConstraint,
{
    let admin_ws_url = ctx
        .runner_context()
        .get_connection_string()
        .expect("")
        .to_string();
    let app_ws_url = ctx.get().app_ws_url();
    let installed_app_id = installed_app_id_for_agent(ctx);
    let reporter = ctx.runner_context().reporter();
    let run_id = ctx.runner_context().get_run_id().to_string();

    let (installed_app_id, cell_id, app_client) = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move {
            log::debug!("Connecting a Holochain admin client: {}", admin_ws_url);
            let client = AdminWebsocket::connect(admin_ws_url, reporter.clone()).await?;

            let key = match agent_pubkey {
                Some(key) => key,
                None => {
                    let key = client
                        .generate_agent_pub_key()
                        .await
                        .map_err(handle_api_err)?;
                    log::debug!("Generated agent pub key: {:}", key);
                    key
                }
            };

            let source = match dna_properties {
                Some(properties) => {
                    let happ = build_happ(app_path, properties).await?;
                    happ
                }
                None => {
                    let content = std::fs::read(app_path)?;
                    AppBundleSource::Bytes(content)
                }
            };
            log::debug!("Installing app with source");
            let app_info = client
                .install_app(InstallAppPayload {
                    source,
                    agent_key: Some(key),
                    installed_app_id: Some(installed_app_id.clone()),
                    roles_settings: None,
                    network_seed: Some(run_id),
                    ignore_genesis_failure: false,
                    allow_throwaway_random_agent_key: false,
                })
                .await
                .map_err(handle_api_err)?;
            log::debug!("Installed app: {:}", installed_app_id);

            client
                .enable_app(installed_app_id.clone())
                .await
                .map_err(handle_api_err)?;
            log::debug!("Enabled app: {:}", installed_app_id);

            let cell_id = get_cell_id_for_role_name(&app_info, role_name)?;
            log::debug!("Got cell id: {:}", cell_id);

            let credentials = client
                .authorize_signing_credentials(AuthorizeSigningCredentialsPayload {
                    cell_id: cell_id.clone(),
                    functions: None,
                })
                .await?;
            log::debug!("Authorized signing credentials");

            let signer = ClientAgentSigner::default();
            signer.add_credentials(cell_id.clone(), credentials);

            let issued = client
                .issue_app_auth_token(installed_app_id.clone().into())
                .await
                .map_err(|e| {
                    anyhow::anyhow!("Could not issue auth token for app client: {:?}", e)
                })?;

            let app_client =
                AppWebsocket::connect(app_ws_url, issued.token, signer.into(), reporter).await?;

            Ok((installed_app_id, cell_id, app_client))
        })
        .context("Failed to install app")?;

    ctx.get_mut().installed_app_id = Some(installed_app_id);
    ctx.get_mut().cell_id = Some(cell_id);
    ctx.get_mut().app_client = Some(app_client);

    Ok(())
}

pub fn custom_install_app_from_bytes<SV>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    app_bytes: &'static [u8],
    role_name: &RoleName,
    agent_pubkey: Option<AgentPubKey>,
    dna_properties: Option<HashMap<String, YamlProperties>>,
) -> WindTunnelResult<()>
where
    SV: UserValuesConstraint,
{
    let admin_ws_url = ctx.get().admin_ws_url();
    let app_ws_url = ctx.get().app_ws_url();
    let installed_app_id = installed_app_id_for_agent(ctx);
    let reporter = ctx.runner_context().reporter();
    let run_id = ctx.runner_context().get_run_id().to_string();

    let (installed_app_id, cell_id, app_client) = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move {
            log::debug!("Connecting a Holochain admin client: {}", admin_ws_url);
            let client = AdminWebsocket::connect(admin_ws_url, reporter.clone()).await?;

            let key = match agent_pubkey {
                Some(key) => key,
                None => {
                    let key = client
                        .generate_agent_pub_key()
                        .await
                        .map_err(handle_api_err)?;
                    log::debug!("Generated agent pub key: {:}", key);
                    key
                }
            };

            let source = match dna_properties {
                Some(properties) => {
                    let happ = build_happ_from_bytes(app_bytes, properties).await?;
                    happ
                }
                None => AppBundleSource::Bytes(app_bytes.to_vec()),
            };
            log::debug!("Installing app with source");
            let app_info = client
                .install_app(InstallAppPayload {
                    source,
                    agent_key: Some(key),
                    installed_app_id: Some(installed_app_id.clone()),
                    roles_settings: None,
                    network_seed: Some(run_id),
                    ignore_genesis_failure: false,
                    allow_throwaway_random_agent_key: false,
                })
                .await
                .map_err(handle_api_err)?;
            log::debug!("Installed app: {:}", installed_app_id);

            client
                .enable_app(installed_app_id.clone())
                .await
                .map_err(handle_api_err)?;
            log::debug!("Enabled app: {:}", installed_app_id);

            let cell_id = get_cell_id_for_role_name(&app_info, role_name)?;
            log::debug!("Got cell id: {:}", cell_id);

            let credentials = client
                .authorize_signing_credentials(AuthorizeSigningCredentialsPayload {
                    cell_id: cell_id.clone(),
                    functions: None,
                })
                .await?;
            log::debug!("Authorized signing credentials");

            let signer = ClientAgentSigner::default();
            signer.add_credentials(cell_id.clone(), credentials);

            let issued = client
                .issue_app_auth_token(installed_app_id.clone().into())
                .await
                .map_err(|e| {
                    anyhow::anyhow!("Could not issue auth token for app client: {:?}", e)
                })?;

            let app_client =
                AppWebsocket::connect(app_ws_url, issued.token, signer.into(), reporter).await?;

            Ok((installed_app_id, cell_id, app_client))
        })
        .context("Failed to install app")?;

    ctx.get_mut().installed_app_id = Some(installed_app_id);
    ctx.get_mut().cell_id = Some(cell_id);
    ctx.get_mut().app_client = Some(app_client);

    Ok(())
}
