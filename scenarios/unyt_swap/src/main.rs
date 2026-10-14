//! Runs HOT → wHOT bridging followed by wHOT → HF swaps.
//!
//! `full_user` and `zero_user` share the depositor/swapper workload, using full and
//! zero storage arcs respectively. `initiate`, `bridge_agent`, and `swap_agent`
//! always use full arcs, so zero-arc users commit to a full-arc counterparty.

mod behaviours;
mod lane;
mod values;

use crate::values::{ArcType, ScenarioValues};
use holochain_wind_tunnel_runner::happ_path;
use holochain_wind_tunnel_runner::prelude::*;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use wind_tunnel_unyt_scenario::telemetry::record_open_connections_if_necessary;
use wind_tunnel_unyt_scenario::{durable_object::DurableObject, unyt_agent::UnytAgentExt};

const NETWORK_INIT_TIMEOUT: Duration = Duration::from_secs(180);
const LANE_SETUP_TIMEOUT: Duration = Duration::from_secs(180);

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    wind_tunnel_unyt_scenario::setup::common_agent_setup(ctx, happ_path!("unyt"), &["zero_user"])?;

    let agent_key = ctx.get().cell_id().agent_pubkey().to_string();
    let reporter = ctx.runner_context().reporter();
    match ctx.assigned_behaviour() {
        "full_user" | "zero_user" => {
            let arc = if ctx.assigned_behaviour() == "zero_user" {
                ArcType::Zero
            } else {
                ArcType::Full
            };
            for metric in [
                "deposit_raves_collected",
                "swap_commitments_created",
                "swap_receipts_created",
            ] {
                reporter.add_custom(
                    ReportMetric::new(metric)
                        .with_tag("agent", agent_key.clone())
                        .with_tag("arc", arc.as_tag())
                        .with_field("value", 0u64),
                );
            }
        }
        "swap_agent" => reporter.add_custom(
            ReportMetric::new("commitments_accepted")
                .with_tag("agent", agent_key)
                .with_field("value", 0u64),
        ),
        _ => {}
    }

    // The bridge agent publishes its key so the progenitor can authorize it
    // as oracle and bridging_agent when building the lane's agreements.
    if ctx.assigned_behaviour() == "bridge_agent" {
        let run_id = ctx.runner_context().get_run_id().to_string();
        let bridge_agent_key = ctx.get().cell_id().agent_pubkey().to_string();
        let posted = ctx
            .runner_context()
            .executor()
            .execute_in_place(async move {
                DurableObject::new()
                    .post_bridge_agent_key(&run_id, &bridge_agent_key)
                    .await
            })?;
        anyhow::ensure!(
            posted,
            "DurableObject rejected bridge agent key (success=false)"
        );
    }

    // The swap agent publishes its key so swappers can address commitments to it.
    if ctx.assigned_behaviour() == "swap_agent" {
        let run_id = ctx.runner_context().get_run_id().to_string();
        let swap_agent_key = ctx.get().cell_id().agent_pubkey().to_string();
        let posted = ctx
            .runner_context()
            .executor()
            .execute_in_place(async move {
                DurableObject::new()
                    .post_swap_agent_key(&run_id, &swap_agent_key)
                    .await
            })?;
        anyhow::ensure!(
            posted,
            "DurableObject rejected swap agent key (success=false)"
        );
    }

    // The progenitor initializes the network in its behaviour, so it must not wait here.
    if ctx.assigned_behaviour() != "initiate" {
        // Other behaviors should wait for network initialization.
        let agent_key = ctx.get().cell_id().agent_pubkey().to_string();
        let waiting_since = Instant::now();
        while !ctx.is_network_initialized() {
            if ctx.shutdown_listener().should_shutdown() {
                anyhow::bail!("Shutdown while waiting for network init");
            }
            if waiting_since.elapsed() > NETWORK_INIT_TIMEOUT {
                anyhow::bail!(
                    "Network was not initialized within {} s",
                    NETWORK_INIT_TIMEOUT.as_secs()
                );
            }
            log::info!("[{agent_key}] network not initialized, waiting");
            thread::sleep(Duration::from_secs(2));
        }

        // The progenitor also sets up the lane, which other behaviors need to
        // start their actions.
        let waiting_since = Instant::now();
        while ctx
            .unyt_get_all_lane()?
            .into_iter()
            .find_map(|l| l.definition)
            .is_none()
        {
            if ctx.shutdown_listener().should_shutdown() {
                anyhow::bail!("shutdown while waiting for lane setup");
            }
            if waiting_since.elapsed() > LANE_SETUP_TIMEOUT {
                anyhow::bail!(
                    "Lane was not set up within {} s",
                    LANE_SETUP_TIMEOUT.as_secs()
                );
            }
            log::info!("[{agent_key}] lane not available yet, waiting");
            thread::sleep(Duration::from_secs(2));
        }
    }

    Ok(())
}

fn record_open_connections(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let previous = ctx.get().scenario_values.open_connections_last_recorded;
    let next = record_open_connections_if_necessary(ctx, previous)?;
    ctx.get_mut().scenario_values.open_connections_last_recorded = next;
    Ok(())
}

fn main() -> WindTunnelResult<()> {
    log::info!("Starting Unyt Swap scenario");
    let builder = ScenarioDefinitionBuilder::<
        HolochainRunnerContext,
        HolochainAgentContext<ScenarioValues>,
    >::new_with_init(env!("CARGO_PKG_NAME"))
    .use_agent_setup(agent_setup)
    // Progenitor: bootstraps the network, then sets up the lane.
    // Must be named "initiate" so `common_agent_setup` generates the progenitor key.
    .use_named_agent_behaviour("initiate", |ctx| {
        record_open_connections(ctx)?;
        self::behaviours::progenitor::agent_behaviour(ctx)
    })
    // Bridge agent: combines oracle + bridging_agent functions, driving deposits.
    .use_named_agent_behaviour("bridge_agent", |ctx| {
        record_open_connections(ctx)?;
        self::behaviours::bridge_agent::agent_behaviour(ctx)
    })
    // Swap agent: the HoloFuel counterparty that accepts swap commitments.
    .use_named_agent_behaviour("swap_agent", |ctx| {
        record_open_connections(ctx)?;
        self::behaviours::swap_agent::agent_behaviour(ctx)
    })
    // Bridge and swap: receives bridged HOT -> wHOT credit by collecting from the RAVE.
    // Then converts wHOT -> HF by committing to the swap agent and receipting.
    .use_named_agent_behaviour("full_user", |ctx| {
        record_open_connections(ctx)?;
        self::behaviours::user::agent_behaviour(ctx, ArcType::Full)
    })
    .use_named_agent_behaviour("zero_user", |ctx| {
        record_open_connections(ctx)?;
        self::behaviours::user::agent_behaviour(ctx, ArcType::Zero)
    })
    .use_agent_teardown(wind_tunnel_unyt_scenario::behaviour::teardown::agent_teardown)
    .add_capture_env("UNYT_DURABLE_OBJECTS_URL")
    .add_capture_env("MIN_AGENTS");

    run(builder)?;

    Ok(())
}
