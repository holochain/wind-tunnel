use holochain_types::prelude::*;
use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;
use std::collections::HashMap;
use std::time::{Duration, Instant};

type OpType = String;
type ReceiptsComplete = bool;

/// Represents a pending action and its validation receipt status.
#[derive(Debug)]
struct PendingAction {
    /// The action hash of the pending action.
    action_hash: ActionHash,
    /// A map of operation types to whether their receipts are complete.
    /// - if the map is empty, we haven't received any receipts yet,
    //  so we're still pending
    //  - if any of the receipts_complete are false, we are still pending
    //  - if all the receipts_complete are true, we are complete
    receipts_complete: HashMap<OpType, ReceiptsComplete>,
    /// The time the action was created.
    created_at: Instant,
}

impl PendingAction {
    /// Create a new [`PendingAction`] for the given action hash.
    pub fn new(action_hash: ActionHash) -> Self {
        Self {
            action_hash,
            receipts_complete: HashMap::new(),
            created_at: Instant::now(),
        }
    }
}

#[derive(Debug, Default)]
pub struct ScenarioValues {
    /// The [`PendingAction`] being tracked for validation receipts.
    pending_action: Option<PendingAction>,
}

impl ScenarioValues {
    /// Get a mutable reference to the receipts_complete for the op type.
    fn mut_op_complete(&mut self, action_hash: &ActionHash, op_type: String) -> &mut bool {
        let inner = self
            .pending_action
            .get_or_insert_with(|| PendingAction::new(action_hash.clone()));
        inner.receipts_complete.entry(op_type).or_default()
    }

    /// Returns whether all the operations for the given action hash are complete.
    ///
    /// If there is no pending op, returns true.
    fn is_action_complete(&self) -> bool {
        let Some(PendingAction {
            receipts_complete, ..
        }) = self.pending_action.as_ref()
        else {
            return true;
        };

        if receipts_complete.is_empty() {
            false
        } else {
            receipts_complete.iter().all(|(_, v)| *v)
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
    if ctx.get().scenario_values.pending_action.is_none() {
        let action_hash: ActionHash = call_zome(
            ctx,
            "crud",
            "create_sample_entry",
            "this is a test entry value",
        )?;

        ctx.get_mut().scenario_values.pending_action = Some(PendingAction::new(action_hash));
    };

    // collect validation receipts until complete
    let wait_for_all = std::env::var_os("NO_VALIDATION_COMPLETE").is_none();

    wait_for_receipts_for_action(ctx, wait_for_all)?;

    Ok(())
}

/// Wait until validation receipts for the given action hash are complete.
///
/// If `wait_for_all` is true, will wait until all receipt types are complete.
/// If false, will return as soon as any receipt type is complete.
///
/// If the validation receipts are marked complete, will report the time taken to the reporter, and
/// set the `pending_action` to [`None`].
fn wait_for_receipts_for_action(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    wait_for_all: bool,
) -> WindTunnelResult<()> {
    let reporter = ctx.runner_context().reporter();
    let agent = ctx.get().cell_id().agent_pubkey().clone().to_string();

    // get pending action hash and created at
    let (action_hash, created_at) = ctx
        .get()
        .scenario_values
        .pending_action
        .as_ref()
        .map(
            |PendingAction {
                 action_hash,
                 created_at,
                 ..
             }| (action_hash.clone(), *created_at),
        )
        .ok_or_else(|| anyhow::anyhow!("No pending action to get receipts for"))?;

    loop {
        let response: Vec<ValidationReceiptSet> = call_zome(
            ctx,
            "crud",
            "get_sample_entry_validation_receipts",
            action_hash.clone(),
        )?;

        for set in response.iter() {
            let op_complete = ctx
                .get_mut()
                .scenario_values
                .mut_op_complete(&action_hash, set.op_type.clone());

            // if the action wasn't already complete report the time
            // and mark it complete
            if set.receipts_complete && !*op_complete {
                reporter.add_custom(
                    ReportMetric::new("validation_receipts_complete_time")
                        .with_tag("op_type", set.op_type.clone())
                        .with_tag("agent", agent.clone())
                        .with_field("value", created_at.elapsed().as_secs_f64()),
                );
                *op_complete = true;
                // if we are not waiting for all, break out
                if !wait_for_all {
                    log::info!(
                        "All required validations received for {action_hash} (short-circuit)"
                    );
                    break;
                }
            }
        }

        if ctx.get().scenario_values.is_action_complete() {
            break;
        }
        // not complete yet, will try again next tick
        log::debug!("Validation receipts not yet complete for {action_hash}; current receipt set response: {response:?}");
        std::thread::sleep(std::time::Duration::from_secs(5));
    }

    log::info!("All required validations received for {action_hash}");
    // mark the action as complete
    ctx.get_mut().scenario_values.pending_action = None;

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
