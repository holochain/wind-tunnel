use crate::UnytScenarioValues;
use crate::unyt_agent::UnytAgentExt;
use holochain_wind_tunnel_runner::prelude::*;
use rave_engine::types::{Actionable, History, Pagination, TransactionType};

/// Shared agent teardown that reports final ledger state, actionable
/// transactions and completed transaction history, then uninstalls the app.
pub fn agent_teardown<SV: UnytScenarioValues>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
) -> HookResult {
    // publish final ledger state
    log::info!("Tearing down agent {}", ctx.get().cell_id().agent_pubkey());
    let reporter = ctx.runner_context().reporter();
    if let Ok(ledger) = ctx.unyt_get_ledger() {
        reporter.add_custom(
            ReportMetric::new("ledger_state")
                .with_field("ledger_balance", ledger.balance.get_base_unyt().to_string())
                .with_field("ledger_fees", ledger.fees_owed.to_string())
                .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string()),
        );
    };
    let actionable_tx = ctx
        .unyt_get_actionable_transactions()
        .unwrap_or(Actionable {
            proposal_actionable: vec![],
            commitment_actionable: vec![],
            accept_actionable: vec![],
            reject_actionable: vec![],
        });

    reporter.add_custom(
        ReportMetric::new("actionable_transactions")
            .with_field("proposals", actionable_tx.proposal_actionable.len() as u64)
            .with_field(
                "commitments",
                actionable_tx.commitment_actionable.len() as u64,
            )
            .with_field("accepts", actionable_tx.accept_actionable.len() as u64)
            .with_field("rejects", actionable_tx.reject_actionable.len() as u64)
            .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string()),
    );

    let mut current_boundary = None;
    let mut accepts = 0u64;
    let mut commitments = 0u64;
    let mut parked_spend = 0u64;
    let mut raves = 0u64;

    while let Ok(History {
        items,
        low_boundary,
        end_of_chain,
    }) = ctx.unyt_get_history(Pagination {
        high_boundary: current_boundary,
        per_page: 100,
    }) {
        current_boundary = Some(low_boundary);

        items.iter().for_each(|item| match item.tx_type {
            TransactionType::Commitment => commitments = commitments.saturating_add(1),
            TransactionType::Accept => accepts = accepts.saturating_add(1),
            TransactionType::ParkedSpend => parked_spend = parked_spend.saturating_add(1),
            TransactionType::RAVE => raves = raves.saturating_add(1),
            _ => (),
        });

        if end_of_chain {
            break;
        }
    }

    reporter.add_custom(
        ReportMetric::new("completed_transactions")
            .with_field("accepts", accepts)
            .with_field("spends", commitments)
            .with_field("raves", raves)
            .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string()),
    );
    reporter.add_custom(
        ReportMetric::new("parked_spends")
            .with_field("parked_spends", parked_spend)
            .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string()),
    );

    log::info!("uninstalling agent {}", ctx.get().cell_id().agent_pubkey());
    uninstall_app(ctx, None)?;
    log::info!(
        "done tearing down agent {}",
        ctx.get().cell_id().agent_pubkey()
    );

    Ok(())
}
