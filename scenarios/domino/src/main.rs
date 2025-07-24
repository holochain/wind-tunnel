mod behaviour;
mod handle_agent_setup;
mod handle_scenario_setup;
use handle_scenario_setup::ScenarioValues;
use holochain_wind_tunnel_runner::prelude::*;
mod domino_agent;
use domino_agent::DominoAgentExt;

fn main() -> WindTunnelResult<()> {
    log::info!("Starting domino scenario");
    let builder = ScenarioDefinitionBuilder::<
        HolochainRunnerContext,
        HolochainAgentContext<ScenarioValues>,
    >::new_with_init(env!("CARGO_PKG_NAME"))
    .use_setup(handle_scenario_setup::setup)
    .use_agent_setup(handle_agent_setup::agent_setup)
    .use_named_agent_behaviour("initiate", behaviour::initiate_network::agent_behaviour)
    .use_named_agent_behaviour("spend", behaviour::spend::agent_behaviour)
    .use_agent_teardown(|ctx| {
        // publish final ledger state
        let ledger = ctx.domino_get_ledger()?;
        let reporter = ctx.runner_context().reporter();
        reporter.add_custom(
            ReportMetric::new("final:ledger_state")
                .with_field("ledger", format!("{:?}", ledger))
                .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string()),
        );
        let actuable_tx = ctx.domino_get_actionable_transactions()?;
        reporter.add_custom(
            ReportMetric::new("final:actionable_transactions")
                .with_field(
                    "invoices",
                    format!("{:?}", actuable_tx.invoice_actionable.len()),
                )
                .with_field(
                    "spends",
                    format!("{:?}", actuable_tx.spend_actionable.len()),
                )
                .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string()),
        );
        // log::info!("Actionable transactions: {:?}", actuable_tx);
        let completed_tx = ctx.domino_get_completed_transactions()?;
        reporter.add_custom(
            ReportMetric::new("final:completed_transactions")
                .with_field("accepts", format!("{:?}", completed_tx.accept.len()))
                .with_field("spends", format!("{:?}", completed_tx.spend.len()))
                .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string()),
        );
        uninstall_app(ctx, None).ok();
        Ok(())
    });

    run(builder)?;

    Ok(())
}
