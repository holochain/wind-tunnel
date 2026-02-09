use crate::analyze::{
    counter_stats, partitioned_rate_stats, partitioned_timing_stats,
    partitioned_timing_stats_allow_empty,
};
use crate::model::{CounterStats, PartitionedRateStats, PartitionedTimingStats};
use crate::query;
use crate::query::holochain_p2p_metrics::{HolochainP2pMetrics, query_holochain_p2p_metrics};
use anyhow::Context;
use polars::prelude::{IntoLazy, col, lit};
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WriteValidatedMustGetAgentActivitySummary {
    write_validated_must_get_agent_activity_chain_len: CounterStats,
    chain_batch_delay_timing: PartitionedTimingStats,
    chain_batch_delay_rate: PartitionedRateStats,
    create_validated_sample_entry_zome_calls: PartitionedTimingStats,
    retrieval_errors: PartitionedTimingStats,
    error_count: usize,
    holochain_p2p_metrics: HolochainP2pMetrics,
}

pub(crate) async fn summarize_write_validated_must_get_agent_activity(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<WriteValidatedMustGetAgentActivitySummary> {
    assert_eq!(
        summary.scenario_name,
        "write_validated_must_get_agent_activity"
    );

    let zome_calls = query::query_zome_call_instrument_data(client.clone(), &summary)
        .await
        .context("Load zome call data")?;

    let write_validated_must_get_agent_activity_chain_len = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.write_validated_must_get_agent_activity_chain_len",
        &["write_agent", "must_get_agent_activity_agent"],
    )
    .await
    .context("Load write_validated_must_get_agent_activity_chain_len data")?;

    let chain_batch_delay = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.write_validated_must_get_agent_activity_chain_batch_delay",
        &["write_agent", "must_get_agent_activity_agent"],
    )
    .await
    .context("Load chain batch delay data")?;

    let retrieval_errors_frame_result = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.write_validated_must_get_agent_activity_retrieval_error_count",
        &["agent"],
    )
    .await;

    let create_validated_sample_entry_zome_calls = zome_calls
        .clone()
        .lazy()
        .filter(col("fn_name").eq(lit("create_validated_sample_entry")))
        .collect()?;

    Ok(WriteValidatedMustGetAgentActivitySummary {
        write_validated_must_get_agent_activity_chain_len: counter_stats(
            write_validated_must_get_agent_activity_chain_len,
            "value",
        )
        .context("Write write_validated_must_get_agent_activity_chain_len stats")?,
        chain_batch_delay_timing: partitioned_timing_stats(
            chain_batch_delay.clone(),
            "value",
            "10s",
            &["must_get_agent_activity_agent"],
        )
        .context("Timing stats for chain batch delay")?,
        chain_batch_delay_rate: partitioned_rate_stats(
            chain_batch_delay,
            "value",
            "10s",
            &["must_get_agent_activity_agent"],
        )
        .context("Rate stats for chain head delay")?,
        create_validated_sample_entry_zome_calls: partitioned_timing_stats(
            create_validated_sample_entry_zome_calls,
            "value",
            "10s",
            &["agent"],
        )
        .context("Write create_validated_sample_entry_zome_calls stats")?,
        retrieval_errors: partitioned_timing_stats_allow_empty(
            retrieval_errors_frame_result,
            "value",
            "10s",
            &["agent"],
        )
        .context("Partitioned timing stats for retrieval errors")?,
        error_count: query::zome_call_error_count(client.clone(), &summary).await?,
        holochain_p2p_metrics: query_holochain_p2p_metrics(&client, &summary).await?,
    })
}
