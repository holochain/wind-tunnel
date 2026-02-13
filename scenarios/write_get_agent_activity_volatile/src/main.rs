use holochain_types::prelude::ActionHash;
use holochain_types::prelude::AgentActivity;
use holochain_types::prelude::AgentPubKey;
use holochain_types::prelude::Timestamp;
use holochain_wind_tunnel_runner::happ_path;
use holochain_wind_tunnel_runner::prelude::*;
use rand::prelude::*;
use std::ops::Range;
use std::time::Duration;

const CONDUCTOR_ON_DURATION_RANGE_S: Range<u64> = 10..31;
const CONDUCTOR_OFF_DURATION_RANGE_S: Range<u64> = 2..11;

fn choose_random_duration_in_range(range: Range<u64>) -> Duration {
    let mut rng = rand::rng();
    let options: Vec<u64> = range.collect();
    let choice = options.choose(&mut rng).unwrap();
    Duration::from_secs(*choice)
}

fn conductor_on_duration() -> Duration {
    choose_random_duration_in_range(CONDUCTOR_ON_DURATION_RANGE_S)
}

fn conductor_off_duration() -> Duration {
    choose_random_duration_in_range(CONDUCTOR_OFF_DURATION_RANGE_S)
}

#[derive(Debug, Default)]
pub struct ScenarioValues {
    write_peer: Option<AgentPubKey>,
    shutdown_conductor_at: Option<Timestamp>,
    startup_count: u32,
    shutdown_count: u32,
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
    try_wait_for_min_agents(ctx, Duration::from_secs(120))?;

    // 'write' peers create a link to announce their behaviour so 'get_agent_activity' peers can find them
    if ctx.assigned_behaviour() == "write" {
        let _: ActionHash = call_zome(ctx, "agent_activity", "announce_write_behaviour", ())?;
    }

    if ctx.assigned_behaviour() == "get_agent_activity_volatile" {
        ctx.get_mut().scenario_values.startup_count = 1;

        let reporter = ctx.runner_context().reporter();
        let agent_pub_key = ctx.get().cell_id().agent_pubkey().to_string();
        reporter.add_custom(
            ReportMetric::new("write_get_agent_activity_volatile_startup_count")
                .with_tag("get_agent_activity_volatile_agent", agent_pub_key)
                .with_field("value", ctx.get().scenario_values.startup_count),
        );

        ctx.get_mut().scenario_values.shutdown_conductor_at = Some(Timestamp::from_micros(
            Timestamp::now().as_micros() + conductor_on_duration().as_micros() as i64,
        ));
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

    Ok(())
}

fn agent_behaviour_get_agent_activity_volatile(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let reporter = ctx.runner_context().reporter();

    if let Some(shutdown_at) = ctx.get().scenario_values.shutdown_conductor_at
        && Timestamp::now() >= shutdown_at
    {
        let agent_pub_key = ctx.get().cell_id().agent_pubkey().to_string();

        // Conductor running time is up, stop it
        stop_holochain_conductor(ctx)?;
        ctx.get_mut().scenario_values.shutdown_count += 1;
        reporter.add_custom(
            ReportMetric::new("write_get_agent_activity_volatile_shutdown_count")
                .with_tag("get_agent_activity_volatile_agent", agent_pub_key.clone())
                .with_field("value", ctx.get().scenario_values.shutdown_count),
        );

        // Sleep for the full duration that conductor should remain stopped
        let off_duration = conductor_off_duration();
        let _ = ctx
            .runner_context()
            .executor()
            .execute_in_place(async move {
                tokio::time::sleep(off_duration).await;
                Ok(())
            });

        // Restart the conductor
        start_holochain_conductor(ctx)?;
        ctx.get_mut().scenario_values.startup_count += 1;
        reporter.add_custom(
            ReportMetric::new("write_get_agent_activity_volatile_startup_count")
                .with_tag("get_agent_activity_volatile_agent", agent_pub_key)
                .with_field("value", ctx.get().scenario_values.startup_count),
        );

        // Schedule the running time
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
                ReportMetric::new("write_get_agent_activity_highest_observed_action_seq")
                    .with_tag("get_agent_activity_agent", agent_pub_key)
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
                ctx.get_mut().scenario_values.write_peer = Some(write_peer.clone());
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
    .with_default_duration_s(500)
    .use_build_info(conductor_build_info)
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
