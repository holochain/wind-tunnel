mod behaviour;
mod durable_object;
mod handle_agent_setup;
mod unyt_agent;

use holochain_types::prelude::*;
use holochain_wind_tunnel_runner::prelude::*;
use rave_engine::types::{Actionable, History, Pagination, TransactionType};
use tokio::time::Instant;
use unyt_agent::UnytAgentExt;

#[derive(Debug, Default)]
pub struct ScenarioValues {
    pub session_start_time: Option<Instant>,
    pub network_initialized: bool,
    pub participating_agents: Vec<AgentPubKeyB64>,
    pub executor_pubkey: Option<AgentPubKeyB64>,
    pub smart_agreement_hash: Option<ActionHashB64>,
    pub progenitor_agent_pubkey: Option<AgentPubKeyB64>,
}

impl UserValuesConstraint for ScenarioValues {}

fn main() -> WindTunnelResult<()> {
    log::info!("Starting Unyt Chain Transaction scenario");
    let builder = ScenarioDefinitionBuilder::<
        HolochainRunnerContext,
        HolochainAgentContext<ScenarioValues>,
    >::new_with_init(env!("CARGO_PKG_NAME"))
    .add_capture_env("UNYT_NUMBER_OF_LINKS_TO_PROCESS")
    .use_agent_setup(handle_agent_setup::agent_setup)
    .use_named_agent_behaviour("initiate", behaviour::initiate_network::agent_behaviour)
    .use_named_agent_behaviour("spend", behaviour::spend::agent_behaviour)
    .use_named_agent_behaviour(
        "smart_agreements",
        behaviour::smart_agreements::agent_behaviour,
    )
    .use_agent_teardown(|ctx| {
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
        while let Ok(History {
            items,
            low_boundary,
            end_of_chain,
        }) = ctx.unyt_get_history(Pagination {
            high_boundary: current_boundary,
            per_page: 100,
        }) {
            current_boundary = Some(low_boundary);

            // TODO: Confirm what metrics we want as part of https://github.com/holochain/wind-tunnel/issues/463
            let mut accepts = 0u64;
            let mut commitments = 0u64;
            let mut parked_spend = 0u64;

            items.iter().for_each(|item| match item.tx_type {
                TransactionType::Commitment => commitments = commitments.saturating_add(1),
                TransactionType::Accept => accepts = accepts.saturating_add(1),
                TransactionType::ParkedSpend => parked_spend = parked_spend.saturating_add(1),
                _ => (),
            });

            reporter.add_custom(
                ReportMetric::new("completed_transactions")
                    .with_field("accepts", accepts)
                    .with_field("spends", commitments)
                    .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string()),
            );
            reporter.add_custom(
                ReportMetric::new("parked_spends")
                    .with_field("parked_spends", parked_spend)
                    .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string()),
            );
            if end_of_chain {
                break;
            }
        }

        log::info!("uninstalling agent {}", ctx.get().cell_id().agent_pubkey());
        uninstall_app(ctx, None).ok();
        log::info!(
            "done tearing down agent {}",
            ctx.get().cell_id().agent_pubkey()
        );

        Ok(())
    });

    run(builder)?;

    Ok(())
}
