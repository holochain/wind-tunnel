use crate::values::ScenarioValues;
use holochain_wind_tunnel_runner::prelude::*;
use rave_engine::types::AcceptInput;
use wind_tunnel_unyt_scenario::unyt_agent::UnytAgentExt;

/// Swap agent to exchange wHOT -> HF. It accepts every incoming commitment,
/// paying out HF against the network's global credit limit.
pub fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let reporter = ctx.runner_context().reporter();
    let agent_key = ctx.get().cell_id().agent_pubkey().to_string();

    let actionable = match ctx.unyt_get_actionable_transactions() {
        Ok(actionable) => actionable,
        Err(err) => {
            log::warn!("[swap_agent] get_actionable_transactions failed: {err}");
            return Ok(());
        }
    };

    // Accept every incoming swap commitment.
    let mut accepted = 0u64;
    for commitment in actionable.commitment_actionable {
        match ctx.unyt_create_accept(AcceptInput {
            commitment: commitment.id.clone(),
            note: None,
        }) {
            Ok(_) => accepted += 1,
            Err(err) => log::warn!(
                "[swap_agent] failed to accept commitment {}: {err}",
                commitment.id
            ),
        }
    }
    log::info!("[swap_agent] accepted {accepted} commitments");
    let cumulative = {
        let values = &mut ctx.get_mut().scenario_values;
        values.commitments_accepted = values.commitments_accepted.saturating_add(accepted);
        values.commitments_accepted
    };
    reporter.add_custom(
        ReportMetric::new("commitments_accepted")
            .with_tag("agent", agent_key)
            .with_field("value", cumulative),
    );

    Ok(())
}
