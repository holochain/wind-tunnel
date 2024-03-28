use crate::context::HolochainAgentContext;
use crate::runner_context::HolochainRunnerContext;
use anyhow::Context;
use holochain_client_instrumented::prelude::{
    AdminWebsocket, AppAgentWebsocket, AuthorizeSigningCredentialsPayload, ClientAgentSigner,
};
use holochain_conductor_api::CellInfo;
use holochain_types::prelude::{AppBundleSource, ExternIO, InstallAppPayload, RoleName};
use holochain_types::websocket::AllowedOrigins;
use std::path::PathBuf;
use wind_tunnel_runner::prelude::{
    AgentContext, RunnerContext, UserValuesConstraint, WindTunnelResult,
};

/// Sets the `app_ws_url` value in [HolochainRunnerContext] using a valid app port on the target conductor.
///
/// After calling this function you will be able to use the `app_ws_url` in your agent hooks:
/// ```rust
/// use holochain_wind_tunnel_runner::prelude::{HolochainAgentContext, HolochainRunnerContext};
/// use wind_tunnel_runner::prelude::{AgentContext, HookResult};
///
/// fn agent_setup(ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>) -> HookResult {
///     let app_ws_url = ctx.runner_context().get().app_ws_url();
///     Ok(())
/// }
/// ```
///
/// Method:
/// - Connects to an admin port using the connection string from the context.
/// - Lists app interfaces and if there are any, uses the first one.
/// - If there are no app interfaces, attaches a new one.
/// - Reads the current admin URL from the [RunnerContext] and swaps the admin port for the app port.
/// - Sets the `app_ws_url` value in [HolochainRunnerContext].
pub fn configure_app_ws_url(
    ctx: &mut RunnerContext<HolochainRunnerContext>,
) -> WindTunnelResult<()> {
    let admin_ws_url = ctx.get_connection_string().to_string();
    let reporter = ctx.reporter();
    let app_port = ctx
        .executor()
        .execute_in_place(async move {
            log::debug!("Connecting a Holochain admin client: {}", admin_ws_url);
            let mut admin_client = AdminWebsocket::connect(admin_ws_url, reporter)
                .await
                .context("Unable to connect admin client")?;

            let existing_app_ports = admin_client
                .list_app_interfaces()
                .await
                .map_err(|e| anyhow::anyhow!("Conductor API error: {:?}", e))?;
            if !existing_app_ports.is_empty() {
                Ok(*existing_app_ports.first().context("No app ports found")?)
            } else {
                let attached_app_port = admin_client
                    // Don't specify the port, let the conductor pick one
                    .attach_app_interface(0, AllowedOrigins::Any)
                    .await
                    .map_err(|e| anyhow::anyhow!("Conductor API error: {:?}", e))?;
                Ok(attached_app_port)
            }
        })
        .context("Failed to set up app port")?;

    // Use the admin URL with the app port we just got to derive a URL for the app websocket
    let admin_ws_url = ctx.get_connection_string();
    let mut admin_ws_url = url::Url::parse(admin_ws_url)
        .map_err(|e| anyhow::anyhow!("Failed to parse admin URL: {}", e))?;
    admin_ws_url
        .set_port(Some(app_port))
        .map_err(|_| anyhow::anyhow!("Failed to set app port on admin URL"))?;

    ctx.get_mut().app_ws_url = Some(admin_ws_url.to_string());

    Ok(())
}

/// Opinionated app installation which will give you what you need in most cases.
///
/// The [RoleName] you provide is used to find the cell id of the installed app.
///
/// Requires:
/// - The [HolochainRunnerContext] to have a valid `app_ws_url`. Consider calling [configure_app_ws_url] in your setup before using this function.
///
/// Call this function as follows:
/// ```rust
/// use std::path::Path;
/// use holochain_wind_tunnel_runner::prelude::{HolochainAgentContext, HolochainRunnerContext, install_app, AgentContext, HookResult};
///
/// fn agent_setup(ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>) -> HookResult {
///     install_app(ctx, Path::new("path/to/your/happ").to_path_buf(), &"your_role_name".to_string())?;
///     Ok(())
/// }
/// ```
///
/// After calling this function you will be able to use the `installed_app_id`, `cell_id` and `app_agent_client` in your agent hooks:
/// ```rust
/// use holochain_wind_tunnel_runner::prelude::{HolochainAgentContext, HolochainRunnerContext, AgentContext, HookResult};
///
/// fn agent_behaviour(ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>) -> HookResult {
///     let installed_app_id = ctx.get().installed_app_id();
///     let cell_id = ctx.get().cell_id();
///     let app_agent_client = ctx.get().app_agent_client();
///
///     Ok(())
/// }
/// ```
///
/// Method:
/// - Connects to an admin port using the connection string from the runner context.
/// - Generates an agent public key.
/// - Installs the app using the provided `app_path` and the agent public key.
/// - Enables the app.
/// - Authorizes signing credentials.
/// - Connects to the app websocket.
/// - Sets the `installed_app_id`, `cell_id` and `app_agent_client` values in [HolochainAgentContext].
pub fn install_app<SV>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    app_path: PathBuf,
    role_name: &RoleName,
) -> WindTunnelResult<()>
where
    SV: UserValuesConstraint,
{
    let admin_ws_url = ctx.runner_context().get_connection_string().to_string();
    let app_ws_url = ctx.runner_context().get().app_ws_url();
    let agent_id = ctx.agent_id().to_string();
    let reporter = ctx.runner_context().reporter();

    let (installed_app_id, cell_id, app_agent_client) = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move {
            log::debug!("Connecting a Holochain admin client: {}", admin_ws_url);
            let mut client = AdminWebsocket::connect(admin_ws_url, reporter.clone()).await?;

            let key = client
                .generate_agent_pub_key()
                .await
                .map_err(|e| anyhow::anyhow!("Conductor API error: {:?}", e))?;
            log::debug!("Generated agent pub key: {:}", key);

            let installed_app_id = format!("{}-app", agent_id).to_string();
            let app_info = client
                .install_app(InstallAppPayload {
                    source: AppBundleSource::Path(app_path),
                    agent_key: key,
                    installed_app_id: Some(installed_app_id.clone()),
                    membrane_proofs: Default::default(),
                    network_seed: None,
                })
                .await
                .map_err(|e| anyhow::anyhow!("Conductor API error: {:?}", e))?;
            log::debug!("Installed app: {:}", installed_app_id);

            client
                .enable_app(installed_app_id.clone())
                .await
                .map_err(|e| anyhow::anyhow!("Conductor API error: {:?}", e))?;
            log::debug!("Enabled app: {:}", installed_app_id);

            let cell_id = match app_info
                .cell_info
                .get(role_name)
                .ok_or(anyhow::anyhow!("Role not found"))?
                .first()
                .ok_or(anyhow::anyhow!("Cell not found"))?
            {
                CellInfo::Provisioned(c) => c.cell_id.clone(),
                _ => anyhow::bail!("Cell not provisioned"),
            };
            log::debug!("Got cell id: {:}", cell_id);

            let credentials = client
                .authorize_signing_credentials(AuthorizeSigningCredentialsPayload {
                    cell_id: cell_id.clone(),
                    functions: None,
                })
                .await
                .map_err(|e| anyhow::anyhow!("Conductor API error: {:?}", e))?;
            log::debug!("Authorized signing credentials");

            let mut signer = ClientAgentSigner::default();
            signer.add_credentials(cell_id.clone(), credentials);

            let app_agent_client = AppAgentWebsocket::connect(
                app_ws_url,
                installed_app_id.clone(),
                signer.into(),
                reporter,
            )
            .await?;

            Ok((installed_app_id, cell_id, app_agent_client))
        })
        .context("Failed to install app")?;

    ctx.get_mut().installed_app_id = Some(installed_app_id);
    ctx.get_mut().cell_id = Some(cell_id);
    ctx.get_mut().app_agent_client = Some(app_agent_client);

    Ok(())
}

/// Calls a zome function on the cell specified in `ctx.get().cell_id()`.
///
/// Requires:
/// - The [HolochainAgentContext] to have a valid `cell_id`. Consider calling [install_app] in your setup before using this function.
/// - The [HolochainAgentContext] to have a valid `app_agent_client`. Consider calling [install_app] in your setup before using this function.
///
/// Call this function as follows:
/// ```rust
/// use holochain_types::prelude::ActionHash;
/// use holochain_wind_tunnel_runner::prelude::{call_zome, HolochainAgentContext, HolochainRunnerContext, AgentContext, HookResult};
///
/// fn agent_behaviour(ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>) -> HookResult {
///     // Return type determined by why you assign the result to
///     let action_hash: ActionHash = call_zome(
///         ctx,
///         "crud", // zome name
///         "create_sample_entry", // function name
///         "this is a test entry value" // payload
///     )?;
///
///     Ok(())
/// }
/// ```
///
/// Method:
/// - Gets the `cell_id` and `app_agent_client` from the context.
/// - Tries to serialize the input payload.
/// - Calls the zome function using the `app_agent_client`.
/// - Tries to deserialize and return the response.
pub fn call_zome<I, O, SV>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    zome_name: &str,
    fn_name: &str,
    payload: I,
) -> anyhow::Result<O>
where
    O: std::fmt::Debug + serde::de::DeserializeOwned,
    I: serde::Serialize + std::fmt::Debug,
    SV: UserValuesConstraint,
{
    let cell_id = ctx.get().cell_id();
    let mut app_agent_client = ctx.get().app_agent_client();
    ctx.runner_context().executor().execute_in_place(async {
        let result = app_agent_client
            .call_zome(
                cell_id.into(),
                zome_name.into(),
                fn_name.into(),
                ExternIO::encode(payload).context("Encoding failure")?,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Conductor API error: {:?}", e))?;

        result
            .decode()
            .map_err(|e| anyhow::anyhow!("Decoding failure: {:?}", e))
    })
}
