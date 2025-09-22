use holochain_types::dna::AgentPubKey;
use holochain_types::prelude::ActionHash;
use holochain_types::prelude::AgentActivity;
use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub struct ScenarioValues {
    write_peer: Option<AgentPubKey>,
    entries_count: u64,
}

impl UserValuesConstraint for ScenarioValues {}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    start_conductor_and_configure_urls(ctx)?;
    install_app(
        ctx,
        scenario_happ_path!("agent_activity"),
        &"agent_activity".to_string(),
    )?;
    try_wait_for_min_agents(ctx, Duration::from_secs(120))?;

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

    ctx.get_mut().scenario_values.entries_count += 1;

    let reporter = ctx.runner_context().reporter();
    let agent_pub_key = ctx.get().cell_id().agent_pubkey().to_string();
    reporter.add_custom(
        ReportMetric::new("create_entry_count")
            .with_tag("agent", agent_pub_key)
            .with_field("value", ctx.get().scenario_values.entries_count as f64),
    );

    Ok(())
}

fn agent_behaviour_get_agent_activity(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let reporter = ctx.runner_context().reporter();

    match ctx.get().scenario_values.write_peer.clone() {
        Some(write_peer) => {
            let now = Instant::now();
            let activity: AgentActivity =
                call_zome(ctx, "agent_activity", "get_agent_activity_full", write_peer)?;
            let elapsed = now.elapsed();

            let agent_pub_key = ctx.get().cell_id().agent_pubkey().to_string();
            reporter.add_custom(
                ReportMetric::new("read_get_agent_activity")
                    .with_tag("agent", agent_pub_key)
                    .with_field(
                        "highest_observed_action_seq",
                        activity.highest_observed.map_or(0, |v| v.action_seq),
                    )
                    .with_field("value", elapsed.as_secs_f64()),
            );
        }
        _ => {
            if let Some(write_peer) = get_peer_list_randomized(ctx)?.first() {
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
    .with_default_duration_s(60)
    .use_agent_setup(agent_setup)
    .use_named_agent_behaviour("write", agent_behaviour_write)
    .use_named_agent_behaviour("get_agent_activity", agent_behaviour_get_agent_activity)
    .use_agent_teardown(|ctx| {
        uninstall_app(ctx, None).ok();
        Ok(())
    });

    run(builder)?;

    Ok(())
}
