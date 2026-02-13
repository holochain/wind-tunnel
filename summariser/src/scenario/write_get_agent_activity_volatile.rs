use crate::analyze::{
    partitioned_gauge_stats, partitioned_rate_stats, partitioned_timing_stats,
    running_conductors_stats,
};
use crate::model::{
    GaugeStats, PartitionedGaugeStats, PartitionedRateStats, PartitionedTimingStats,
};
use crate::query::holochain_p2p_metrics::{HolochainP2pMetrics, query_holochain_p2p_metrics};
use crate::query::{query_custom_data, query_zome_call_instrument_data, zome_call_error_count};
use anyhow::Context;
use polars::prelude::{IntoLazy, col, lit};
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WriteGetAgentActivityVolatileSummary {
    highest_observed_action_seq: PartitionedRateStats,
    get_agent_activity_full_zome_calls: PartitionedTimingStats,
    running_conductors_get_agent_activity_volatile: GaugeStats,
    total_on_duration_s: PartitionedTimingStats,
    on_duration_s: PartitionedTimingStats,
    off_duration_s: PartitionedTimingStats,
    reached_target_arc_before_shutdown: PartitionedGaugeStats,
    error_count: usize,
    holochain_p2p_metrics: HolochainP2pMetrics,
}

pub(crate) async fn summarize_write_get_agent_activity_volatile(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<WriteGetAgentActivityVolatileSummary> {
    assert_eq!(summary.scenario_name, "write_get_agent_activity_volatile");

    let highest_observed_action_seq = query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.highest_observed_action_seq",
        &["write_agent", "get_agent_activity_volatile_agent"],
    )
    .await
    .context("Load highest_observed_action_seq data")?;

    let startups = query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.startup_count",
        &["get_agent_activity_volatile_agent"],
    )
    .await
    .context("Load startup_count data")?;

    let shutdowns = query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.shutdown_count",
        &["get_agent_activity_volatile_agent"],
    )
    .await
    .context("Load shutdown_count data")?;

    let get_agent_activity_full_zome_calls =
        query_zome_call_instrument_data(client.clone(), &summary)
            .await
            .context("Load get_agent_activity_full zome call data")?
            .lazy()
            .filter(col("fn_name").eq(lit("get_agent_activity_full")))
            .collect()?;

    let total_on_duration_s = query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.total_on_duration_s",
        &["reached_target_arc", "get_agent_activity_volatile_agent"],
    )
    .await
    .context("Load total on duration data")?;

    let on_duration_s = query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.on_duration_s",
        &["get_agent_activity_volatile_agent"],
    )
    .await
    .context("Load on duration data")?;

    let off_duration_s = query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.off_duration_s",
        &["get_agent_activity_volatile_agent"],
    )
    .await
    .context("Load off duration data")?;

    let reached_target_arc = query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.reached_target_arc",
        &["get_agent_activity_volatile_agent"],
    )
    .await
    .context("Load reached target arc data")?;

    Ok(WriteGetAgentActivityVolatileSummary {
        highest_observed_action_seq: partitioned_rate_stats(
            highest_observed_action_seq,
            "value",
            "10s",
            &["write_agent", "get_agent_activity_volatile_agent"],
        )
        .context("Highest observed action seq stats")?,
        get_agent_activity_full_zome_calls: partitioned_timing_stats(
            get_agent_activity_full_zome_calls,
            "value",
            "10s",
            &["agent"],
        )
        .context("Timing stats for zome call get_agent_activity_full")?,
        running_conductors_get_agent_activity_volatile: running_conductors_stats(
            startups,
            shutdowns,
            "get_agent_activity_volatile_agent",
            "10s",
        )
        .context("Running conductors stats")?,
        total_on_duration_s: partitioned_timing_stats(
            total_on_duration_s.clone(),
            "value",
            "10s",
            &["get_agent_activity_volatile_agent"],
        )
        .context("Total on duration stats")?,
        on_duration_s: partitioned_timing_stats(
            on_duration_s.clone(),
            "value",
            "10s",
            &["get_agent_activity_volatile_agent"],
        )
        .context("On duration stats")?,
        off_duration_s: partitioned_timing_stats(
            off_duration_s.clone(),
            "value",
            "10s",
            &["get_agent_activity_volatile_agent"],
        )
        .context("Off duration stats")?,
        reached_target_arc_before_shutdown: partitioned_gauge_stats(
            reached_target_arc,
            "value",
            &["get_agent_activity_volatile_agent"],
            "10s",
        )
        .context("Reached target arc stats")?,
        error_count: zome_call_error_count(client.clone(), &summary).await?,
        holochain_p2p_metrics: query_holochain_p2p_metrics(&client, &summary).await?,
    })
}
