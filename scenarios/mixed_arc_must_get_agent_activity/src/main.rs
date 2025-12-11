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

#[derive(Debug, Default)]
pub struct ScenarioValues {
    write_peer: Option<AgentPubKey>,
    batch_count: u32,
    retrieval_errors_count: u32,
    currently_pending_batch_chain_top: Option<BatchChainTop>,
    last_successfully_fetched_batch: Option<u32>,
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
        scenario_happ_path!("validated_must_get_agent_activity"),
        &"validated_must_get_agent_activity".to_string(),
    )?;

    // 'zero_write' and 'full_write' peers create a link to announce their behaviour so 'zero_read' peers can find them
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
    println!("\n\n\n%%%%% WRITING WRITING WRITING {batch_count} %%%%%%%%%%%\n\n\n");

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

    // println!("\n\n\n%%%%%WRITE RESULT: {:?}. %%%%%%%%%%%\n\n\n", res);

    ctx.get_mut().scenario_values.batch_count += 1;

    let agent_pub_key = ctx.get().cell_id().agent_pubkey().to_string();
    let metric = ReportMetric::new("mixed_arc_must_get_agent_activity_entry_created_count")
        .with_tag("agent", agent_pub_key)
        .with_tag("behaviour", ctx.assigned_behaviour().to_string())
        .with_field("value", ctx.get().scenario_values.batch_count * BATCH_SIZE);
    ctx.runner_context().reporter().clone().add_custom(metric);

    Ok(())
}

fn agent_behaviour_must_get_agent_activity(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    record_open_connections_if_necessary(ctx)?;

    let reporter = ctx.runner_context().reporter();

    match ctx.get().scenario_values.write_peer.clone() {
        Some(write_peer) => {
            // If we can to move on with the next batch, fetch the associated BatchChainTop
            // ---
            // Iteration 0: currently_pending_batch_chain_top is None, last_successfully_fetched_batch is None
            // Iteration 1: currently_pending_batch_chain_top is Some({ batch_num: 0, ...}), last_successfully_fetched_batch is None
            // Iteration 2.1: currently_pending_batch_chain_top is Some({ batch_num: 0, ...}), last_successfully_fetched_batch is Some(0)
            // Iteration 2.x: currently_pending_batch_chain_top is Some({ batch_num: 1, ...}), last_successfully_fetched_batch is Some(0)
            // Iteration 3.3: currently_pending_batch_chain_top is Some({ batch_num: 1, ...}), last_successfully_fetched_batch is Some(1)
            // Iteration 3.x: currently_pending_batch_chain_top is Some({ batch_num: 2, ...}), last_successfully_fetched_batch is Some(1)
            // ...
            let batch_num_to_get = match (
                ctx.get()
                    .scenario_values
                    .currently_pending_batch_chain_top
                    .clone(),
                ctx.get().scenario_values.last_successfully_fetched_batch,
            ) {
                (None, _) => Some(0),
                (Some(BatchChainTop { batch_num: 0, .. }), None) => Some(1),
                (Some(BatchChainTop { batch_num: m, .. }), Some(n)) => {
                    if m == n {
                        Some(n + 1)
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(next_batch_num) = batch_num_to_get {
                let batch_chain_top: BatchChainTop = match call_zome(
                    ctx,
                    "validated_must_get_agent_activity",
                    "get_chain_top_for_batch",
                    GetChainTopForBatchInput {
                        agent: write_peer.clone(),
                        batch_num: next_batch_num,
                    },
                ) {
                    Ok(t) => t,
                    Err(_) => {
                        println!("\n\n\n#######\nFAILED TO GET CHAIN TOP FOR BATCH #{next_batch_num}\n#######\n\n\n");
                        // The next batch may not yet have been created by the writing peer.
                        return Ok(());
                    }
                };
                ctx.get_mut()
                    .scenario_values
                    .currently_pending_batch_chain_top = Some(batch_chain_top);
            }

            let batch_chain_top = ctx
                .get()
                .scenario_values
                .currently_pending_batch_chain_top
                .clone()
                .expect("No currently pending batch chain top.");

            println!("WRITE_PEER: {:?}", write_peer);

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

            let agent_pub_key = ctx.get().cell_id().agent_pubkey().to_string();

            if let Ok(()) = result {
                // record the time difference between now and when the batch was created. Note that
                // there is likely an accumulating lag here because writing peers write entries faster
                // than reading peers can read it and we always wait until must_get_agent_activity
                // succeeded before proceeding to the next batch.
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
