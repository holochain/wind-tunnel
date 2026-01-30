mod behaviour;
mod handle_agent_setup;
mod handle_scenario_setup;
use handle_scenario_setup::ScenarioValues;
use holochain_wind_tunnel_runner::prelude::*;
mod unyt_agent;
use rave_engine::types::{Actionable, Completed};
use unyt_agent::UnytAgentExt;
mod durable_object;

fn main() -> WindTunnelResult<()> {
    log::info!("Starting unyt scenario");
    let builder = ScenarioDefinitionBuilder::<
        HolochainRunnerContext,
        HolochainAgentContext<ScenarioValues>,
    >::new_with_init(env!("CARGO_PKG_NAME"))
    .use_setup(handle_scenario_setup::setup)
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
                .with_field("ledger_fees", ledger.fees.to_string())
                .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string()),
        );
        let actuable_tx = ctx
            .unyt_get_actionable_transactions()
            .unwrap_or(Actionable {
                invoice_actionable: vec![],
                spend_actionable: vec![],
            });
        let completed_tx = ctx.unyt_get_completed_transactions().unwrap_or(Completed {
            accept: vec![],
            spend: vec![],
        });

        let parked_spend = ctx.unyt_get_parked_spend().unwrap_or(vec![]);
        let executed_agreements = ctx.unyt_get_all_my_executed_saveds().unwrap_or(vec![]);
        reporter.add_custom(
            ReportMetric::new("actionable_transactions")
                .with_field("invoices", actuable_tx.invoice_actionable.len() as u64)
                .with_field("spends", actuable_tx.spend_actionable.len() as u64)
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
