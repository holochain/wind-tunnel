use holochain_types::prelude::*;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use trycp_wind_tunnel_runner::embed_conductor_config;
use trycp_wind_tunnel_runner::prelude::*;

embed_conductor_config!();

type OpType = String;
type ReceiptsComplete = bool;

#[derive(Debug, Default)]
pub struct ScenarioValues {
    /// Action hash to a map of validation receipt types:
    /// - if the sub map is empty, we haven't received any receipts yet,
    ///   so we're still pending
    /// - if any of the receipts_complete are false, we are still pending
    /// - if all the receipts_complete are true, we are complete
    ///   so the action should be removed from the map
    pending_actions: HashMap<ActionHash, HashMap<OpType, ReceiptsComplete>>,
}

impl ScenarioValues {
    fn mut_op_complete(&mut self, action_hash: &ActionHash, op_type: String) -> &mut bool {
        self.pending_actions
            .get_mut(action_hash)
            .unwrap()
            .entry(op_type)
            .or_default()
    }

    fn mut_any_pending(&mut self) -> bool {
        self.pending_actions.retain(|_, m| {
            if m.is_empty() {
                return true;
            }
            let mut all_complete = true;
            for c in m.values() {
                if !c {
                    all_complete = false;
                    break;
                }
            }
            !all_complete
        });

        !self.pending_actions.is_empty()
    }
}

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
    let agent_name = ctx.agent_name().to_string();

    let action_hash: ActionHash = call_zome(
        ctx,
        "crud",
        "create_sample_entry",
        "this is a test entry value",
        Some(Duration::from_secs(80)),
    )?;

    ctx.get_mut()
        .scenario_values
        .pending_actions
        .insert(action_hash, HashMap::new());

    let start = Instant::now();

    'outer: loop {
        // sleep a bit, we don't want to busy loop
        ctx.runner_context()
            .executor()
            .execute_in_place(async move {
                tokio::time::sleep(Duration::from_millis(20)).await;
                Ok(())
            })?;

        // get our list of pending actions
        let action_hash_list = ctx
            .get()
            .scenario_values
            .pending_actions
            .keys()
            .cloned()
            .collect::<Vec<_>>();

        for action_hash in action_hash_list {
            // for each action, get the validation receipts
            let response: Vec<ValidationReceiptSet> = call_zome(
                ctx,
                "crud",
                "get_sample_entry_validation_receipts",
                action_hash.clone(),
                Some(Duration::from_secs(80)),
            )?;

            for set in response.iter() {
                let cur = *ctx
                    .get_mut()
                    .scenario_values
                    .mut_op_complete(&action_hash, set.op_type.clone());

                if set.receipts_complete && !cur {
                    // if the action wasn't already complete report the time
                    // and mark it complete
                    reporter.add_custom(
                        ReportMetric::new("validation_receipts_complete_time")
                            .with_tag("op_type", set.op_type.clone())
                            .with_tag("agent_name", agent_name.clone())
                            .with_field("value", start.elapsed().as_secs_f64()),
                    );
                    *ctx.get_mut()
                        .scenario_values
                        .mut_op_complete(&action_hash, set.op_type.clone()) = true;
                }
            }
        }

        // if there are no remaining pending actions, break out of the loop
        if !ctx.get_mut().scenario_values.mut_any_pending() {
            break 'outer;
        }

        // if we were instructed to not wait for validation complete,
        // don't wait for validation complete
        if std::env::var_os("NO_VALIDATION_COMPLETE").is_some() {
            break 'outer;
        }
    }

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
    .with_default_duration_s(300)
    .add_capture_env("NO_VALIDATION_COMPLETE")
    .use_agent_setup(agent_setup)
    .use_agent_behaviour(agent_behaviour)
    .use_agent_teardown(agent_teardown);

    let agents_at_completion = run(builder)?;

    println!("Finished with {} agents", agents_at_completion);

    Ok(())
}
