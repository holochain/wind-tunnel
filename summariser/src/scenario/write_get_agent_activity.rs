use crate::analyze::{chain_head_stats, partitioned_timing_stats};
use crate::model::{ChainHeadStats, PartitionedTimingStats};
use crate::query;
use crate::query::holochain_p2p_metrics::{HolochainP2pMetrics, query_holochain_p2p_metrics};
use anyhow::Context;
use polars::prelude::{IntoLazy, col, lit};
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WriteGetAgentActivitySummary {
    /// Maximum highest-observed action sequence per write agent, aggregated across all reading agents.
    ///
    /// For each write agent, the maximum action sequence seen by any reader is taken (collapsing
    /// the reader dimension). The resulting per-write-agent maxima are then summarised with
    /// `mean_max` and `max` across all write agents. Captures how far readers tracked writers'
    /// chains during the run.
    highest_observed_action_seq: ChainHeadStats,
    /// Duration of `get_agent_activity_full` zome calls per agent (seconds)
    get_agent_activity_full_zome_calls: PartitionedTimingStats,
    /// Number of zome call errors observed during the run
    error_count: usize,
    /// Holochain p2p network metrics for the run
    holochain_p2p_metrics: HolochainP2pMetrics,
}

pub(crate) async fn summarize_write_get_agent_activity(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<WriteGetAgentActivitySummary> {
    assert_eq!(summary.scenario_name, "write_get_agent_activity");

    let highest_observed_action_seq_frame = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.highest_observed_action_seq",
        &["write_agent", "agent"],
    )
    .await
    .context("Load highest_observed_action_seq data")?;

    let get_agent_activity_full_zome_calls =
        query::query_zome_call_instrument_data(client.clone(), &summary)
            .await
            .context("Load get_agent_activity_full zome call data")?
            .clone()
            .lazy()
            .filter(col("fn_name").eq(lit("get_agent_activity_full")))
            .collect()?;

    Ok(WriteGetAgentActivitySummary {
        highest_observed_action_seq: chain_head_stats(
            highest_observed_action_seq_frame,
            "value",
            "write_agent",
            "10s",
        )
        .context("Chain head stats for highest_observed_action_seq")?,
        get_agent_activity_full_zome_calls: partitioned_timing_stats(
            get_agent_activity_full_zome_calls,
            "value",
            "10s",
            &["agent"],
        )
        .context("Timing stats for zome call get_agent_activity_full")?,
        error_count: query::zome_call_error_count(client.clone(), &summary).await?,
        holochain_p2p_metrics: query_holochain_p2p_metrics(&client, &summary).await?,
    })
}
