//! Role attribution and cross-arc acceptance analysis shared by Unyt summaries.

use crate::analyze::partitioned_gauge_stats;
use crate::frame::LoadError;
use crate::model::PartitionedGaugeStats;
use crate::query;
use anyhow::{Context, ensure};
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

/// Successful accepts attributed to the actual commitment author and acceptor.
/// Counts unique commitment/acceptor pairs, not repeated successful API calls.
/// A direction absent from this list had no observed successful accepts;
/// acceptance does not imply a receipt was finalized before shutdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CommitmentAcceptanceSummary {
    /// Storage arc of the agent that authored the actual commitment: full or zero.
    pub author_arc: String,
    /// Storage arc of the agent whose create_accept call succeeded: full or zero.
    pub acceptor_arc: String,
    /// Unique commitment/acceptor pairs successfully accepted during this run.
    pub accepted_count: u64,
}

/// Load the public-key-to-role registry emitted at app installation.
/// Network-wide measurements are attributed independently of each runner's
/// local `assigned_behaviours` metadata.
pub(crate) async fn load_agent_roles(
    client: influxdb::Client,
    summary: &RunSummary,
) -> anyhow::Result<DataFrame> {
    let roles = query::query_custom_data(
        client,
        summary,
        "wt.custom.agent_role",
        &["agent", "behaviour", "arc"],
    )
    .await
    .context("Load Unyt agent roles; capture mixed-arc data with role instrumentation")?
    .lazy()
    .select([col("agent"), col("behaviour"), col("arc")])
    .unique_stable(None, UniqueKeepStrategy::First)
    .collect()?;
    ensure!(
        roles.column("agent")?.n_unique()? == roles.height(),
        "An Unyt agent has conflicting role registrations"
    );
    Ok(roles)
}

/// Attribute samples to roles without dropping unregistered agents silently.
pub(crate) fn with_agent_roles(frame: DataFrame, roles: &DataFrame) -> anyhow::Result<DataFrame> {
    let frame = frame
        .lazy()
        .join(
            roles.clone().lazy(),
            [col("agent")],
            [col("agent")],
            JoinArgs::new(JoinType::Left),
        )
        .collect()?;
    ensure!(
        frame.column("behaviour")?.null_count() == 0 && frame.column("arc")?.null_count() == 0,
        "Unyt samples reference agents missing from the role registry"
    );
    Ok(frame)
}

/// Fetch successful and failed zome calls together, excluding admin operations.
/// `is_error` remains a string tag; consumers must exclude errors from timings.
pub(crate) async fn load_zome_calls(
    client: influxdb::Client,
    summary: &RunSummary,
    roles: &DataFrame,
) -> anyhow::Result<DataFrame> {
    let frame = query::query_custom_data(
        client,
        summary,
        "wt.instruments.operation_duration",
        &["agent", "fn_name", "is_error", "operation_id"],
    )
    .await
    .context("Load Unyt zome calls")?
    .lazy()
    .filter(
        col("operation_id")
            .eq(lit("app_call_zome"))
            .or(col("operation_id").eq(lit("trycp_app_call_zome"))),
    )
    .collect()?;
    with_agent_roles(frame, roles)
}

/// No observations are optional; transport, parsing and analysis errors are not.
pub(crate) fn optional_frame(
    result: anyhow::Result<DataFrame>,
) -> anyhow::Result<Option<DataFrame>> {
    match result {
        Ok(frame) if frame.is_empty() => Ok(None),
        Ok(frame) => Ok(Some(frame)),
        Err(error) if error.downcast_ref::<LoadError>().is_some() => Ok(None),
        Err(error) => Err(error),
    }
}

/// Connection distributions retain behaviour labels, including infrastructure.
/// A connection count is not proof of DHT integration or authority readiness.
pub(crate) async fn open_connections(
    client: influxdb::Client,
    summary: &RunSummary,
) -> anyhow::Result<PartitionedGaugeStats> {
    let frame = query::query_custom_data(
        client,
        summary,
        "wt.custom.open_connections",
        &["behaviour"],
    )
    .await
    .context("Load Unyt open connections")?;
    partitioned_gauge_stats(frame, "value", &["behaviour"], "10s")
        .context("Connection distributions by Unyt behaviour")
}

/// Load successful accepts and correlate both public keys with registered arcs.
pub(crate) async fn acceptance_directions(
    client: influxdb::Client,
    summary: &RunSummary,
    roles: &DataFrame,
) -> anyhow::Result<Vec<CommitmentAcceptanceSummary>> {
    let Some(events) = optional_frame(
        query::query_custom_data(
            client,
            summary,
            "wt.custom.commitment_accepted",
            &["agent", "author", "commitment"],
        )
        .await,
    )?
    else {
        return Ok(Vec::new());
    };
    let events = events
        .lazy()
        .select([col("agent"), col("author"), col("commitment")])
        .unique_stable(
            Some(vec!["agent".into(), "commitment".into()]),
            UniqueKeepStrategy::First,
        )
        .collect()?;
    let mut events = with_agent_roles(events, roles)?;
    events.rename("arc", "acceptor_arc".into())?;
    let events = events
        .lazy()
        .join(
            roles
                .clone()
                .lazy()
                .select([col("agent").alias("author"), col("arc").alias("author_arc")]),
            [col("author")],
            [col("author")],
            JoinArgs::new(JoinType::Left),
        )
        .collect()?;
    ensure!(
        events.column("author_arc")?.null_count() == 0,
        "Accepted commitments reference authors missing from the role registry"
    );
    let counts = events
        .lazy()
        .group_by([col("author_arc"), col("acceptor_arc")])
        .agg([len().cast(DataType::UInt64).alias("accepted_count")])
        .sort(["author_arc", "acceptor_arc"], Default::default())
        .collect()?;
    let authors = counts.column("author_arc")?.str()?;
    let acceptors = counts.column("acceptor_arc")?.str()?;
    let values = counts.column("accepted_count")?.u64()?;
    (0..counts.height())
        .map(|row| {
            Ok(CommitmentAcceptanceSummary {
                author_arc: authors.get(row).context("Missing author arc")?.to_string(),
                acceptor_arc: acceptors
                    .get(row)
                    .context("Missing acceptor arc")?
                    .to_string(),
                accepted_count: values.get(row).context("Missing accepted count")?,
            })
        })
        .collect()
}
