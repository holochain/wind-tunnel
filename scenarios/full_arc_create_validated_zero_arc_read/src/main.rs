use anyhow::anyhow;
use holochain_types::prelude::Record;
use holochain_types::prelude::{ActionHash, Timestamp};
use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;
use std::collections::HashSet;
use std::time::SystemTime;
use timed_and_validated_integrity::TimedSampleEntry;

const RECORD_OPEN_CONNECTIONS_PERIOD_MS: i64 = 3_000;

#[derive(Debug, Default)]
struct ScenarioValues {
    sent_actions_count: u32,
    seen_actions: HashSet<ActionHash>,
    open_connections_last_recorded: Option<Timestamp>,
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
    install_app(
        ctx,
        scenario_happ_path!("timed_and_validated"),
        &"timed_and_validated".to_string(),
    )?;

    Ok(())
}

fn record_open_connections_if_necessary(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    arc: String,
) -> anyhow::Result<()> {
    let now = Timestamp::now();
    let should_record = match ctx.get_mut().scenario_values.open_connections_last_recorded {
        None => true,
        Some(t) => now.as_millis() - t.as_millis() > RECORD_OPEN_CONNECTIONS_PERIOD_MS,
    };

    if should_record {
        let app_client = ctx.get().app_client();
        let network_stats = ctx
            .runner_context()
            .executor()
            .execute_in_place(async move { Ok(app_client.dump_network_stats().await?) })?;

        let metric = ReportMetric::new("full_arc_create_validated_zero_arc_read_open_connections")
            .with_tag("arc", arc)
            .with_field("value", network_stats.connections.len() as u32);
        ctx.runner_context().reporter().clone().add_custom(metric);

        ctx.get_mut().scenario_values.open_connections_last_recorded = Some(now);
    }

    Ok(())
}

fn agent_behaviour_zero(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let found: anyhow::Result<Vec<Record>> =
        call_zome(ctx, "timed_and_validated", "get_timed_entries_network", ());

    let reporter_handle = ctx.runner_context().reporter().clone();
    let agent_pub_key = ctx.get().cell_id().agent_pubkey().to_string();

    if let Ok(records) = found {
        let filtered_records = records
            .into_iter()
            .filter(|r| {
                !ctx.get()
                    .scenario_values
                    .seen_actions
                    .contains(r.action_address())
            })
            .collect::<Vec<_>>();

        for new_record in &filtered_records {
            let timed_sample_entry: TimedSampleEntry = new_record
                .entry()
                .to_app_option()
                .map_err(|e| anyhow!("Failed to deserialize TimedEntry: {}", e))?
                .unwrap();

            let metric = ReportMetric::new("full_arc_create_validated_zero_arc_read_sync_lag");
            let now_us = metric
                .timestamp
                .clone()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_micros();
            let created_us = timed_sample_entry.created_at.as_micros() as u128;
            let lag_s = now_us.saturating_sub(created_us) as f64 / 1e6;

            let metric = metric
                .with_tag("agent", agent_pub_key.clone())
                .with_field("value", lag_s);

            reporter_handle.add_custom(metric);

            ctx.get_mut()
                .scenario_values
                .seen_actions
                .insert(new_record.action_address().clone());
        }
    } else {
        let metric = ReportMetric::new("full_arc_create_validated_zero_arc_read_retrieval_error")
            .with_tag("agent", agent_pub_key.clone())
            .with_tag("arc", "zero")
            .with_field("value", 1_f64);
        reporter_handle.add_custom(metric);
    }

    // Record the total number of entries successfully gotten so far
    let metric = ReportMetric::new("full_arc_create_validated_zero_arc_read_recv_count")
        .with_tag("agent", agent_pub_key)
        .with_field("value", ctx.get().scenario_values.seen_actions.len() as f64);
    reporter_handle.add_custom(metric);

    // Report the number of open connections if necessary
    record_open_connections_if_necessary(ctx, "zero".into())?;

    Ok(())
}

fn agent_behaviour_full(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let _: ActionHash = call_zome(
        ctx,
        "timed_and_validated",
        "create_timed_entry",
        TimedSampleEntry {
            created_at: Timestamp::now(),
            value: String::from("this is a test entry value"),
        },
    )?;

    ctx.get_mut().scenario_values.sent_actions_count += 1;

    // Report number of timed entries created
    let agent_pub_key = ctx.get().cell_id().agent_pubkey().to_string();
    let metric = ReportMetric::new("full_arc_create_validated_zero_arc_read_entry_created_count")
        .with_tag("agent", agent_pub_key)
        .with_tag("arc", "full")
        .with_field("value", ctx.get().scenario_values.sent_actions_count);
    ctx.runner_context().reporter().clone().add_custom(metric);

    // Report the number of open connections
    record_open_connections_if_necessary(ctx, "full".into())?;

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
