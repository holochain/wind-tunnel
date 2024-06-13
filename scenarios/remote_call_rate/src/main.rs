use anyhow::Context;
use holochain_types::app::{AppBundleSource, InstallAppPayload};
use holochain_types::prelude::{AgentPubKey, AppBundle, CellId, ExternIO};
use holochain_types::websocket::AllowedOrigins;
use remote_call_integrity::TimedResponse;
use std::time::Instant;
use trycp_wind_tunnel_runner::prelude::*;

const CONDUCTOR_CONFIG: &str = include_str!("../../../conductor-config.yaml");

#[derive(Debug, Default)]
pub struct ScenarioValues {
    app_port: u16,
    cell_id: Option<CellId>,
    remote_call_peers: Vec<AgentPubKey>,
}

impl UserValuesConstraint for ScenarioValues {}

fn agent_setup(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<ScenarioValues>>,
) -> HookResult {
    connect_trycp_client(ctx)?;
    reset_trycp_remote(ctx)?;

    let run_id = ctx.runner_context().get_run_id().to_string();
    let client = ctx.get().trycp_client();
    let agent_id = ctx.agent_id().to_string();

    let (app_port, cell_id, credentials) =
        ctx.runner_context()
            .executor()
            .execute_in_place(async move {
                client
                    .configure_player(agent_id.clone(), CONDUCTOR_CONFIG.to_string(), None)
                    .await?;

                client
                    .startup(agent_id.clone(), Some("info".to_string()), None)
                    .await?;

                let agent_key = client
                    .generate_agent_pub_key(agent_id.clone(), None)
                    .await?;

                let path = scenario_happ_path!("remote_call");
                let content = tokio::fs::read(path).await?;

                let app_info = client
                    .install_app(
                        agent_id.clone(),
                        InstallAppPayload {
                            source: AppBundleSource::Bundle(AppBundle::decode(&content)?),
                            agent_key,
                            installed_app_id: Some("remote_call".into()),
                            membrane_proofs: Default::default(),
                            network_seed: Some(run_id),
                        },
                        None,
                    )
                    .await?;

                let enable_result = client
                    .enable_app(agent_id.clone(), app_info.installed_app_id.clone(), None)
                    .await?;
                if !enable_result.errors.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Failed to enable app: {:?}",
                        enable_result.errors
                    ));
                }

                let start_discovery = Instant::now();
                for _ in 0..60 {
                    let agent_list = client.agent_info(agent_id.clone(), None, None).await?;

                    // TODO Configure how many peers are required before starting
                    if agent_list.len() > 1 {
                        break;
                    }

                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }

                println!(
                    "Discovery for agent {} took: {}s",
                    agent_id,
                    start_discovery.elapsed().as_secs()
                );

                let app_port = client
                    .attach_app_interface(agent_id.clone(), None, AllowedOrigins::Any, None, None)
                    .await?;

                let issued = client
                    .issue_app_auth_token(
                        agent_id.clone(),
                        IssueAppAuthenticationTokenPayload {
                            installed_app_id: "remote_call".to_string(),
                            expiry_seconds: 30,
                            single_use: true,
                        },
                        None,
                    )
                    .await?;

                client
                    .connect_app_interface(issued.token, app_port, None)
                    .await?;

                let cell_id = match app_info.cell_info.values().next().unwrap().first().unwrap() {
                    CellInfo::Provisioned(pc) => pc.cell_id.clone(),
                    _ => panic!("Could not find cell id in app info: {app_info:?}"),
                };

                let credentials = client
                    .authorize_signing_credentials(
                        agent_id.clone(),
                        AuthorizeSigningCredentialsPayload {
                            cell_id: cell_id.clone(),
                            functions: None, // Equivalent to all functions
                        },
                        None,
                    )
                    .await?;

                Ok((app_port, cell_id, credentials))
            })?;

    ctx.get_mut().scenario_values.app_port = app_port;
    ctx.get_mut()
        .signer()
        .add_credentials(cell_id.clone(), credentials);
    ctx.get_mut().scenario_values.cell_id = Some(cell_id);

    Ok(())
}

fn agent_behaviour(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<ScenarioValues>>,
) -> HookResult {
    let client = ctx.get().trycp_client();

    let agent_id = ctx.agent_id().to_string();
    let app_port = ctx.get().scenario_values.app_port;
    let cell_id = ctx.get().scenario_values.cell_id.clone().unwrap();
    let next_remote_call_peer = ctx.get_mut().scenario_values.remote_call_peers.pop();
    let reporter = ctx.runner_context().reporter();

    let new_peers = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move {
            match next_remote_call_peer {
                None => {
                    // No more agents available to call, get a new list.
                    // This is also the initial condition.
                    Ok(client
                        .agent_info(agent_id, None, None)
                        .await?
                        .into_iter()
                        .map(|info| AgentPubKey::from_raw_36(info.agent.0.clone()))
                        .filter(|k| k != cell_id.agent_pubkey()) // Don't call ourselves!
                        .collect::<Vec<_>>())
                }
                Some(agent_pub_key) => {
                    // Send a remote call to this agent
                    let start = Instant::now();
                    let response = client
                        .call_zome(
                            app_port,
                            cell_id,
                            "remote_call",
                            "call_echo_timestamp",
                            ExternIO::encode(agent_pub_key).context("Encoding failure")?,
                            None,
                        )
                        .await?;
                    let round_trip_time_s = start.elapsed();

                    let response: TimedResponse = response
                        .decode()
                        .map_err(|e| anyhow::anyhow!("Decoding failure: {:?}", e))?;

                    let dispatch_time_s = response.request_value.as_micros() as f64 / 1_000_000.0;
                    let receive_time_s = response.value.as_micros() as f64 / 1_000_000.0;

                    reporter.add_custom(
                        ReportMetric::new("remote_call_dispatch")
                            .with_field("value", receive_time_s - dispatch_time_s),
                    );
                    reporter.add_custom(
                        ReportMetric::new("remote_call_round_trip")
                            .with_field("value", round_trip_time_s.as_secs_f64()),
                    );

                    // Add no new agents, that should only happen when we exhaust the list.
                    Ok(Vec::with_capacity(0))
                }
            }
        })?;

    ctx.get_mut()
        .scenario_values
        .remote_call_peers
        .extend(new_peers);

    Ok(())
}

fn agent_teardown(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<ScenarioValues>>,
) -> HookResult {
    let client = ctx.get().trycp_client();
    let agent_id = ctx.agent_id().to_string();
    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            client.shutdown(agent_id, None, None).await?;
            Ok(())
        })?;

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

    run(builder)?;

    Ok(())
}
