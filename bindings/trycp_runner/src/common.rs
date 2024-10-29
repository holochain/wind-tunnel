use crate::context::TryCPAgentContext;
use crate::runner_context::TryCPRunnerContext;
use anyhow::{bail, Context};
use holochain_client::AuthorizeSigningCredentialsPayload;
use holochain_conductor_api::{CellInfo, IssueAppAuthenticationTokenPayload};
use holochain_types::app::{AppBundle, AppBundleSource, InstallAppPayload};
use holochain_types::prelude::RoleName;
use holochain_types::websocket::AllowedOrigins;
use log::{debug, warn};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use trycp_client_instrumented::prelude::TryCPClient;
use wind_tunnel_runner::prelude::{
    run, AgentContext, HookResult, ScenarioDefinitionBuilder, UserValuesConstraint,
    WindTunnelResult,
};

/// Connects to a TryCP server using the current agent index and the list of targets.
///
/// Call this function as follows:
/// ```rust
/// use std::path::Path;
/// use trycp_wind_tunnel_runner::prelude::{AgentContext, connect_trycp_client, HookResult, TryCPAgentContext, TryCPRunnerContext};
///
/// fn agent_setup(ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>) -> HookResult {
///     connect_trycp_client(ctx)?;
///     Ok(())
/// }
/// ```
pub fn connect_trycp_client<SV: UserValuesConstraint>(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<SV>>,
) -> HookResult {
    let agent_index = ctx.agent_index();

    let nodes = ctx.runner_context().get_connection_string().split(',');
    let target = ctx
        .runner_context()
        .get_connection_string()
        .split(',')
        .nth(agent_index % nodes.count());

    // This should never happen because the behaviour assignment should have checked that there were enough agents.
    let target = target
        .ok_or_else(|| anyhow::anyhow!("Not enough targets to pick a target URL for agent",))?;

    let signer = Arc::new(holochain_client::ClientAgentSigner::default());
    let reporter = ctx.runner_context().reporter();

    let client = ctx.runner_context().executor().execute_in_place({
        let signer = signer.clone();
        async move { TryCPClient::connect(target, signer.clone(), reporter).await.with_context(|| format!("Could not connect TryCP client, is there a server running and reachable at {}?", target)) }
    })?;

    ctx.get_mut().trycp_client = Some(client);
    ctx.get_mut().signer = Some(signer);

    Ok(())
}

/// Opinionated app installation which will give you what you need in most cases.
///
/// The [RoleName] you provide is used to find the cell id within the installed app that you want
/// to call during your scenario.
///
/// Requires:
/// - The [TryCPAgentContext] must already have a connected TryCP client. You can use
///   `connect_trycp_client` to do this.
///
/// Call this function as follows:
/// ```rust
/// use std::path::Path;
/// use trycp_wind_tunnel_runner::prelude::{TryCPAgentContext, TryCPRunnerContext, AgentContext, HookResult, install_app};
///
/// fn agent_setup(ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>) -> HookResult {
///     install_app(ctx, Path::new("path/to/your/happ").to_path_buf(), &"your_role_name".to_string())?;
///     Ok(())
/// }
/// ```
///
/// After calling this function you will be able to use the `app_port` and `cell_id` in your agent hooks:
/// ```rust
///
/// use trycp_wind_tunnel_runner::prelude::{TryCPAgentContext, TryCPRunnerContext, AgentContext, HookResult};
///
/// fn agent_behaviour(ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>) -> HookResult {
///     let app_agent_client = ctx.get().app_port();
///     let cell_id = ctx.get().cell_id();
///
///     Ok(())
/// }
/// ```
///
/// Method:
/// - Uses the existing connection to a TryCP server.
/// - Generates an agent public key.
/// - Installs the app using the provided `app_path` and the agent public key.
/// - Enables the app.
/// - Attaches an app interface.
/// - Authorizes signing credentials.
/// - Registers the signing credentials so that they will be available for zome calls.
/// - Sets the `app_port` and `cell_id` values in [TryCPAgentContext].
pub fn install_app<SV>(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<SV>>,
    app_path: PathBuf,
    role_name: &RoleName,
) -> WindTunnelResult<()>
where
    SV: UserValuesConstraint,
{
    let run_id = ctx.runner_context().get_run_id().to_string();
    let client = ctx.get().trycp_client();
    let agent_name = ctx.agent_name().to_string();

    let (app_port, cell_id, credentials) = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move {
            let agent_key = client
                .generate_agent_pub_key(agent_name.clone(), None)
                .await
                .context("Failed to generate a new agent pub key")?;

            let content = std::fs::read(app_path)?;

            let installed_app_id = format!("{}-app", agent_name).to_string();
            let app_info = client
                .install_app(
                    agent_name.clone(),
                    InstallAppPayload {
                        source: AppBundleSource::Bundle(AppBundle::decode(&content)?),
                        agent_key: Some(agent_key),
                        installed_app_id: Some(installed_app_id.clone()),
                        membrane_proofs: Default::default(),
                        network_seed: Some(run_id),
                        existing_cells: Default::default(),
                        ignore_genesis_failure: false,
                        allow_throwaway_random_agent_key: false,
                    },
                    // Allow more time to install the app when running many agents. The upload of
                    // the app bundle can take some time when targeting many nodes.
                    Some(Duration::from_secs(180)),
                )
                .await
                .context("App install request failed")?;

            let enable_result = client
                .enable_app(agent_name.clone(), app_info.installed_app_id.clone(), None)
                .await
                .context("Enable app failed")?;
            if !enable_result.errors.is_empty() {
                return Err(anyhow::anyhow!(
                    "Failed to enable app: {:?}",
                    enable_result.errors
                ));
            }

            let app_port = client
                .attach_app_interface(agent_name.clone(), None, AllowedOrigins::Any, None, None)
                .await
                .context("Could not attach an app interface")?;

            let issued = client
                .issue_app_auth_token(
                    agent_name.clone(),
                    IssueAppAuthenticationTokenPayload {
                        installed_app_id,
                        expiry_seconds: 30,
                        single_use: true,
                    },
                    None,
                )
                .await
                .context("Request to issue an app authentication token failed")?;

            client
                .connect_app_interface(issued.token, app_port, None)
                .await
                .context("App interface connection failed")?;

            let cell_id = match app_info
                .cell_info
                .get(role_name)
                .ok_or(anyhow::anyhow!("Role not found"))?
                .first()
                .ok_or(anyhow::anyhow!("Cell not found"))?
            {
                CellInfo::Provisioned(pc) => pc.cell_id.clone(),
                _ => anyhow::bail!("Cell not provisioned"),
            };
            log::debug!("Got cell id: {:}", cell_id);

            let credentials = client
                .authorize_signing_credentials(
                    agent_name.clone(),
                    AuthorizeSigningCredentialsPayload {
                        cell_id: cell_id.clone(),
                        functions: None, // Equivalent to all functions
                    },
                    None,
                )
                .await
                .context("Could not authorize signing credentials")?;

            Ok((app_port, cell_id, credentials))
        })
        .context("Failed to install app")?;

    ctx.get_mut().app_port = Some(app_port);
    ctx.get_mut().cell_id = Some(cell_id.clone());
    ctx.get_mut().signer().add_credentials(cell_id, credentials);

    Ok(())
}

/// Tries to wait for a minimum number of peers to be discovered.
///
/// If you call this function in you agent setup then the scenario will become configurable using
/// the `MIN_PEERS` environment variable. The default value is 2.
///
/// Note that the number of peers seen by each node includes itself. So having two nodes means that
/// each node will immediately see one peer after app installation.
///
/// Example:
/// ```rust
/// use std::path::Path;
/// use std::time::Duration;
/// use trycp_wind_tunnel_runner::prelude::{TryCPAgentContext, TryCPRunnerContext, AgentContext, HookResult, install_app, try_wait_for_min_peers};
///
/// fn agent_setup(ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>) -> HookResult {
///     install_app(ctx, Path::new("path/to/your/happ").to_path_buf(), &"your_role_name".to_string())?;
///     try_wait_for_min_peers(ctx, Duration::from_secs(60))?;
///     Ok(())
/// }
/// ```
///
/// Note that if no apps have been installed, you are waiting for too many peers, or anything else
/// prevents enough peers being discovered then the function will wait up to the `wait_for` duration
/// before continuing. It will not fail if too few peers were discovered.
///
/// Note that the smallest resolution is 1s. This is because the function will sleep between
/// querying peers from the conductor. You could probably not use this function for performance
/// testing peer discovery!
pub fn try_wait_for_min_peers<SV>(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<SV>>,
    wait_for: Duration,
) -> HookResult
where
    SV: UserValuesConstraint,
{
    static MIN_PEERS: OnceLock<usize> = OnceLock::new();

    let client = ctx.get().trycp_client();
    let agent_name = ctx.agent_name().to_string();

    let min_peers = *MIN_PEERS.get_or_init(|| {
        std::env::var("MIN_PEERS")
            .ok()
            .map(|s| s.parse().expect("MIN_PEERS must be a number"))
            .unwrap_or(2)
    });
    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            let start_discovery = Instant::now();
            for _ in 0..wait_for.as_secs() {
                let agent_list = client.agent_info(agent_name.clone(), None, None).await?;

                if agent_list.len() >= min_peers {
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

/// Dumps the logs from the Holochain conductor and lair keystore managed by the TryCP server for the current agent.
///
/// The stderr output for Lair keystore will be downloaded to `logs/{run_id}/{agent_name}/lair-stderr.log`.
/// The stdout and stderr output for the Holochain conductor will be downloaded to `logs/{run_id}/{agent_name}/conductor-stdout.log` and `logs/{run_id}/{agent_name}/conductor-stderr.log` respectively.
///
/// Example:
/// ```rust
/// use trycp_wind_tunnel_runner::prelude::{TryCPAgentContext, TryCPRunnerContext, AgentContext, HookResult, dump_logs};
///
/// fn agent_teardown(ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>) -> HookResult {
///     dump_logs(ctx)?;
///     Ok(())
/// }
///```
///
/// Note that once you reset the TryCP server using [reset_trycp_remote] the logs will be deleted.
/// You should call this function before resetting the server if you want to keep the logs.
///
pub fn dump_logs<SV>(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<SV>>,
) -> HookResult
where
    SV: UserValuesConstraint,
{
    let client = ctx.get().trycp_client();
    let agent_name = ctx.agent_name().to_string();
    let run_id = ctx.runner_context().get_run_id();

    let logs = ctx.runner_context().executor().execute_in_place({
        let agent_name = agent_name.clone();
        async move {
            let logs = client
                .download_logs(agent_name, Some(Duration::from_secs(10 * 60)))
                .await
                .context("Failed to download logs")?;
            Ok(logs)
        }
    })?;

    let path = std::env::current_dir()
        .context("Failed to get current directory")?
        .join("logs")
        .join(run_id)
        .join(agent_name);
    std::fs::create_dir_all(&path)
        .with_context(|| format!("Failed to create log directory at {path:?}"))?;

    std::fs::write(path.join("conductor-stdout.log"), logs.conductor_stdout)?;
    std::fs::write(path.join("conductor-stderr.log"), logs.conductor_stderr)?;

    Ok(())
}

/// Shuts down the Holochain conductor managed by the TryCP server for the current agent.
///
/// You *MUST* call this function in your agent teardown. Otherwise, dropping the agent context will
/// panic when the TryCP client is dropped.
///
/// ```rust
/// use trycp_wind_tunnel_runner::prelude::{TryCPRunnerContext, AgentContext, TryCPAgentContext, HookResult, shutdown_remote};
///
/// fn agent_teardown(ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>) -> HookResult {
///     shutdown_remote(ctx)?;
///     Ok(())
/// }
/// ```
pub fn shutdown_remote<SV>(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<SV>>,
) -> HookResult
where
    SV: UserValuesConstraint,
{
    let client = ctx.get().trycp_client();
    let agent_name = ctx.agent_name().to_string();

    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            client.shutdown(agent_name.clone(), None, None).await?;
            Ok(())
        })?;

    Ok(())
}

/// Asks the TryCP server to reset all state for managed Holochain instances.
///
/// You must call `connect_trycp_client` before calling this function. Or otherwise ensure that the
/// [TryCPAgentContext] has a TryCPClient set.
///
/// Call this function as follows:
/// ```rust
/// use std::path::Path;
/// use trycp_wind_tunnel_runner::prelude::{AgentContext, connect_trycp_client, HookResult, reset_trycp_remote, TryCPAgentContext, TryCPRunnerContext};
///
/// fn agent_setup(ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>) -> HookResult {
///     connect_trycp_client(ctx)?;
///     reset_trycp_remote(ctx)?;
///     Ok(())
/// }
/// ```
pub fn reset_trycp_remote<SV: UserValuesConstraint>(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<SV>>,
) -> HookResult {
    let client = ctx.get().trycp_client();

    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            client.reset(None).await?;
            Ok(())
        })?;

    Ok(())
}

/// Disconnects the TryCP client.
///
/// You must call `connect_trycp_client` before calling this function. Or otherwise ensure that the
/// [TryCPAgentContext] has a TryCPClient set.
///
/// Note that you *must* call this function in the agent teardown, otherwise the `Drop`
/// implementation for the TrycpClient will panic when the runner drops the agent context.
///
/// Call this function as follows:
/// ```rust
/// use std::path::Path;
/// use trycp_wind_tunnel_runner::prelude::{AgentContext, connect_trycp_client, disconnect_trycp_client, HookResult, reset_trycp_remote, TryCPAgentContext, TryCPRunnerContext};
///
/// fn agent_setup(ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>) -> HookResult {
///     connect_trycp_client(ctx)?;
///     Ok(())
/// }
///
/// fn agent_teardown(ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>) -> HookResult {
///     disconnect_trycp_client(ctx)?;
///     Ok(())
/// }
/// ```
pub fn disconnect_trycp_client<SV: UserValuesConstraint>(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<SV>>,
) -> HookResult {
    let client = ctx.get_mut().take_trycp_client();

    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            // The drop implementation requires launching a new task, so drop inside an async context.
            // Could also do a `runtime.enter` here but that's deliberately not exposed by the runner.
            drop(client);
            Ok(())
        })?;

    Ok(())
}

/// Calls a zome function on the cell specified in `ctx.get().cell_id()`.
///
/// You must have a valid trycp_client in your context.
pub fn call_zome<I, O, SV>(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<SV>>,
    zome_name: &str,
    fn_name: &str,
    payload: I,
    timeout: Option<Duration>,
) -> anyhow::Result<O>
where
    O: std::fmt::Debug + serde::de::DeserializeOwned,
    I: serde::Serialize + std::fmt::Debug,
    SV: UserValuesConstraint,
{
    let client = ctx.get().trycp_client();
    let app_port = ctx.get().app_port();
    let cell_id = ctx.get().cell_id();
    ctx.runner_context().executor().execute_in_place(async {
        client
            .call_zome(app_port, cell_id, zome_name, fn_name, payload, timeout)
            .await
            .map_err(|e| anyhow::anyhow!("call failure: {:?}", e))?
            .decode()
            .map_err(|e| anyhow::anyhow!("Decoding failure: {:?}", e))
    })
}

/// Call [`run`] for a scenario and check that it completed with the minimum required agents.
///
/// The value of `min_required_agents` can be overridden with the environment variable
/// `MIN_REQUIRED_AGENTS` when running a scenario.
pub fn run_with_required_agents<RV: UserValuesConstraint, V: UserValuesConstraint>(
    definition: ScenarioDefinitionBuilder<RV, V>,
    min_required_agents: usize,
) -> anyhow::Result<()> {
    let agents_at_completion = run(definition)?;

    let min_required_agents = std::env::var("MIN_REQUIRED_AGENTS")
        .inspect_err(|_| debug!("MIN_REQUIRED_AGENTS not set, using default value"))
        .ok()
        .and_then(|v| {
            v.parse()
                .inspect_err(|_| warn!("Invalid MIN_REQUIRED_AGENTS value. Using default"))
                .ok()
        })
        .unwrap_or(min_required_agents);

    if agents_at_completion < min_required_agents {
        bail!("Not enough agents ran scenario to completion: expected at least {min_required_agents}, actual {agents_at_completion}");
    }

    println!("Finished with {} agents", agents_at_completion);

    Ok(())
}
