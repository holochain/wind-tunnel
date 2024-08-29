use holochain_types::prelude::*;
use std::time::{Duration, Instant};
use trycp_wind_tunnel_runner::embed_conductor_config;
use trycp_wind_tunnel_runner::prelude::*;

embed_conductor_config!();

#[derive(Debug, Default)]
pub struct ScenarioValues {}

impl UserValuesConstraint for ScenarioValues {}

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

    install_app(ctx, scenario_happ_path!("crud"), &"crud".to_string())?;
    try_wait_for_min_peers(ctx, Duration::from_secs(120))?;

    Ok(())
}

fn agent_behaviour(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<ScenarioValues>>,
) -> HookResult {
    let reporter = ctx.runner_context().reporter();

    let action_hash: ActionHash = call_zome(
        ctx,
        "crud",
        "create_sample_entry",
        "this is a test entry value",
        Some(Duration::from_secs(80)),
    )?;

    let response: Option<Record> = call_zome(
        ctx,
        "crud",
        "get_sample_entry",
        action_hash.clone(),
        Some(Duration::from_secs(80)),
    )?;

    assert!(response.is_some(), "Expected record to be found");

    let start = Instant::now();

    'outer: loop {
        let response: Vec<ValidationReceiptSet> = call_zome(
            ctx,
            "crud",
            "get_sample_entry_validation_receipts",
            action_hash.clone(),
            Some(Duration::from_secs(80)),
        )?;

        // only check for complete if we actually get data back
        // before any receipts are received, this is an empty vec
        if !response.is_empty() {
            let mut all_complete = true;

            'inner: for set in response.iter() {
                if !set.receipts_complete {
                    all_complete = false;
                    break 'inner;
                }
            }

            if all_complete {
                break 'outer;
            }
        }
    }

    reporter.add_custom(
        ReportMetric::new("validation_receipts_complete_time")
            .with_field("value", start.elapsed().as_secs_f64()),
    );

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
