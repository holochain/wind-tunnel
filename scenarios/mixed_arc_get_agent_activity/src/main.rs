use holochain_types::prelude::ActionHash;
use holochain_types::prelude::AgentActivity;
use holochain_types::prelude::AgentPubKey;
use holochain_types::prelude::Timestamp;
use holochain_wind_tunnel_runner::happ_path;
use holochain_wind_tunnel_runner::prelude::*;

const RECORD_OPEN_CONNECTIONS_PERIOD_MS: i64 = 3_000;

#[derive(Debug, Default, Clone, Copy)]
pub struct ObservedChainHead {
    pub action_seq: u32,
    pub timestamp_micros: i64,
}

#[derive(Debug, Default)]
pub struct ScenarioValues {
    write_peer: Option<AgentPubKey>,
    entries_created_count: u32,
    retrieval_errors_count: u32,
    last_observed_chain_head: ObservedChainHead,
    open_connections_last_recorded: Option<Timestamp>,
}

impl UserValuesConstraint for ScenarioValues {}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    if ctx.assigned_behaviour() == "zero_write" || ctx.assigned_behaviour() == "zero_read" {
        ctx.get_mut()
            .holochain_config_mut()
            .with_target_arc_factor(0);
    }
    start_conductor_and_configure_urls(ctx)?;
    install_app(
        ctx,
        happ_path!("agent_activity"),
        &"agent_activity".to_string(),
    )?;

    // 'zero_write' and 'full_write' peers create a link to announce their behaviour so 'zero_read' peers can find them
    if ctx.assigned_behaviour() == "zero_write" || ctx.assigned_behaviour() == "full_write" {
        try_wait_until_full_arc_peer_discovered(ctx)?;
        let _: ActionHash = call_zome(ctx, "agent_activity", "announce_write_behaviour", ())?;
    }

    // Set the starting timestamp of last observed chain head to now
    ctx.get_mut().scenario_values.last_observed_chain_head = ObservedChainHead {
        action_seq: 0,
        timestamp_micros: Timestamp::now().0,
    };

    Ok(())
}

fn record_open_connections_if_necessary(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
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
            .execute_in_place(async move { app_client.dump_network_stats().await })?;

        let metric = ReportMetric::new("mixed_arc_get_agent_activity_open_connections")
            .with_tag("behaviour", ctx.assigned_behaviour().to_string())
            .with_field("value", network_stats.connections.len() as u32);
        ctx.runner_context().reporter().clone().add_custom(metric);

        ctx.get_mut().scenario_values.open_connections_last_recorded = Some(now);
    }

    Ok(())
}

fn agent_behaviour_write(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    record_open_connections_if_necessary(ctx)?;

    let _: ActionHash = call_zome(
        ctx,
        "agent_activity",
        "create_sample_entry",
        "this is a test entry value",
    )?;

    ctx.get_mut().scenario_values.entries_created_count += 1;

    let agent_pub_key = ctx.get().cell_id().agent_pubkey().to_string();
    let metric = ReportMetric::new("mixed_arc_get_agent_activity_entry_created_count")
        .with_tag("agent", agent_pub_key)
        .with_tag("behaviour", ctx.assigned_behaviour().to_string())
        .with_field("value", ctx.get().scenario_values.entries_created_count);
    ctx.runner_context().reporter().clone().add_custom(metric);

    Ok(())
}

fn agent_behaviour_get_agent_activity(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    record_open_connections_if_necessary(ctx)?;

    let reporter = ctx.runner_context().reporter();

    match ctx.get().scenario_values.write_peer.clone() {
        Some(write_peer) => {
            let activity_result: anyhow::Result<AgentActivity> = call_zome(
                ctx,
                "agent_activity",
                "get_agent_activity_full",
                write_peer.clone(),
            );

            let agent_pub_key = ctx.get().cell_id().agent_pubkey().to_string();

            if let Ok(activity) = activity_result {
                let previous_observed_chain_head =
                    ctx.get().scenario_values.last_observed_chain_head;

                let highest_observed = activity
                    .highest_observed
                    .map_or(previous_observed_chain_head.action_seq, |v| v.action_seq);

                // Note that the chain head may decrease again in case the agent activity gets fetched
                // from a different peer than in the previous invocation of the behaviour. We therefore
                // allow for positive as well as negative time deltas and divide by the number (positive
                // or negative) of action sequences that the chain jumped since the recorded chain head.
                let n_jump =
                    highest_observed as i32 - previous_observed_chain_head.action_seq as i32;
                if n_jump != 0 {
                    let now = Timestamp::now().0;
                    let time_delta_s =
                        (now as f64 - previous_observed_chain_head.timestamp_micros as f64) / 1e6;
                    let time_delta_per_action_sequence_s = time_delta_s / n_jump as f64;
                    reporter.add_custom(
                        ReportMetric::new("mixed_arc_get_agent_activity_new_chain_head_delay")
                            .with_tag("agent", agent_pub_key.clone())
                            .with_field("value", time_delta_per_action_sequence_s),
                    );

                    reporter.add_custom(
                        ReportMetric::new(
                            "mixed_arc_get_agent_activity_highest_observed_action_seq",
                        )
                        .with_tag("get_agent_activity_agent", agent_pub_key)
                        .with_tag("write_agent", write_peer.to_string())
                        .with_field("value", highest_observed),
                    );

                    // Set new observed chain head
                    ctx.get_mut().scenario_values.last_observed_chain_head = ObservedChainHead {
                        action_seq: highest_observed,
                        timestamp_micros: now,
                    };
                }
            } else {
                ctx.get_mut().scenario_values.retrieval_errors_count += 1;
                let metric =
                    ReportMetric::new("mixed_arc_get_agent_activity_retrieval_error_count")
                        .with_tag("agent", agent_pub_key.clone())
                        .with_field("value", ctx.get().scenario_values.retrieval_errors_count);
                reporter.add_custom(metric);
            }
        }
        None => {
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
    .with_default_duration_s(60)
    .use_agent_setup(agent_setup)
    .use_named_agent_behaviour("zero_read", agent_behaviour_get_agent_activity)
    .use_named_agent_behaviour("zero_write", agent_behaviour_write)
    .use_named_agent_behaviour("full_write", agent_behaviour_write)
    .use_agent_teardown(|ctx| {
        uninstall_app(ctx, None).ok();
        Ok(())
    });

    run(builder)?;

    Ok(())
}
