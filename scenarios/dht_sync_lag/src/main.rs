use std::collections::HashSet;
use holochain_types::prelude::{ActionHash, Record, Timestamp};
use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;
use std::path::Path;
use std::time::SystemTime;
use anyhow::anyhow;
use timed_integrity::TimedEntry;

#[derive(Debug, Default)]
struct ScenarioValues {
    sent_actions: u32,
    seen_actions: HashSet<ActionHash>,
}

impl UserValuesConstraint for ScenarioValues {}

fn setup(ctx: &mut RunnerContext<HolochainRunnerContext>) -> HookResult {
    configure_app_ws_url(ctx)?;
    Ok(())
}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    install_app(ctx, scenario_happ_path!("timed"), &"timed".to_string())?;

    Ok(())
}

fn agent_behaviour_write(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let _: ActionHash = call_zome(
        ctx,
        "timed",
        "created_timed_entry",
        TimedEntry {
            created_at: Timestamp::now(),
        },
    )?;

    ctx.get_mut().scenario_values.sent_actions += 1;

    let metric = ReportMetric::new("dht_sync_sent_count")
        .with_field("value", ctx.get().scenario_values.sent_actions);
    ctx.runner_context().reporter().clone().add_custom(metric);

    Ok(())
}

fn agent_behaviour_record_lag(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let found: Vec<Record> = call_zome(ctx, "timed", "get_timed_entries_local", ())?;

    let found = found.into_iter().filter(|r| !ctx.get().scenario_values.seen_actions.contains(r.action_address())).collect::<Vec<_>>();

    let reporter_handle = ctx.runner_context().reporter().clone();
    for new_record in &found {
        let timed_entry: TimedEntry = new_record.entry().to_app_option().map_err(|e| anyhow!("Failed to deserialize TimedEntry: {}", e))?.unwrap();

        let metric = ReportMetric::new("dht_sync_lag");
        let lag_ms = (metric.timestamp.clone().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_micros() - timed_entry.created_at.as_micros() as u128) as f64 / 1000.0;
        let metric = metric
            .with_field("value", lag_ms);

        reporter_handle.add_custom(metric);

        ctx.get_mut().scenario_values.seen_actions.insert(new_record.action_address().clone());
    }

    let metric = ReportMetric::new("dht_sync_recv_count")
        .with_field("value", ctx.get().scenario_values.seen_actions.len() as f64);
    reporter_handle.add_custom(metric);

    Ok(())
}

fn main() -> WindTunnelResult<()> {
    let builder = ScenarioDefinitionBuilder::<
        HolochainRunnerContext,
        HolochainAgentContext<ScenarioValues>,
    >::new_with_init(env!("CARGO_PKG_NAME"))
    .with_default_duration_s(60)
    .use_setup(setup)
    .use_agent_setup(agent_setup)
    .use_named_agent_behaviour("write", agent_behaviour_write)
    .use_named_agent_behaviour("record_lag", agent_behaviour_record_lag);

    run(builder)?;

    Ok(())
}
