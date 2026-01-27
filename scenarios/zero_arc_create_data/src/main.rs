use anyhow::anyhow;
use holochain_types::prelude::Record;
use holochain_types::prelude::{ActionHash, Timestamp};
use holochain_wind_tunnel_runner::happ_path;
use holochain_wind_tunnel_runner::prelude::*;
use std::collections::HashSet;
use std::time::SystemTime;
use timed_integrity::TimedEntry;

#[derive(Debug, Default)]
struct ScenarioValues {
    sent_actions_count: u32,
    seen_actions: HashSet<ActionHash>,
}

impl UserValuesConstraint for ScenarioValues {}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    if ctx.assigned_behaviour() == "zero" {
        ctx.get_mut()
            .holochain_config_mut()
            .with_target_arc_factor(0);
    }
    start_conductor_and_configure_urls(ctx)?;
    install_app(ctx, happ_path!("timed"), &"timed".to_string())?;

    if ctx.assigned_behaviour() == "zero" {
        try_wait_until_full_arc_peer_discovered(ctx)?;
    }
    Ok(())
}

fn agent_behaviour_zero(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    // Report the number of open connections
    let app_client = ctx.get().app_client();

    let network_stats = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move { Ok(app_client.dump_network_stats().await?) })?;

    let metric = ReportMetric::new("zero_arc_create_data_open_connections")
        .with_tag("arc", "zero")
        .with_field("value", network_stats.connections.len() as u32);
    ctx.runner_context().reporter().clone().add_custom(metric);

    let _: ActionHash = call_zome(
        ctx,
        "timed",
        "created_timed_entry",
        TimedEntry {
            created_at: Timestamp::now(),
        },
    )?;

    ctx.get_mut().scenario_values.sent_actions_count += 1;

    // Report number of timed entries created
    let agent_pub_key = ctx.get().cell_id().agent_pubkey().to_string();
    let metric = ReportMetric::new("zero_arc_create_data_entry_created_count")
        .with_tag("agent", agent_pub_key)
        .with_tag("arc", "zero")
        .with_field("value", ctx.get().scenario_values.sent_actions_count);
    ctx.runner_context().reporter().clone().add_custom(metric);

    Ok(())
}

fn agent_behaviour_full(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let found: Vec<Record> = call_zome(ctx, "timed", "get_timed_entries_local", ())?;

    let found = found
        .into_iter()
        .filter(|r| {
            !ctx.get()
                .scenario_values
                .seen_actions
                .contains(r.action_address())
        })
        .collect::<Vec<_>>();

    let reporter_handle = ctx.runner_context().reporter().clone();
    let agent_pub_key = ctx.get().cell_id().agent_pubkey().to_string();
    for new_record in &found {
        let timed_entry: TimedEntry = new_record
            .entry()
            .to_app_option()
            .map_err(|e| anyhow!("Failed to deserialize TimedEntry: {}", e))?
            .unwrap();

        let metric = ReportMetric::new("zero_arc_create_data_sync_lag");
        let lag_s = (metric
            .timestamp
            .clone()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_micros()
            - timed_entry.created_at.as_micros() as u128) as f64
            / 1e6;
        let metric = metric
            .with_tag("agent", agent_pub_key.clone())
            .with_field("value", lag_s);

        reporter_handle.add_custom(metric);

        ctx.get_mut()
            .scenario_values
            .seen_actions
            .insert(new_record.action_address().clone());
    }

    let metric = ReportMetric::new("zero_arc_create_data_recv_count")
        .with_tag("agent", agent_pub_key)
        .with_field("value", ctx.get().scenario_values.seen_actions.len() as f64);
    reporter_handle.add_custom(metric);

    // Report the number of open connections
    let app_client = ctx.get().app_client();
    let network_stats = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move { Ok(app_client.dump_network_stats().await?) })?;

    let metric = ReportMetric::new("zero_arc_create_data_open_connections")
        .with_tag("arc", "full")
        .with_field("value", network_stats.connections.len() as u32);
    ctx.runner_context().reporter().clone().add_custom(metric);

    Ok(())
}

fn main() -> WindTunnelResult<()> {
    let builder = ScenarioDefinitionBuilder::<
        HolochainRunnerContext,
        HolochainAgentContext<ScenarioValues>,
    >::new_with_init(env!("CARGO_PKG_NAME"))
    .with_default_duration_s(60)
    .use_build_info(conductor_build_info)
    .use_agent_setup(agent_setup)
    .use_named_agent_behaviour("zero", agent_behaviour_zero)
    .use_named_agent_behaviour("full", agent_behaviour_full)
    .use_agent_teardown(|ctx| {
        uninstall_app(ctx, None).ok();
        Ok(())
    });

    run(builder)?;

    Ok(())
}
