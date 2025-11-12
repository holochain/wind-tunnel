use holochain_types::prelude::*;
use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;
use std::collections::HashMap;
use std::time::{Duration, Instant};

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
    pending_action: Option<(ActionHash, HashMap<OpType, ReceiptsComplete>)>,
}

impl ScenarioValues {
    /// Get a mutable reference to the receipts_complete for the given action and op type.
    ///
    /// If it returns [`None`] the action hash has completed or does not exist.
    fn mut_op_complete(&mut self, action_hash: &ActionHash, op_type: String) -> Option<&mut bool> {
        let inner = self
            .pending_action
            .get_or_insert_with(|| (action_hash.clone(), HashMap::new()));
        Some(inner.1.entry(op_type).or_default())
    }

    /// Returns whether all the actions for the given action hash are complete.
    fn is_action_complete(&self) -> bool {
        let Some((_hash, ops)) = self.pending_action.as_ref() else {
            return true;
        };

        if ops.is_empty() {
            false
        } else {
            ops.iter().all(|(_, v)| *v)
        }
    }
}

impl UserValuesConstraint for ScenarioValues {}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    start_conductor_and_configure_urls(ctx)?;
    install_app(ctx, scenario_happ_path!("crud"), &"crud".to_string())?;
    try_wait_for_min_agents(ctx, Duration::from_secs(120))?;

    Ok(())
}

fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    // check if pending action is empty, if so create a new entry
    let action_hash = if ctx.get().scenario_values.pending_action.is_none() {
        let action_hash: ActionHash = call_zome(
            ctx,
            "crud",
            "create_sample_entry",
            "this is a test entry value",
        )?;

        ctx.get_mut().scenario_values.pending_action = Some((action_hash.clone(), HashMap::new()));

        action_hash
    } else {
        ctx.get()
            .scenario_values
            .pending_action
            .as_ref()
            .map(|(hash, _)| hash)
            .cloned()
            .unwrap()
    };

    // collect validation receipts until complete
    let start = Instant::now();
    let wait_for_all = std::env::var_os("NO_VALIDATION_COMPLETE").is_none();

    wait_for_receipts_for_action(ctx, &action_hash, start, wait_for_all)?;

    // remove the action from pending if complete
    if ctx.get().scenario_values.is_action_complete() {
        ctx.get_mut().scenario_values.pending_action = None;
    }

    Ok(())
}

/// Wait for validation receipts for a specific action.
fn wait_for_receipts_for_action(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    action_hash: &ActionHash,
    start: Instant,
    wait_for_all: bool,
) -> WindTunnelResult<()> {
    let reporter = ctx.runner_context().reporter();
    let agent = ctx.get().cell_id().agent_pubkey().clone().to_string();

    let started_at = Instant::now();
    // keep checking until all receipts are complete
    while !ctx.get().scenario_values.is_action_complete() {
        let response: Vec<ValidationReceiptSet> = call_zome(
            ctx,
            "crud",
            "get_sample_entry_validation_receipts",
            action_hash.clone(),
        )?;

        if started_at.elapsed() > Duration::from_secs(60) {
            log::error!("Timed out waiting for validation receipts for action {action_hash}; last validation receipt set response: {response:#?}");
            anyhow::bail!("Timed out waiting for validation receipts for action {action_hash}");
        }

        for set in response.iter() {
            let Some(cur) = ctx
                .get_mut()
                .scenario_values
                .mut_op_complete(action_hash, set.op_type.clone())
            else {
                // already complete
                return Ok(());
            };

            // if the action wasn't already complete report the time
            // and mark it complete
            if set.receipts_complete && !*cur {
                reporter.add_custom(
                    ReportMetric::new("validation_receipts_complete_time")
                        .with_tag("op_type", set.op_type.clone())
                        .with_tag("agent", agent.clone())
                        .with_field("value", start.elapsed().as_secs_f64()),
                );
                *cur = true;
                // if we are not waiting for all, break out
                if !wait_for_all {
                    break;
                }
            }
        }

        // sleep a bit before checking again
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    log::info!("All required validations received for {action_hash}");

    Ok(())
}

fn agent_teardown(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    uninstall_app(ctx, None)
}

fn main() -> WindTunnelResult<()> {
    let builder = ScenarioDefinitionBuilder::<
        HolochainRunnerContext,
        HolochainAgentContext<ScenarioValues>,
    >::new_with_init(env!("CARGO_PKG_NAME"))
    .with_default_duration_s(300)
    .add_capture_env("NO_VALIDATION_COMPLETE")
    .use_build_info(conductor_build_info)
    .use_agent_setup(agent_setup)
    .use_agent_behaviour(agent_behaviour)
    .use_agent_teardown(agent_teardown);

    run(builder)?;

    Ok(())
}
