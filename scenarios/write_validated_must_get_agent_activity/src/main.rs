use holochain_types::prelude::Timestamp;
use holochain_types::prelude::{ActionHash, AgentPubKey};
use holochain_wind_tunnel_runner::happ_path;
use holochain_wind_tunnel_runner::prelude::*;
use std::time::Duration;
use validated_must_get_agent_activity_coordinator::GetChainTopForBatchInput;
use validated_must_get_agent_activity_coordinator::SampleEntryInput;
use validated_must_get_agent_activity_coordinator::{BatchChainTop, CreateEntriesBatchInput};

const BATCH_SIZE: u32 = 10;
const SLEEP_INTERVAL_WRITE_BEHAVIOUR_MS: u64 = 5_000;

#[derive(Debug, Default)]
pub struct ScenarioValues {
    write_peer: Option<AgentPubKey>,
    batch_count: u32,
    currently_pending_batch_chain_top: Option<BatchChainTop>,
    last_successfully_fetched_batch: Option<u32>,
    retrieval_errors_count: u32,
}

impl UserValuesConstraint for ScenarioValues {}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    start_conductor_and_configure_urls(ctx)?;
    install_app(
        ctx,
        happ_path!("validated_must_get_agent_activity"),
        &"validated_must_get_agent_activity".to_string(),
    )?;
    try_wait_for_min_agents(ctx, Duration::from_secs(120))?;

    // 'write' peers create a link to announce their behaviour so 'get_agent_activity' peers can find them
    if ctx.assigned_behaviour() == "write" {
        let _: ActionHash = call_zome(
            ctx,
            "validated_must_get_agent_activity",
            "announce_write_behaviour",
            (),
        )?;
    }

    Ok(())
}

fn agent_behaviour_write(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let batch_count = ctx.get().scenario_values.batch_count;
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
    let metric = ReportMetric::new("write_validated_must_get_agent_activity_entry_created_count")
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
    let reporter = ctx.runner_context().reporter();

    // get write peer if we don't have one yet
    let Some(write_peer) = ctx.get().scenario_values.write_peer.clone() else {
        log::debug!("Fetching random write peer agent...");
        ctx.get_mut().scenario_values.write_peer = call_zome(
            ctx,
            "validated_must_get_agent_activity",
            "get_random_agent_with_write_behaviour",
            (),
        )?;
        log::info!(
            "Got random write peer agent: {:?}",
            ctx.get().scenario_values.write_peer
        );
        std::thread::sleep(std::time::Duration::from_secs(1));
        return Ok(());
    };

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
    log::debug!("batch_num_to_get: {batch_num_to_get:?}");

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
            Err(err) => {
                log::debug!(
                    "Could not get chain top for batch {next_batch_num} for agent {write_peer}: {err}",
                );
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
    log::debug!("batch_chain_top: {batch_chain_top:?}");

    // Create an entry for which must_get_agent_activity gets called in validation.
    // If the chain cannot fully be retrieved yet until the latest known action hash
    // at the time the entry is attempted to be created, validation and consequently
    // this zome call will fail.
    let create_validate_sample_entry_result: anyhow::Result<()> = call_zome(
        ctx,
        "validated_must_get_agent_activity",
        "create_validated_sample_entry",
        SampleEntryInput {
            agent: write_peer.clone(),
            chain_top: batch_chain_top.chain_top,
            chain_len: batch_chain_top.chain_len,
        },
    );

    // if an error occurred during creation, log it and increase the retrieval error count and return
    if let Err(e) = create_validate_sample_entry_result {
        log::error!(
            "Error creating validated sample entry for agent {write_peer} at batch {}: {e}",
            batch_chain_top.batch_num,
        );
        ctx.get_mut().scenario_values.retrieval_errors_count += 1;
        let metric =
            ReportMetric::new("write_validated_must_get_agent_activity_retrieval_error_count")
                .with_tag("agent", agent_pub_key.clone())
                .with_field(
                    "value",
                    ctx.get_mut().scenario_values.retrieval_errors_count,
                );
        reporter.add_custom(metric);

        std::thread::sleep(std::time::Duration::from_secs(1));
        return Ok(());
    }

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
    let delta_s = (now.as_millis() - batch_chain_top.timestamp.as_millis()) as f64 / 1e3;

    reporter.add_custom(
        ReportMetric::new("write_validated_must_get_agent_activity_chain_batch_delay")
            .with_tag("must_get_agent_activity_agent", agent_pub_key.clone())
            .with_tag("write_agent", write_peer.to_string())
            .with_field("value", delta_s),
    );
    reporter.add_custom(
        ReportMetric::new("write_validated_must_get_agent_activity_chain_len")
            .with_tag("must_get_agent_activity_agent", agent_pub_key)
            .with_tag("write_agent", write_peer.to_string())
            .with_field("value", batch_chain_top.chain_len as f64),
    );

    // Increase the last successfully fetched batch counter by one
    ctx.get_mut()
        .scenario_values
        .last_successfully_fetched_batch = Some(batch_chain_top.batch_num);

    log::info!(
        "Successfully created validated sample entry for agent {write_peer} at batch {}",
        batch_chain_top.batch_num
    );

    std::thread::sleep(Duration::from_secs(1));

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
    .use_named_agent_behaviour("write", agent_behaviour_write)
    .use_named_agent_behaviour(
        "must_get_agent_activity",
        agent_behaviour_must_get_agent_activity,
    )
    .use_agent_teardown(|ctx| {
        uninstall_app(ctx, None).ok();
        Ok(())
    });

    run(builder)?;

    Ok(())
}
