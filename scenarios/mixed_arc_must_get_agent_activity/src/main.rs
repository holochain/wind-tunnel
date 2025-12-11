use holochain_types::prelude::ActionHash;
use holochain_types::prelude::AgentPubKey;
use holochain_types::prelude::Timestamp;
use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;
use validated_must_get_agent_activity_coordinator::{
    BatchChainTop, CreateEntriesBatchInput, GetChainTopForBatchInput, SampleEntryInput,
};

const RECORD_OPEN_CONNECTIONS_PERIOD_MS: i64 = 3_000;
const BATCH_SIZE: u32 = 10;
const SLEEP_INTERVAL_WRITE_BEHAVIOUR_MS: u64 = 5_000;

#[derive(Debug, Default)]
pub struct ScenarioValues {
    write_peer: Option<AgentPubKey>,
    batch_count: u32,
    currently_pending_batch_chain_top: Option<BatchChainTop>,
    last_successfully_fetched_batch: Option<u32>,
    retrieval_errors_count: u32,
    open_connections_last_recorded: Option<Timestamp>,
}

impl UserValuesConstraint for ScenarioValues {}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    if ctx.assigned_behaviour() == "zero_write"
        || ctx.assigned_behaviour() == "zero_must_get_agent_activity"
    {
        ctx.get_mut()
            .holochain_config_mut()
            .with_target_arc_factor(0);
    }
    start_conductor_and_configure_urls(ctx)?;
    install_app(
        ctx,
        scenario_happ_path!("validated_must_get_agent_activity"),
        &"validated_must_get_agent_activity".to_string(),
    )?;

    // 'zero_write' and 'full_write' peers create a link to announce their behaviour so
    // 'zero_must_get_agent_activity' peers can find them
    if ctx.assigned_behaviour() == "zero_write" || ctx.assigned_behaviour() == "full_write" {
        try_wait_until_full_arc_peer_discovered(ctx)?;
        let _: ActionHash = call_zome(
            ctx,
            "validated_must_get_agent_activity",
            "announce_write_behaviour",
            (),
        )?;
    }

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
            .execute_in_place(async move { Ok(app_client.dump_network_stats().await?) })?;

        let metric = ReportMetric::new("mixed_arc_must_get_agent_activity_open_connections")
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

    let batch_count = ctx.get().scenario_values.batch_count;

    // Create a batch of 10 entries
    let _: () = call_zome(
        ctx,
        "validated_must_get_agent_activity",
        "create_sample_entries_batch",
        CreateEntriesBatchInput {
            num_entries: BATCH_SIZE,
            batch_num: batch_count,
        },
    )?;

    ctx.get_mut().scenario_values.batch_count += 1;

    let agent_pub_key = ctx.get().cell_id().agent_pubkey().to_string();
    let metric = ReportMetric::new("mixed_arc_must_get_agent_activity_entry_created_count")
        .with_tag("agent", agent_pub_key)
        .with_tag("behaviour", ctx.assigned_behaviour().to_string())
        .with_field("value", ctx.get().scenario_values.batch_count * BATCH_SIZE);
    ctx.runner_context().reporter().clone().add_custom(metric);

    std::thread::sleep(std::time::Duration::from_millis(
        SLEEP_INTERVAL_WRITE_BEHAVIOUR_MS,
    ));

    Ok(())
}

fn agent_behaviour_must_get_agent_activity(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    record_open_connections_if_necessary(ctx)?;

    let reporter = ctx.runner_context().reporter();

    match ctx.get().scenario_values.write_peer.clone() {
        Some(write_peer) => {
            // If we can move on with the next batch, fetch the associated BatchChainTop. Otherwise proceed
            // with the BatchChainTop from the previous iteration.
            // ---
            // Iteration 0: currently_pending_batch_chain_top=None, last_successfully_fetched_batch=None
            // Iteration 1: currently_pending_batch_chain_top=Some({ batch_num: 0, ...}), last_successfully_fetched_batch=None
            // Iteration 2.1: currently_pending_batch_chain_top=Some({ batch_num: 0, ...}), last_successfully_fetched_batch=Some(0)
            // Iteration 2.x: currently_pending_batch_chain_top=Some({ batch_num: 1, ...}), last_successfully_fetched_batch=Some(0)
            // Iteration 3.3: currently_pending_batch_chain_top=Some({ batch_num: 1, ...}), last_successfully_fetched_batch=Some(1)
            // Iteration 3.x: currently_pending_batch_chain_top=Some({ batch_num: 2, ...}), last_successfully_fetched_batch=Some(1)
            // ...
            let batch_num_to_get = match (
                ctx.get()
                    .scenario_values
                    .currently_pending_batch_chain_top
                    .clone(),
                ctx.get().scenario_values.last_successfully_fetched_batch,
            ) {
                (None, _) => Some(0),
                (Some(BatchChainTop { batch_num, .. }), Some(last_successfully_fetched_batch)) => {
                    if batch_num == last_successfully_fetched_batch {
                        Some(last_successfully_fetched_batch + 1)
                    } else {
                        None
                    }
                }
                _ => None,
            };

            let agent_pub_key = ctx.get().cell_id().agent_pubkey().to_string();

            let batch_chain_top = if let Some(next_batch_num) = batch_num_to_get {
                match call_zome::<GetChainTopForBatchInput, BatchChainTop, ScenarioValues>(
                    ctx,
                    "validated_must_get_agent_activity",
                    "get_chain_top_for_batch",
                    GetChainTopForBatchInput {
                        agent: write_peer.clone(),
                        batch_num: next_batch_num,
                    },
                ) {
                    Ok(batch_chain_top) => {
                        ctx.get_mut()
                            .scenario_values
                            .currently_pending_batch_chain_top = Some(batch_chain_top.clone());
                        batch_chain_top
                    }
                    Err(_) => {
                        // The next batch may not yet have been created by the writing peer.
                        return Ok(());
                    }
                }
            } else {
                // Use the one from the previous behaviour run again
                ctx.get()
                    .scenario_values
                    .currently_pending_batch_chain_top
                    .clone()
                    .expect("No currently pending batch chain top.")
            };

            // Create an entry for which must_get_agent_activity gets called in validation.
            // If the chain cannot fully be retrieved yet until the latest known action hash
            // at the time the entry is attempted to be created, validation and consequently
            // this zome call will fail.
            let result: anyhow::Result<()> = call_zome(
                ctx,
                "validated_must_get_agent_activity",
                "create_validated_sample_entry",
                SampleEntryInput {
                    agent: write_peer.clone(),
                    chain_top: batch_chain_top.chain_top,
                    chain_len: batch_chain_top.chain_len,
                },
            );

            if let Ok(()) = result {
                // Record the time difference between now and when the batch was created.
                // Note that if this difference is longer than SLEEP_INTERVAL_WRITE_BEHAVIOUR_MS,
                // it will stop to be a representative measure for the time it takes from writing
                // a batch of entries and being able to retrieve them via must_get_agent_activity,
                // since the writing behaviour will create entries in a higher frequency than we
                // attempt to retrieve them here, given that we only move on with retrieving the
                // next batch of entries once we successfully retrieved the one before.
                // Furthermore there might be a clock difference between the node that created
                // the batch and our own clock here, which may add a constant offset.
                let now = Timestamp::now();
                let delta_s =
                    (now.as_millis() - batch_chain_top.timestamp.as_millis()) as f64 / 1e3;

                reporter.add_custom(
                    ReportMetric::new("mixed_arc_must_get_agent_activity_chain_batch_delay")
                        .with_tag("must_get_agent_activity_agent", agent_pub_key.clone())
                        .with_tag("write_agent", write_peer.to_string())
                        .with_field("value", delta_s),
                );
                reporter.add_custom(
                    ReportMetric::new("mixed_arc_must_get_agent_activity_chain_len")
                        .with_tag("must_get_agent_activity_agent", agent_pub_key)
                        .with_tag("write_agent", write_peer.to_string())
                        .with_field("value", batch_chain_top.chain_len as f64),
                );

                // Increase the last successfully fetched batch counter by one
                ctx.get_mut()
                    .scenario_values
                    .last_successfully_fetched_batch = Some(batch_chain_top.batch_num);
            } else {
                ctx.get_mut().scenario_values.retrieval_errors_count += 1;
                let metric =
                    ReportMetric::new("mixed_arc_must_get_agent_activity_retrieval_error_count")
                        .with_tag("agent", agent_pub_key.clone())
                        .with_field(
                            "value",
                            ctx.get_mut().scenario_values.retrieval_errors_count,
                        );
                reporter.add_custom(metric);
            }
        }
        None => {
            let maybe_write_peer: Option<AgentPubKey> = call_zome(
                ctx,
                "validated_must_get_agent_activity",
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
    .use_named_agent_behaviour(
        "zero_must_get_agent_activity",
        agent_behaviour_must_get_agent_activity,
    )
    .use_named_agent_behaviour("zero_write", agent_behaviour_write)
    .use_named_agent_behaviour("full_write", agent_behaviour_write)
    .use_agent_teardown(|ctx| {
        uninstall_app(ctx, None).ok();
        Ok(())
    });

    run(builder)?;

    Ok(())
}
