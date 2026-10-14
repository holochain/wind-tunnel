//! Shared role, connection, and commitment acceptance measurements for Unyt scenarios.

use crate::{ArcType, UnytScenarioValues};
use holochain_wind_tunnel_runner::prelude::*;
use rave_engine::types::Transaction;
use std::time::{Duration, Instant};

const RECORD_OPEN_CONNECTIONS_PERIOD: Duration = Duration::from_secs(3);

/// Registers the installed agent's assigned behaviour and storage arc once during setup.
pub fn record_agent_role<SV: UnytScenarioValues>(
    ctx: &AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    arc: ArcType,
) {
    ctx.runner_context().reporter().add_custom(
        ReportMetric::new("agent_role")
            .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string())
            .with_tag("behaviour", ctx.assigned_behaviour().to_string())
            .with_tag("arc", arc.as_tag())
            .with_field("value", 1u64),
    );
}

/// Samples open connections when due, returning the timestamp for scenario-owned state.
pub fn record_open_connections_if_necessary<SV: UnytScenarioValues>(
    ctx: &AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    previous: Option<Instant>,
) -> anyhow::Result<Option<Instant>> {
    let now = Instant::now();
    if previous
        .is_some_and(|recorded| now.duration_since(recorded) <= RECORD_OPEN_CONNECTIONS_PERIOD)
    {
        return Ok(previous);
    }

    let app_client = ctx.get().app_client();
    let network_stats = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move { app_client.dump_network_stats().await })?;
    ctx.runner_context().reporter().add_custom(
        ReportMetric::new("open_connections")
            .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string())
            .with_tag("behaviour", ctx.assigned_behaviour().to_string())
            .with_field(
                "value",
                network_stats.transport_stats.connections.len() as u32,
            ),
    );

    Ok(Some(now))
}

/// Records an acceptance after `create_accept` succeeds, using the commitment's actual author.
pub fn record_commitment_accepted<SV: UnytScenarioValues>(
    ctx: &AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    commitment: &Transaction,
) {
    ctx.runner_context().reporter().add_custom(
        ReportMetric::new("commitment_accepted")
            .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string())
            .with_tag("author", commitment.creator.to_string())
            .with_tag("commitment", commitment.id.to_string())
            .with_field("value", 1u64),
    );
}
