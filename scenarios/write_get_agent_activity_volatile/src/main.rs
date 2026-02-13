use holochain_types::prelude::ActionHash;
use holochain_types::prelude::AgentActivity;
use holochain_types::prelude::AgentPubKey;
use holochain_types::prelude::Timestamp;
use holochain_wind_tunnel_runner::happ_path;
use holochain_wind_tunnel_runner::prelude::*;
use rand::random_range;
use std::ops::RangeInclusive;
use std::sync::LazyLock;
use std::time::Duration;

static CONDUCTOR_ON_RANGE_S: LazyLock<RangeInclusive<u64>> = LazyLock::new(|| {
    let min: u64 = std::env::var_os("CONDUCTOR_ON_MIN_S")
        .map(|var| {
            var.into_string()
                .expect("Failed to cast env variable CONDUCTOR_ON_MIN_S to string")
                .parse::<u64>()
                .expect("Failed to parse env variable CONDUCTOR_ON_MIN_S to u64")
        })
        .unwrap_or(10);
    let max: u64 = std::env::var_os("CONDUCTOR_ON_MAX_S")
        .map(|var| {
            var.into_string()
                .expect("Failed to cast env variable CONDUCTOR_ON_MAX_S to string")
                .parse::<u64>()
                .expect("Failed to parse env variable CONDUCTOR_ON_MAX_S to u64")
        })
        .unwrap_or(30);
    min..=max
});

static CONDUCTOR_OFF_RANGE_S: LazyLock<RangeInclusive<u64>> = LazyLock::new(|| {
    let min: u64 = std::env::var_os("CONDUCTOR_OFF_MIN_S")
        .map(|var| {
            var.into_string()
                .expect("Failed to cast env variable CONDUCTOR_OFF_MIN_S to string")
                .parse::<u64>()
                .expect("Failed to parse env variable CONDUCTOR_OFF_MIN_S to u64")
        })
        .unwrap_or(2);
    let max: u64 = std::env::var_os("CONDUCTOR_OFF_MAX_S")
        .map(|var| {
            var.into_string()
                .expect("Failed to cast env variable CONDUCTOR_OFF_MAX_S to string")
                .parse::<u64>()
                .expect("Failed to parse env variable CONDUCTOR_OFF_MAX_S to u64")
        })
        .unwrap_or(10);
    min..=max
});

fn choose_random_duration_in_range(range: RangeInclusive<u64>) -> Duration {
    let choice: u64 = random_range(range);
    Duration::from_secs(choice)
}

fn conductor_on_duration() -> Duration {
    choose_random_duration_in_range(CONDUCTOR_ON_RANGE_S.clone())
}

fn conductor_off_duration() -> Duration {
    choose_random_duration_in_range(CONDUCTOR_OFF_RANGE_S.clone())
}

/// Check if the local agent with the primary context cell_id has a storage arc that matches their target arc.
fn has_reached_target_arc(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> anyhow::Result<bool> {
    let cell_id = ctx.get().cell_id();
    let dna_hash = cell_id.dna_hash();
    let dna_hash_clone = dna_hash.clone();
    let app_client = ctx.get().app_client();
    let network_metrics = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move {
            app_client
                .dump_network_metrics(Some(dna_hash_clone), false)
                .await
        })?;
    let local_agent = network_metrics
        .get(dna_hash)
        .expect("Network metrics did not include primary cell")
        .local_agents
        .first()
        .expect("No local agents for primary cell");

    Ok(local_agent.storage_arc == local_agent.target_arc)
}

#[derive(Debug, Default)]
pub struct ScenarioValues {
    write_peer: Option<AgentPubKey>,
    started_conductor_at: Option<Timestamp>,
    shutdown_conductor_at: Option<Timestamp>,
    startup_count: u32,
    shutdown_count: u32,
    /// How long the conductor has been running over the scenario run, in microseconds.
    total_running_time_micros: u64,
}

impl UserValuesConstraint for ScenarioValues {}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    start_conductor_and_configure_urls(ctx)?;
    install_app(
        ctx,
        happ_path!("agent_activity"),
        &"agent_activity".to_string(),
    )?;
    if ctx.assigned_behaviour() == "get_agent_activity_volatile" {
        // Track when the volatile conductor has started up
        ctx.get_mut().scenario_values.started_conductor_at = Some(Timestamp::now());
        ctx.get_mut().scenario_values.startup_count = 1;
        let reporter = ctx.runner_context().reporter();
        let agent_pub_key = ctx.get().cell_id().agent_pubkey().to_string();
        reporter.add_custom(
            ReportMetric::new("startup_count")
                .with_tag("get_agent_activity_volatile_agent", agent_pub_key)
                .with_field("value", ctx.get().scenario_values.startup_count),
        );
    }

    // Wait for min agents
    try_wait_for_min_agents(ctx, Duration::from_secs(120))?;

    // Once we have reached min agents, schedule the volatile conductor shutdown
    if ctx.assigned_behaviour() == "get_agent_activity_volatile" {
        ctx.get_mut().scenario_values.shutdown_conductor_at = Some(Timestamp::from_micros(
            Timestamp::now().as_micros() + conductor_on_duration().as_micros() as i64,
        ));
    }

    // 'write' peers create a link to announce their behaviour so 'get_agent_activity' peers can find them
    if ctx.assigned_behaviour() == "write" {
        let _: ActionHash = call_zome(ctx, "agent_activity", "announce_write_behaviour", ())?;
    }

    Ok(())
}

fn agent_behaviour_write(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let _: ActionHash = call_zome(
        ctx,
        "agent_activity",
        "create_sample_entry",
        "this is a test entry value",
    )?;

    std::thread::sleep(std::time::Duration::from_millis(100));

    Ok(())
}

fn agent_behaviour_get_agent_activity_volatile(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let reporter = ctx.runner_context().reporter();

    if let Some(shutdown_at) = ctx.get().scenario_values.shutdown_conductor_at
        && Timestamp::now() >= shutdown_at
    {
        // Conductors running time is scheduled to end.

        // Report the conductor's off time, running time, total running time, and if it has reached its target arc
        let agent_pub_key = ctx.get().cell_id().agent_pubkey().to_string();

        // Report last running time
        let last_running_time_micros = Timestamp::now().as_micros().saturating_sub(
            ctx.get()
                .scenario_values
                .started_conductor_at
                .unwrap()
                .as_micros(),
        ) as u64;
        reporter.add_custom(
            ReportMetric::new("on_duration_s")
                .with_tag("get_agent_activity_volatile_agent", agent_pub_key.clone())
                .with_field("value", last_running_time_micros as f64 / 1e6_f64),
        );

        // Report total running time
        ctx.get_mut().scenario_values.total_running_time_micros =
            ctx.get().scenario_values.total_running_time_micros + last_running_time_micros;
        let reached_target_arc = has_reached_target_arc(ctx)? as u8;
        reporter.add_custom(
            ReportMetric::new("total_on_duration_s")
                .with_tag("get_agent_activity_volatile_agent", agent_pub_key.clone())
                .with_tag("reached_target_arc", reached_target_arc)
                .with_field(
                    "value",
                    ctx.get().scenario_values.total_running_time_micros / 1_000_000,
                ),
        );
        reporter.add_custom(
            ReportMetric::new("reached_target_arc")
                .with_tag("get_agent_activity_volatile_agent", agent_pub_key.clone())
                .with_field("value", reached_target_arc),
        );

        // Stop the conductor
        stop_holochain_conductor(ctx)?;

        // Report the number of shutdowns
        ctx.get_mut().scenario_values.shutdown_count += 1;
        reporter.add_custom(
            ReportMetric::new("shutdown_count")
                .with_tag("get_agent_activity_volatile_agent", agent_pub_key.clone())
                .with_field("value", ctx.get().scenario_values.shutdown_count),
        );

        // Sleep for the full duration that conductor should remain off
        std::thread::sleep(conductor_off_duration());

        // Restart the conductor
        start_holochain_conductor(ctx)?;
        let started_conductor_at = Timestamp::now();
        ctx.get_mut().scenario_values.started_conductor_at = Some(started_conductor_at);
        ctx.get_mut().scenario_values.startup_count += 1;

        // Report startup
        reporter.add_custom(
            ReportMetric::new("startup_count")
                .with_tag("get_agent_activity_volatile_agent", agent_pub_key.clone())
                .with_field("value", ctx.get().scenario_values.startup_count),
        );

        // Report last off time
        let last_off_time_micros = started_conductor_at.as_micros().saturating_sub(
            ctx.get()
                .scenario_values
                .shutdown_conductor_at
                .unwrap()
                .as_micros(),
        ) as f64;
        reporter.add_custom(
            ReportMetric::new("off_duration_s")
                .with_tag("get_agent_activity_volatile_agent", agent_pub_key)
                .with_field("value", last_off_time_micros / 1e6_f64),
        );

        // Wait for the minimum agents
        try_wait_for_min_agents(ctx, Duration::from_secs(120))?;

        // Schedule the next shutdown time
        ctx.get_mut().scenario_values.shutdown_conductor_at = Some(Timestamp::from_micros(
            Timestamp::now().as_micros() + conductor_on_duration().as_micros() as i64,
        ));
    }

    match ctx.get().scenario_values.write_peer.clone() {
        Some(write_peer) => {
            let activity: AgentActivity = call_zome(
                ctx,
                "agent_activity",
                "get_agent_activity_full",
                write_peer.clone(),
            )?;

            let agent_pub_key = ctx.get().cell_id().agent_pubkey().to_string();
            reporter.add_custom(
                ReportMetric::new("highest_observed_action_seq")
                    .with_tag("get_agent_activity_volatile_agent", agent_pub_key)
                    .with_tag("write_agent", write_peer.to_string())
                    .with_field(
                        "value",
                        activity.highest_observed.map_or(0, |v| v.action_seq),
                    ),
            );
        }
        _ => {
            let maybe_write_peer: Option<AgentPubKey> = call_zome(
                ctx,
                "agent_activity",
                "get_random_agent_with_write_behaviour",
                (),
            )?;

            if let Some(write_peer) = maybe_write_peer {
                ctx.get_mut().scenario_values.write_peer = Some(write_peer);
            }
        }
    }

    Ok(())
}

fn main() -> WindTunnelResult<()> {
    let builder = ScenarioDefinitionBuilder::<
        HolochainRunnerContext,
        HolochainAgentContext<ScenarioValues>,
    >::new_with_init(env!("CARGO_PKG_NAME"))
    .with_default_duration_s(300)
    .use_build_info(conductor_build_info)
    .add_capture_env("CONDUCTOR_ON_MIN_S")
    .add_capture_env("CONDUCTOR_ON_MAX_S")
    .add_capture_env("CONDUCTOR_OFF_MIN_S")
    .add_capture_env("CONDUCTOR_OFF_MAX_S")
    .use_agent_setup(agent_setup)
    .use_named_agent_behaviour("write", agent_behaviour_write)
    .use_named_agent_behaviour(
        "get_agent_activity_volatile",
        agent_behaviour_get_agent_activity_volatile,
    )
    .use_agent_teardown(|ctx| {
        uninstall_app(ctx, None).ok();
        Ok(())
    });

    run(builder)?;

    Ok(())
}
