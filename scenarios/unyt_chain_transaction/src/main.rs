mod behaviour;
mod durable_object;
mod handle_agent_setup;
mod unyt_agent;

use holochain_types::prelude::*;
use holochain_wind_tunnel_runner::prelude::*;
use rave_engine::types::{Actionable, Completed};
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
        let ledger = ctx.unyt_get_ledger()?;
        let reporter = ctx.runner_context().reporter();
        reporter.add_custom(
            ReportMetric::new("ledger_state")
                .with_field("ledger_balance", ledger.balance.get_base_unyt().to_string())
                .with_field("ledger_fees", ledger.fees_owed.to_string())
                .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string()),
        );
        let actuable_tx = ctx
            .unyt_get_actionable_transactions()
            .unwrap_or(Actionable {
                proposal_actionable: vec![],
                commitment_actionable: vec![],
                accept_actionable: vec![],
                reject_actionable: vec![],
            });
        let completed_tx = ctx.unyt_get_completed_transactions().unwrap_or(Completed {
            accept: vec![],
            spend: vec![],
        });

        let parked_spend = ctx.unyt_get_parked_spend().unwrap_or(vec![]);
        let executed_agreements = ctx.unyt_get_all_my_executed_saveds().unwrap_or(vec![]);
        reporter.add_custom(
            ReportMetric::new("actionable_transactions")
                .with_field("proposals", actuable_tx.proposal_actionable.len() as u64)
                .with_field(
                    "commitments",
                    actuable_tx.commitment_actionable.len() as u64,
                )
                .with_field("accepts", actuable_tx.accept_actionable.len() as u64)
                .with_field("rejects", actuable_tx.reject_actionable.len() as u64)
                .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string()),
        );
        reporter.add_custom(
            ReportMetric::new("completed_transactions")
                .with_field("accepts", completed_tx.accept.len() as u64)
                .with_field("spends", completed_tx.spend.len() as u64)
                .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string()),
        );
        reporter.add_custom(
            ReportMetric::new("parked_spends")
                .with_field("parked_spends", parked_spend.len() as u64)
                .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string()),
        );
        reporter.add_custom(
            ReportMetric::new("executed_agreements")
                .with_field("number", executed_agreements.len() as u64)
                .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string()),
        );
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
