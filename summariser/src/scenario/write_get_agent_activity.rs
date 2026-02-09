use crate::analyze::counter_stats;
use crate::model::{CounterStats, PartitionedTimingStats};
use crate::query::holochain_p2p_metrics::{HolochainP2pMetrics, query_holochain_p2p_metrics};
use crate::{analyze, query};
use analyze::partitioned_timing_stats;
use anyhow::Context;
use polars::prelude::{IntoLazy, col, lit};
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WriteGetAgentActivitySummary {
    highest_observed_action_seq: CounterStats,
    get_agent_activity_full_zome_calls: PartitionedTimingStats,
    error_count: usize,
    holochain_p2p_metrics: HolochainP2pMetrics,
}

pub(crate) async fn summarize_write_get_agent_activity(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<WriteGetAgentActivitySummary> {
    assert_eq!(summary.scenario_name, "write_get_agent_activity");

    let highest_observed_action_seq = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.write_get_agent_activity_highest_observed_action_seq",
        &["write_agent", "get_agent_activity_agent"],
    )
    .await
    .context("Load write_get_agent_activity_highest_observed_action_seq data")?;

    let get_agent_activity_full_zome_calls =
        query::query_zome_call_instrument_data(client.clone(), &summary)
            .await
            .context("Load get_agent_activity_full zome call data")?
            .clone()
            .lazy()
            .filter(col("fn_name").eq(lit("get_agent_activity_full")))
            .collect()?;

    Ok(WriteGetAgentActivitySummary {
        highest_observed_action_seq: counter_stats(highest_observed_action_seq, "value")
            .context("Highest observed action seq stats")?,
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
