use crate::{ArcType, UnytScenarioValues};
use holochain_wind_tunnel_runner::prelude::{
    AgentContext, HolochainAgentContext, HolochainRunnerContext, ReportMetric,
};
use rave_engine::types::Transaction;
use std::time::SystemTime;

pub fn record_sync_lag<SV: UnytScenarioValues>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    arc_type: &ArcType,
    actionables: &[Transaction],
    tx_type: &'static str,
) {
    let reporter = ctx.runner_context().reporter();
    let now_us = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_micros();
    let agent_key = ctx.get().cell_id().agent_pubkey().to_string();
    let unseen_txs: Vec<_> = actionables
        .iter()
        .filter(|tx| {
            !ctx.get()
                .scenario_values
                .seen_transactions()
                .contains(&(tx.id.clone(), tx_type))
        })
        .collect();
    for tx in unseen_txs {
        let published_at_us = tx.timestamp.as_micros() as u128;
        let lag_s = now_us.saturating_sub(published_at_us) as f64 / 1e6;
        reporter.add_custom(
            ReportMetric::new("sync_lag")
                .with_tag("tx_type", tx_type.to_owned())
                .with_tag("agent", agent_key.clone())
                .with_tag("arc", arc_type.as_tag())
                .with_field("value", lag_s),
        );
        ctx.get_mut()
            .scenario_values
            .seen_transactions_mut()
            .insert((tx.id.clone(), tx_type));
    }
}
