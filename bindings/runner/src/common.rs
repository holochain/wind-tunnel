use crate::bin_path::{WT_HOLOCHAIN_PATH_ENV, holochain_path};
use crate::build_info::holochain_build_info;
use crate::context::HolochainAgentContext;
use crate::holochain_runner::{HolochainConfig, HolochainRunner};
use crate::prelude::CallZomeOptions;
use crate::runner_context::HolochainRunnerContext;
use anyhow::Context;
use holochain_client_instrumented::ToSocketAddr;
use holochain_client_instrumented::prelude::{
    AdminWebsocket, AppWebsocket, AuthorizeSigningCredentialsPayload, ClientAgentSigner,
};
use holochain_conductor_api::{AppInfo, CellInfo};
use holochain_types::prelude::*;
use holochain_types::prelude::{
    AppBundleSource, CellId, ExternIO, InstallAppPayload, InstalledAppId, RoleName,
};
use holochain_types::websocket::AllowedOrigins;
use kitsune2_api::{AgentInfoSigned, DhtArc};
use kitsune2_core::Ed25519Verifier;
use rand::rng;
use rand::seq::SliceRandom;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{env, fs, io};
use wind_tunnel_runner::prelude::{
    AgentContext, HookResult, Reporter, RunnerContext, UserValuesConstraint, WindTunnelResult,
};
use wind_tunnel_summary_model::BuildInfo;

/// Sets the [`HolochainAgentContext::admin_ws_url`], if not already set, getting the value from
/// [`wind_tunnel_runner::context::RunnerContext::connection_string`].
///
/// After calling this function you will be able to use the [`HolochainAgentContext::admin_ws_url`]
/// in your agent hooks:
/// ```rust
/// use holochain_wind_tunnel_runner::prelude::{HolochainAgentContext, HolochainRunnerContext};
/// use wind_tunnel_runner::prelude::{AgentContext, HookResult};
///
/// fn agent_setup(ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>) -> HookResult {
///     let admin_ws_url = ctx.get().admin_ws_url();
///     Ok(())
/// }
/// ```
pub fn configure_admin_ws_url<SV: UserValuesConstraint>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
) -> WindTunnelResult<()> {
    if ctx.get().admin_ws_url.is_none() {
        ctx.get_mut().admin_ws_url = Some(
            ctx.runner_context()
                .get_connection_string()
                .context("Need to call run_holochain_conductor in agent_setup or set connection-string so an admin_port can be established")?
                .to_socket_addr()
                .context("Failed to convert connection-string to admin_ws_url")?,
        );
    }

    Ok(())
}

/// Sets the `app_ws_url` value in [HolochainRunnerContext] using a valid app port on the target conductor.
///
/// After calling this function you will be able to use the `app_ws_url` in your agent hooks:
/// ```rust
/// use holochain_wind_tunnel_runner::prelude::{HolochainAgentContext, HolochainRunnerContext};
/// use wind_tunnel_runner::prelude::{AgentContext, HookResult};
///
/// fn agent_setup(ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>) -> HookResult {
///     let app_ws_url = ctx.get().app_ws_url();
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
pub fn configure_app_ws_url<SV: UserValuesConstraint>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
) -> WindTunnelResult<()> {
    let admin_ws_url = ctx.get().admin_ws_url();
    let reporter = ctx.runner_context().reporter();
    let app_port = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move {
            log::debug!("Connecting a Holochain admin client: {admin_ws_url}");
            let admin_client = AdminWebsocket::connect(admin_ws_url, None, reporter)
                .await
                .context("Unable to connect admin client")?;

            let existing_app_interfaces = admin_client.list_app_interfaces().await?;

            let existing_app_ports = existing_app_interfaces
                .into_iter()
                .filter_map(|interface| {
                    if interface.allowed_origins == AllowedOrigins::Any
                        && interface.installed_app_id.is_none()
                    {
                        Some(interface.port)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            if !existing_app_ports.is_empty() {
                Ok(*existing_app_ports.first().context("No app ports found")?)
            } else {
                let attached_app_port = admin_client
                    // Don't specify the port, let the conductor pick one
                    .attach_app_interface(0, None, AllowedOrigins::Any, None)
                    .await?;
                Ok(attached_app_port)
            }
        })
        .context("Failed to set up app port, is a conductor running?")?;

    // Use the admin URL with the app port we just got to derive a URL for the app websocket
    let mut app_ws_url = admin_ws_url;
    app_ws_url.set_port(app_port);

    ctx.get_mut().app_ws_url = Some(app_ws_url);

    Ok(())
}

/// Opinionated app installation which will give you what you need in most cases.
///
/// The [RoleName] you provide is used to find the cell id within the installed app that you want
/// to call during your scenario.
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
///     let installed_app_id = ctx.get().installed_app_id()?;
///     let cell_id = ctx.get().cell_id();
///     let app_agent_client = ctx.get().app_client();
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
    let admin_ws_url = ctx.get().admin_ws_url();
    let app_ws_url = ctx.get().app_ws_url();
    let installed_app_id = installed_app_id_for_agent(ctx);
    let reporter = ctx.runner_context().reporter();
    let run_id = ctx.runner_context().get_run_id().to_string();

    let (installed_app_id, cell_id, app_client) = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move {
            log::debug!("Connecting a Holochain admin client: {admin_ws_url}");
            let client = AdminWebsocket::connect(admin_ws_url, None, reporter.clone()).await?;

            let key = client.generate_agent_pub_key().await?;
            log::debug!("Generated agent pub key: {key}");

            let content = std::fs::read(app_path)?;

            let app_info = client
                .install_app(InstallAppPayload {
                    source: AppBundleSource::Bytes(bytes::Bytes::from(content)),
                    agent_key: Some(key),
                    installed_app_id: Some(installed_app_id.clone()),
                    roles_settings: None,
                    network_seed: Some(run_id),
                    ignore_genesis_failure: false,
                })
                .await?;
            log::debug!("Installed app: {installed_app_id}");

            client.enable_app(installed_app_id.clone()).await?;
            log::debug!("Enabled app: {installed_app_id}");

            let cell_id = get_cell_id_for_role_name(&app_info, role_name)?;
            log::debug!("Got cell id: {cell_id:?}");

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
                .map_err(|e| anyhow::anyhow!("Could not issue auth token for app client: {e:?}"))?;

            let app_client =
                AppWebsocket::connect(app_ws_url, issued.token, signer.into(), None, reporter)
                    .await?;

            Ok((installed_app_id, cell_id, app_client))
        })
        .context("Failed to install app")?;

    ctx.get_mut().installed_app_id = Some(installed_app_id);
    ctx.get_mut().cell_role_name = Some(role_name.clone());
    ctx.get_mut().cell_id = Some(cell_id);
    ctx.get_mut().app_client = Some(app_client);

    Ok(())
}

/// Used an installed app as though it had been installed by [install_app].
///
/// It doesn't matter whether the app was installed by [install_app], but if it wasn't then it is
/// your responsibility to make sure the naming expectations are met. Namely, the app is installed
/// under `<agent_name>-app`.
///
/// Once this function has run, you should be able to use any functions that would normally use the
/// outputs of [install_app]. This makes it a useful drop-in if you want to run further code against
/// an installed app after a scenario has finished.
///
/// Call this function as follows:
/// ```rust
/// use std::path::Path;
/// use holochain_wind_tunnel_runner::prelude::{HolochainAgentContext, HolochainRunnerContext, use_installed_app, AgentContext, HookResult};
///
/// fn agent_setup(ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>) -> HookResult {
///     use_installed_app(ctx, &"your_role_name".to_string())?;
///     Ok(())
/// }
/// ```
///
/// Method:
/// - Connects to an admin port using the connection string from the runner context.
/// - Generates the expected installed_app_id for this agent.
/// - Gets a list of installed apps and tries to find the matching one by app id.
/// - If the app is not found, or is not in the Running state, then error.
/// - Authorizes signing credentials.
/// - Connects to the app websocket.
/// - Sets the `installed_app_id`, `cell_id` and `app_agent_client` values in [HolochainAgentContext].
pub fn use_installed_app<SV>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    role_name: &RoleName,
) -> HookResult
where
    SV: UserValuesConstraint,
{
    let admin_ws_url = ctx.get().admin_ws_url();
    let app_ws_url = ctx.get().app_ws_url();
    let reporter = ctx.runner_context().reporter();
    let installed_app_id = installed_app_id_for_agent(ctx);

    let (installed_app_id, cell_id, app_client) = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move {
            let client = AdminWebsocket::connect(admin_ws_url, None, reporter.clone()).await?;

            let app_infos = client.list_apps(None).await?;
            let app_info = app_infos
                .into_iter()
                .find(|app_info| app_info.installed_app_id == installed_app_id)
                .ok_or(anyhow::anyhow!("App not found: {installed_app_id:?}"))?;

            if app_info.status != AppStatus::Enabled {
                anyhow::bail!("App is not enabled: {installed_app_id:?}");
            }

            let cell_id = get_cell_id_for_role_name(&app_info, role_name)?;

            let credentials = client
                .authorize_signing_credentials(AuthorizeSigningCredentialsPayload {
                    cell_id: cell_id.clone(),
                    functions: None,
                })
                .await?;

            let signer = ClientAgentSigner::default();
            signer.add_credentials(cell_id.clone(), credentials);

            let issued = client
                .issue_app_auth_token(installed_app_id.clone().into())
                .await
                .map_err(|e| anyhow::anyhow!("Could not issue auth token for app client: {e:?}"))?;

            let app_client =
                AppWebsocket::connect(app_ws_url, issued.token, signer.into(), None, reporter)
                    .await?;

            Ok((installed_app_id, cell_id, app_client))
        })?;

    ctx.get_mut().installed_app_id = Some(installed_app_id);
    ctx.get_mut().cell_id = Some(cell_id);
    ctx.get_mut().app_client = Some(app_client);

    Ok(())
}

/// Tries to wait for a minimum number of agents to be discovered.
///
/// If you call this function in your agent setup, then the scenario will become configurable using
/// the `MIN_AGENTS` environment variable. The default value is 2.
///
/// Note that the number of agents seen by each node includes itself. So having two conductors with
/// one agent on each, means that each node will immediately see one agent after app installation.
///
/// Example:
/// ```rust
/// use std::path::Path;
/// use std::time::Duration;
/// use holochain_wind_tunnel_runner::prelude::{HolochainAgentContext, HolochainRunnerContext, AgentContext, HookResult, install_app, try_wait_for_min_agents};
///
/// fn agent_setup(ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>) -> HookResult {
///     install_app(ctx, Path::new("path/to/your/happ").to_path_buf(), &"your_role_name".to_string())?;
///     try_wait_for_min_agents(ctx, Duration::from_secs(60))?;
///     Ok(())
/// }
/// ```
///
/// Note that if no apps have been installed, you are waiting for too many agents, or anything else
/// prevents enough agents being discovered then the function will wait up to the `wait_for` duration
/// before continuing. It will not fail if too few agents were discovered.
///
/// Note that the smallest resolution is 1s. This is because the function will sleep between
/// querying agents from the conductor. You could probably not use this function for performance
/// testing peer discovery!
pub fn try_wait_for_min_agents<SV>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    wait_for: Duration,
) -> HookResult
where
    SV: UserValuesConstraint,
{
    let admin_ws_url = ctx.get().admin_ws_url();
    let reporter = ctx.runner_context().reporter();
    let agent_name = ctx.agent_name().to_string();

    let min_agents = std::env::var("MIN_AGENTS")
        .ok()
        .map(|s| s.parse().expect("MIN_AGENTS must be a number"))
        .unwrap_or(2);

    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            let client = AdminWebsocket::connect(admin_ws_url, None, reporter.clone()).await?;

            let start_discovery = Instant::now();
            for _ in 0..wait_for.as_secs() {
                let agent_list = client.agent_info(None).await?;

                if agent_list.len() >= min_agents {
                    break;
                }

                tokio::time::sleep(Duration::from_secs(1)).await;
            }

            println!(
                "Discovery for agent {} took: {}s",
                agent_name,
                start_discovery.elapsed().as_secs()
            );

            Ok(())
        })?;

    Ok(())
}

/// Tries to wait for a full arc to show up in the agent infos.
///
/// Example:
/// ```rust
/// use std::path::Path;
/// use std::time::Duration;
/// use holochain_wind_tunnel_runner::prelude::{HolochainAgentContext, HolochainRunnerContext, AgentContext, HookResult, install_app, try_wait_until_full_arc_peer_discovered};
///
/// fn agent_setup(ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>) -> HookResult {
///     install_app(ctx, Path::new("path/to/your/happ").to_path_buf(), &"your_role_name".to_string())?;
///     if ctx.assigned_behaviour() == "zero" {
///        try_wait_until_full_arc_peer_discovered(ctx)?;
///     }
///     Ok(())
/// }
/// ```
pub fn try_wait_until_full_arc_peer_discovered<SV>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
) -> HookResult
where
    SV: UserValuesConstraint,
{
    let start_discovery = Instant::now();

    loop {
        // If the wait time specified in the wait_for argument exceeds the
        // duration that the scenario is supposed to run, we should break
        // the loop here.
        if ctx.shutdown_listener().should_shutdown() {
            break;
        }

        let app_client = ctx.get().app_client();
        let full_arc_node_discovered =
            ctx.runner_context()
                .executor()
                .execute_in_place(async move {
                    let agent_infos_encoded = app_client.agent_info(None).await?;

                    let full_arc_nodes: Vec<Arc<AgentInfoSigned>> = agent_infos_encoded
                        .iter()
                        .filter_map(|agent_info| {
                            AgentInfoSigned::decode(&Ed25519Verifier, agent_info.as_bytes()).ok()
                        })
                        .filter(|agent_info| agent_info.storage_arc == DhtArc::FULL)
                        .collect();

                    if !full_arc_nodes.is_empty() {
                        return Ok(true);
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    Ok(false)
                })?;

        if full_arc_node_discovered {
            // Since we don't know how many full arc nodes we expect in total,
            // we wait an additional 5 seconds for other full arc nodes to have
            // time to join the party as well.
            ctx.runner_context()
                .executor()
                .execute_in_place(async move {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    Ok(())
                })?;
            break;
        }
    }

    println!(
        "Discovery of full arc took: {}s",
        start_discovery.elapsed().as_secs()
    );

    Ok(())
}

/// Uninstall an application. Intended to be used by scenarios that clean up after themselves or
/// need to uninstall and re-install the same application.
///
/// Requires:
/// - Either you provide the `installed_app_id` or the [HolochainAgentContext] must have an `installed_app_id`.
///   Note that this means that when passing `None`, only the last app that was installed using [install_app] will be uninstalled.
///
/// Call this function as follows:
/// ```rust
/// use std::path::Path;
/// use holochain_wind_tunnel_runner::prelude::{HolochainAgentContext, HolochainRunnerContext, uninstall_app, AgentContext, HookResult};
///
/// fn agent_teardown(ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>) -> HookResult {
///     uninstall_app(ctx, None)?;
///     Ok(())
/// }
/// ```
///
/// Or if you are uninstalling in the agent behaviour and in the teardown:
/// ```rust
/// use std::path::Path;
/// use holochain_wind_tunnel_runner::prelude::{HolochainAgentContext, HolochainRunnerContext, uninstall_app, AgentContext, HookResult, install_app};
///
/// fn agent_behaviour(ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>) -> HookResult {
///    install_app(ctx, Path::new("path/to/your/happ").to_path_buf(), &"your_role_name".to_string())?;
///    uninstall_app(ctx, None)?;
///    Ok(())
/// }
///
/// fn agent_teardown(ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>) -> HookResult {
///     // The app may have already been uninstalled if the scenario stopped after uninstalling the app but the agent behaviour is
///     // not guaranteed to complete so we don't error when uninstalling here.
///     uninstall_app(ctx, None).ok();
///     Ok(())
/// }
/// ```
///
/// Method:
/// - Either uses the provided `installed_app_id` or gets the `installed_app_id` from the agent context.
/// - Connects to an admin port using the connection string from the runner context.
/// - Uninstalls the specified app and returns the result.
pub fn uninstall_app<SV>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    installed_app_id: Option<InstalledAppId>,
) -> HookResult
where
    SV: UserValuesConstraint,
{
    let admin_ws_url = ctx.get().admin_ws_url();

    let installed_app_id = installed_app_id.or_else(|| ctx.get().installed_app_id().ok());
    if installed_app_id.is_none() {
        // If there is no installed app id, we can't uninstall anything
        log::info!("No installed app id found, skipping uninstall");
        return Ok(());
    }

    let reporter = ctx.runner_context().reporter();

    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            let admin_client = AdminWebsocket::connect(admin_ws_url, None, reporter).await?;

            admin_client
                .uninstall_app(installed_app_id.unwrap())
                .await?;
            Ok(())
        })?;

    Ok(())
}

/// Calls a zome function on the cell specified in `ctx.get().cell_id()`.
///
/// This is equivalent to calling [`call_zome_with_options`] with default options,
/// so it will use the default timeout and other default settings for the call.
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
    call_zome_with_options(ctx, zome_name, fn_name, payload, CallZomeOptions::default())
}

/// Calls a zome function on the cell specified in `ctx.get().cell_id()`.
/// This is equivalent to [`call_zome`] but with the addition of being able to specify [`CallZomeOptions`] for the call.
///
/// Requires:
///
/// - The [`HolochainAgentContext`] to have a valid `cell_id`. Consider calling [`install_app`] in your setup before using this function.
/// - The [`HolochainAgentContext`] to have a valid `app_agent_client`. Consider calling [`install_app`] in your setup before using this function.
///
/// Call this function as follows:
///
/// ```rust
/// use std::time::Duration;
/// use holochain_types::prelude::ActionHash;
/// use holochain_wind_tunnel_runner::prelude::{call_zome_with_options, HolochainAgentContext, HolochainRunnerContext, AgentContext, HookResult, CallZomeOptions};
///
/// fn agent_behaviour(ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>) -> HookResult {
///     // Return type determined by why you assign the result to
///     let action_hash: ActionHash = call_zome_with_options(
///         ctx,
///         "crud", // zome name
///         "create_sample_entry", // function name
///         "this is a test entry value", // payload
///         CallZomeOptions::new().with_timeout(Duration::from_secs(30)) // example option, set a timeout of 30s for this call
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
pub fn call_zome_with_options<I, O, SV>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    zome_name: &str,
    fn_name: &str,
    payload: I,
    options: CallZomeOptions,
) -> anyhow::Result<O>
where
    O: std::fmt::Debug + serde::de::DeserializeOwned,
    I: serde::Serialize + std::fmt::Debug,
    SV: UserValuesConstraint,
{
    let cell_id = ctx.get().cell_id();
    let app_agent_client = ctx.get().app_client();
    ctx.runner_context().executor().execute_in_place(async {
        let result = app_agent_client
            .call_zome(
                cell_id.into(),
                zome_name,
                fn_name,
                ExternIO::encode(payload).context("Encoding failure")?,
                options,
            )
            .await?;

        result
            .decode()
            .map_err(|e| anyhow::anyhow!("Decoding failure: {e:?}"))
    })
}

/// Get a randomized list of peers connected to the conductor in the `ctx` for a given cell.
///
/// Requires:
/// - The [HolochainAgentContext] to have a valid `cell_id`. Consider calling [install_app] in your setup before using this function.
///
/// Call this function as follows:
/// ```rust
/// use holochain_types::prelude::ActionHash;
/// use holochain_wind_tunnel_runner::prelude::{call_zome, HolochainAgentContext, HolochainRunnerContext, AgentContext, HookResult, get_peer_list_randomized};
///
/// fn agent_behaviour(ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>) -> HookResult {
///     let peer_list = get_peer_list_randomized(ctx)?;
///     println!("Connected peers: {:?}", peer_list);
///     Ok(())
/// }
/// ```
///
/// Method:
/// - calls `app_agent_info` on the websocket.
/// - filters out self
/// - shuffles the list
pub fn get_peer_list_randomized<SV>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
) -> WindTunnelResult<Vec<AgentPubKey>>
where
    SV: UserValuesConstraint,
{
    let cell_id = ctx.get().cell_id();
    let reporter: Arc<Reporter> = ctx.runner_context().reporter();
    let admin_ws_url = ctx.get().admin_ws_url();

    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            let admin_client = AdminWebsocket::connect(admin_ws_url, None, reporter).await?;
            // No more agents available to signal, get a new list.
            // This is also the initial condition.
            let agent_infos_encoded = admin_client
                .agent_info(None)
                .await
                .context("Failed to get agent info")?;
            let mut peer_list = Vec::with_capacity(agent_infos_encoded.len().saturating_sub(1));
            for info_encoded in agent_infos_encoded {
                let info_decoded =
                    AgentInfoSigned::decode(&Ed25519Verifier, info_encoded.as_bytes())?;
                let agent_pub_key = AgentPubKey::from_k2_agent(&info_decoded.agent);

                // Add all agents except for ourselves
                if &agent_pub_key != cell_id.agent_pubkey() {
                    peer_list.push(agent_pub_key);
                }
            }
            peer_list.shuffle(&mut rng());
            Ok(peer_list)
        })
}

fn installed_app_id_for_agent<SV>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
) -> String
where
    SV: UserValuesConstraint,
{
    let agent_name = ctx.agent_name().to_string();
    format!("{agent_name}-app").to_string()
}

fn get_cell_id_for_role_name(app_info: &AppInfo, role_name: &RoleName) -> anyhow::Result<CellId> {
    match app_info
        .cell_info
        .get(role_name)
        .ok_or(anyhow::anyhow!("Role not found"))?
        .first()
        .ok_or(anyhow::anyhow!("Cell not found"))?
    {
        CellInfo::Provisioned(c) => Ok(c.cell_id.clone()),
        _ => anyhow::bail!("Cell not provisioned"),
    }
}

/// If [`wind_tunnel_runner::prelude::RunnerContext::connection_string`] is not set then this
/// function runs an instance of the Holochain conductor, using the configuration built from the
/// [`HolochainAgentContext::holochain_config`] and stores the running process in
/// [`HolochainAgentContext::holochain_runner`].
///
/// This function also creates an admin interface bound to a random, available port and sets
/// [`HolochainAgentContext::admin_ws_url`] to a 127.0.0.1 address with that port.
///
/// Override the binary used to start the conductor with the [`WT_HOLOCHAIN_PATH_ENV`] environment
/// variable.
pub fn run_holochain_conductor<SV: UserValuesConstraint>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
) -> WindTunnelResult<()> {
    if ctx.runner_context().get_connection_string().is_some() {
        log::info!(
            "connection-string is set so assuming a Holochain conductor is running externally"
        );
        return Ok(());
    }

    let holochain_path = holochain_path()?;
    log::debug!(
        "Using holochain binary at path: {}",
        holochain_path.display()
    );
    ctx.get_mut()
        .holochain_config_mut()
        .with_bin_path(holochain_path);

    let admin_port = {
        // Bind to an ephemeral port to reserve it, then release before starting the conductor.
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0))
            .context("Failed to bind ephemeral port for admin interface")?;
        listener.local_addr()?.port()
    };
    let conductor_root_path = {
        let mut path = env::temp_dir();
        path.push(ctx.runner_context().get_run_id());
        path.push(ctx.agent_name());

        path
    };
    let holochain_metrics_path = {
        let mut path: PathBuf = std::env::var("WT_METRICS_DIR")
            .expect("WT_METRICS_DIR must be set.")
            .into();
        path.push(format!(
            "holochain-{}-{}.influx",
            ctx.runner_context().get_run_id(),
            ctx.agent_name()
        ));

        path
    };
    let agent_name = ctx.agent_name().to_string();
    ctx.get_mut()
        .holochain_config_mut()
        .with_conductor_root_path(&conductor_root_path)
        .with_admin_port(admin_port)
        .with_agent_name(agent_name)
        .with_metrics_path(&holochain_metrics_path);

    let config = ctx.get_mut().take_holochain_config().build()?;

    ctx.get_mut().holochain_runner = match ctx.runner_context().executor().execute_in_place(
        create_and_start_holochain_conductor(config, &conductor_root_path),
    ) {
        Ok(runner) => Some(runner),
        Err(err) => {
            log::error!("Failed to start Holochain conductor: {err}");
            // force stop conductor if we failed to start it and return error
            ctx.runner_context().force_stop_scenario();
            return Err(err);
        }
    };

    ctx.get_mut().admin_ws_url = Some(
        format!("ws://127.0.0.1:{admin_port}")
            .to_socket_addr()
            .with_context(|| {
                format!("Failed to create admin_ws_url from 'ws://127.0.0.1:{admin_port}'")
            })?,
    );

    Ok(())
}

/// Starts a Holochain conductor with the provided configuration.
///
/// If starting the conductor fails, attempts to clean up the conductor root directory.
/// Returns [`HolochainRunner`] on success.
async fn create_and_start_holochain_conductor(
    config: HolochainConfig,
    conductor_root_path: &Path,
) -> anyhow::Result<HolochainRunner> {
    let mut err = match async {
        let mut runner = HolochainRunner::create(&config)?;
        log::info!("Created runner {runner:?}");
        runner.run().await?;
        anyhow::Ok(runner)
    }
    .await
    {
        Ok(runner) => return Ok(runner),
        Err(err) => err,
    };

    // in case of error, clean up the conductor directory
    log::trace!("Error whilst starting conductor so cleaning up directory");
    if let Err(err) = fs::remove_dir_all(conductor_root_path) {
        log::error!("Failed to cleanup the conductor files: {err}");
    } else {
        log::info!("Successfully cleaned up the conductor files after error");
    }
    if let Some(parent) = conductor_root_path.parent()
        && fs::remove_dir(parent).is_ok()
    {
        log::info!("Successfully cleaned up all conductor directories after error");
    }

    if let Some(io_error) = err.root_cause().downcast_ref::<io::Error>()
        && io_error.kind() == io::ErrorKind::NotFound
    {
        if let Err(_) | Ok("holochain") = env::var(WT_HOLOCHAIN_PATH_ENV).as_deref() {
            err = err.context("'holochain' binary not found in your PATH");
        } else {
            err = err.context(format!("Cannot run 'holochain' binary found at the path provided with '{WT_HOLOCHAIN_PATH_ENV}'"));
        }
    }

    Err(err)
}

/// Helper function to be called from `agent_setup` to config and start a conductor with admin and
/// app URLs.
///
/// Calls [`run_holochain_conductor`] which runs a conductor with the config from
/// [`HolochainAgentContext::holochain_config`] if
/// [`wind_tunnel_runner::prelude::RunnerContext::connection_string`] is not set, then sets
/// [`HolochainAgentContext::admin_ws_url`] and [`HolochainAgentContext::app_ws_url`]
///
/// Override the binary used to start the conductor with the [`WT_HOLOCHAIN_PATH_ENV`] environment
/// variable.
pub fn start_conductor_and_configure_urls<SV: UserValuesConstraint>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
) -> WindTunnelResult<()> {
    run_holochain_conductor(ctx)?;
    configure_admin_ws_url(ctx)?;
    configure_app_ws_url(ctx)
}

/// Get build info from holochain binary
pub fn conductor_build_info(
    runner_ctx: Arc<RunnerContext<HolochainRunnerContext>>,
) -> WindTunnelResult<Option<BuildInfo>> {
    if runner_ctx.get_connection_string().is_some() {
        log::info!(
            "connection-string is set so assuming a Holochain conductor is running externally"
        );
        return Ok(None);
    }

    let holochain_path = holochain_path()?;
    let holochain_build_info = holochain_build_info(holochain_path)?;
    let build_info = BuildInfo {
        info_type: "holochain".to_string(),
        info: serde_json::to_value(holochain_build_info)?,
    };
    Ok(Some(build_info))
}

/// Stops the Holochain conductor if one is running.
///
/// This function will take the `holochain_runner` from the context and drop it, which will
/// gracefully shut down the conductor process and clean up its directories.
///
/// If no conductor is running this function does nothing.
/// If using an external conductor via connection-string, this function does nothing.
///
/// Call this function as follows:
/// ```rust
/// use holochain_wind_tunnel_runner::prelude::{HolochainAgentContext, HolochainRunnerContext, stop_holochain_conductor, AgentContext, HookResult};
///
/// fn agent_behaviour(ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>) -> HookResult {
///     stop_holochain_conductor(ctx)?;
///     Ok(())
/// }
/// ```
pub fn stop_holochain_conductor<SV: UserValuesConstraint>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
) -> WindTunnelResult<()> {
    if let Some(mut runner) = ctx.get_mut().holochain_runner.take() {
        log::info!("Stopping Holochain conductor");
        runner.shutdown();
        log::info!("Holochain conductor stopped");

        ctx.get_mut().holochain_runner = Some(runner);
    } else {
        log::debug!("No Holochain conductor is running, so nothing to stop");
    }

    Ok(())
}

/// Starts an already created Holochain conductor with the same configuration and installed apps.
///
/// If no conductor was created, this function does nothing.
/// If using an external conductor via connection-string, this function does nothing.
pub fn start_holochain_conductor<SV: UserValuesConstraint>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
) -> WindTunnelResult<()> {
    log::info!(
        "Restarting Holochain conductor {:?}",
        ctx.get().holochain_runner
    );
    if let Some(mut runner) = ctx.get_mut().holochain_runner.take() {
        log::info!("Restarting Holochain conductor");

        if let Err(err) = ctx
            .runner_context()
            .executor()
            .execute_in_place(runner.run())
        {
            log::error!("Failed to restart Holochain conductor: {err}");

            // force stop conductor if we failed to restart it and return error
            ctx.runner_context().force_stop_scenario();
            return Err(err);
        }
        log::info!("Holochain conductor restarted");

        ctx.get_mut().holochain_runner = Some(runner);

        configure_admin_ws_url(ctx)?;
        configure_app_ws_url(ctx)?;
        use_installed_app(ctx, &ctx.get().cell_role_name())?;
    } else {
        log::debug!(
            "No Holochain runner in context, did you forget to call create_and_start_holochain_conductor?"
        );
    }

    Ok(())
}
