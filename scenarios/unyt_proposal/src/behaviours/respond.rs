use crate::behaviours::common::{self, ProposalWeights};
use crate::values::ScenarioValues;
use holochain_wind_tunnel_runner::prelude::{
    AgentContext, HolochainAgentContext, HolochainRunnerContext, HookResult,
};
use std::thread;
use std::time::Duration;
use wind_tunnel_unyt_scenario::unyt_agent::UnytAgentExt as _;

pub fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    // step 1 - wait for network init
    if !common::is_network_initialized(ctx)? {
        return Ok(());
    }

    // step 2 - handle incoming transactions
    {
        let actionable_transactions = match ctx.unyt_get_actionable_transactions() {
            Ok(txs) => txs,
            Err(err) => {
                log::warn!("Failed to get actionable transactions (transient DHT issue): {err}");
                thread::sleep(Duration::from_secs(1));
                return Ok(());
            }
        };

        // measure sync lag for all newly seen transactions
        common::measure_sync_lag(ctx, &actionable_transactions);

        // for each reject actionable, call `create_reclaim_balance`
        common::create_reclaim_balance(ctx, actionable_transactions.reject_actionable);
        // for each accept actionable, call `create_receipt_for_accept`
        common::create_receipt_for_accept(ctx, actionable_transactions.accept_actionable);
        // handle incoming proposals
        let weights = ProposalWeights::get_responder_weights_from_env()?;
        common::handle_proposals(ctx, actionable_transactions.proposal_actionable, &weights)?;
        // handle incoming commitments
        common::handle_commitments(ctx, actionable_transactions.commitment_actionable)?;
    }

    // step 3 - check ledger
    let _spendable_amount = common::get_spendable_amount(ctx)?;

    // step 4 - poll transaction status
    common::poll_watched_transactions(ctx);

    // step 5 - get history
    common::poll_history(ctx);

    thread::sleep(Duration::from_secs(1));

    Ok(())
}
