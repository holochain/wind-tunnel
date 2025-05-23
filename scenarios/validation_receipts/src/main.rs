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

fn setup(ctx: &mut RunnerContext<HolochainRunnerContext>) -> HookResult {
    configure_app_ws_url(ctx)?;
    Ok(())
}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    install_app(ctx, scenario_happ_path!("crud"), &"crud".to_string())?;
    try_wait_for_min_agents(ctx, Duration::from_secs(120))?;

    Ok(())
}

fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let reporter = ctx.runner_context().reporter();
    let agent = ctx.get().cell_id().agent_pubkey().clone().to_string();

    let action_hash: ActionHash = call_zome(
        ctx,
        "crud",
        "create_sample_entry",
        "this is a test entry value",
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
                            .with_tag("agent", agent.clone())
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
    .use_setup(setup)
    .use_agent_setup(agent_setup)
    .use_agent_behaviour(agent_behaviour)
    .use_agent_teardown(agent_teardown);

    run(builder)?;

    Ok(())
}
