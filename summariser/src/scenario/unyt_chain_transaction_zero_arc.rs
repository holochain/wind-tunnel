use crate::analyze::{
    aggregated_single_value, partitioned_counter_stats_allow_empty, partitioned_timing_stats,
};
use crate::model::{AggregatedSingleValue, PartitionedCounterStats, PartitionedTimingStats};
use crate::query;
use anyhow::Context;
use polars::prelude::{IntoLazy, col, lit};
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct UnytChainTransactionZeroArcSummary {
    /// Seconds elapsed from session start until each agent detected the global_definition
    /// propagation. Recorded once per agent via the `at` field. Records the `arc` type
    /// as well.
    pub global_definition_propagation_time: PartitionedTimingStats,
    /// Time between commitment creation and discovery by the receiving agent (seconds),
    /// partitioned by arc and agent.
    pub sync_lag_commitment: PartitionedTimingStats,
    /// Time between RAVE creation and discovery by smart_agreements agents (seconds),
    /// partitioned by arc and agent.
    pub sync_lag_rave: PartitionedTimingStats,
    /// Time between grouped_parked request creation and discovery (seconds),
    /// partitioned by arc and agent
    pub sync_lag_grouped_parked: PartitionedTimingStats,
    /// Final ledger balance across all agents at teardown (base unyt units).
    /// Sum should be stable across runs; large deviation from expected indicates value was created or destroyed.
    pub ledger_balance: AggregatedSingleValue,
    /// Final ledger fees owed across all agents at teardown (base unyt units).
    pub ledger_fees: AggregatedSingleValue,
    /// Remaining actionable transaction proposals at teardown.
    /// Non-zero sum indicates work left unfinished.
    pub actionable_transaction_proposals: AggregatedSingleValue,
    /// Remaining actionable transaction commitments at teardown.
    /// Non-zero sum indicates work left unfinished.
    pub actionable_transaction_commitments: AggregatedSingleValue,
    /// Remaining actionable transaction accepts at teardown.
    /// Non-zero sum indicates work left unfinished.
    pub actionable_transaction_accepts: AggregatedSingleValue,
    /// Remaining actionable transaction rejects at teardown.
    /// Non-zero sum indicates work left unfinished.
    pub actionable_transaction_rejects: AggregatedSingleValue,
    /// Completed transaction accepts at teardown.
    pub completed_transaction_accepts: AggregatedSingleValue,
    /// Completed transaction spends at teardown.
    pub completed_transaction_spends: AggregatedSingleValue,
    /// Completed transaction RAVEs at teardown.
    pub completed_transaction_raves: AggregatedSingleValue,
    /// Parked spend counts at teardown. These are pending link-spending transactions.
    pub parked_spends: AggregatedSingleValue,
    /// Cumulative count of unique code templates discovered by observer agents over the run,
    /// partitioned by arc type (zero/full) and agent. Observers poll the code template library
    /// and this tracks their discovery rate.
    pub recv_count: PartitionedCounterStats,
    /// Counters of all errors that happened during the scenario run.
    pub error_count: usize,
}

pub(crate) async fn summarize_unyt_chain_transaction_zero_arc(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<UnytChainTransactionZeroArcSummary> {
    assert_eq!(summary.scenario_name, "unyt_chain_transaction_zero_arc");

    let global_definition_propagation_time = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.global_definition_propagation_time",
        &["agent", "arc"],
    )
    .await
    .context("Load global_definition_propagation_time data")?;

    let sync_lag = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.sync_lag",
        &["tx_type", "arc", "agent"],
    )
    .await
    .context("Load sync_lag data")?;
    let sync_lag_commitment = sync_lag
        .clone()
        .lazy()
        .filter(col("tx_type").eq(lit("commitment")))
        .collect()?;
    let sync_lag_rave = sync_lag
        .clone()
        .lazy()
        .filter(col("tx_type").eq(lit("rave")))
        .collect()?;
    let sync_lag_grouped_parked = sync_lag
        .lazy()
        .filter(col("tx_type").eq(lit("grouped_parked")))
        .collect()?;

    let ledger_balance = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ledger_balance",
        &["agent"],
    )
    .await
    .context("Load ledger_balance data")?;

    let ledger_fees = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ledger_fees",
        &["agent"],
    )
    .await
    .context("Load ledger_fees data")?;

    let actionable_transaction_proposals = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.actionable_transaction_proposals",
        &["agent"],
    )
    .await
    .context("Load actionable_transaction_proposals data")?;

    let actionable_transaction_commitments = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.actionable_transaction_commitments",
        &["agent"],
    )
    .await
    .context("Load actionable_transaction_commitments data")?;

    let actionable_transaction_accepts = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.actionable_transaction_accepts",
        &["agent"],
    )
    .await
    .context("Load actionable_transaction_accepts data")?;

    let actionable_transaction_rejects = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.actionable_transaction_rejects",
        &["agent"],
    )
    .await
    .context("Load actionable_transaction_rejects data")?;

    let completed_transaction_accepts = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.completed_transaction_accepts",
        &["agent"],
    )
    .await
    .context("Load completed_transaction_accepts data")?;

    let completed_transaction_spends = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.completed_transaction_spends",
        &["agent"],
    )
    .await
    .context("Load completed_transaction_spends data")?;

    let completed_transaction_raves = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.completed_transaction_raves",
        &["agent"],
    )
    .await
    .context("Load completed_transaction_raves data")?;

    let parked_spends = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.parked_spends",
        &["agent"],
    )
    .await
    .context("Load parked_spends data")?;

    let recv_count_result = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.recv_count",
        &["agent", "arc"],
    )
    .await;

    let error_count = query::zome_call_error_count(client, &summary)
        .await
        .context("Load error count")?;

    Ok(UnytChainTransactionZeroArcSummary {
        global_definition_propagation_time: partitioned_timing_stats(
            global_definition_propagation_time,
            "value",
            "10s",
            &["agent", "arc"],
        )
        .context("Timing stats for global_definition_propagation_time")?,
        sync_lag_commitment: partitioned_timing_stats(
            sync_lag_commitment,
            "value",
            "10s",
            &["arc", "agent"],
        )
        .context("Partitioned timing stats for commitment sync lag")?,
        sync_lag_rave: partitioned_timing_stats(sync_lag_rave, "value", "10s", &["arc", "agent"])
            .context("Partitioned timing stats for rave sync lag")?,
        sync_lag_grouped_parked: partitioned_timing_stats(
            sync_lag_grouped_parked,
            "value",
            "10s",
            &["arc", "agent"],
        )
        .context("Partitioned timing stats for grouped_parked sync lag")?,
        ledger_balance: aggregated_single_value(ledger_balance, "value")
            .context("Aggregated single value for ledger_balance")?,
        ledger_fees: aggregated_single_value(ledger_fees, "value")
            .context("Aggregated single value for ledger_fees")?,
        actionable_transaction_proposals: aggregated_single_value(
            actionable_transaction_proposals,
            "value",
        )
        .context("Aggregated single value for actionable_transaction_proposals")?,
        actionable_transaction_commitments: aggregated_single_value(
            actionable_transaction_commitments,
            "value",
        )
        .context("Aggregated single value for actionable_transaction_commitments")?,
        actionable_transaction_accepts: aggregated_single_value(
            actionable_transaction_accepts,
            "value",
        )
        .context("Aggregated single value for actionable_transaction_accepts")?,
        actionable_transaction_rejects: aggregated_single_value(
            actionable_transaction_rejects,
            "value",
        )
        .context("Aggregated single value for actionable_transaction_rejects")?,
        completed_transaction_accepts: aggregated_single_value(
            completed_transaction_accepts,
            "value",
        )
        .context("Aggregated single value for completed_transaction_accepts")?,
        completed_transaction_spends: aggregated_single_value(
            completed_transaction_spends,
            "value",
        )
        .context("Aggregated single value for completed_transaction_spends")?,
        completed_transaction_raves: aggregated_single_value(completed_transaction_raves, "value")
            .context("Aggregated single value for completed_transaction_raves")?,
        parked_spends: aggregated_single_value(parked_spends, "value")
            .context("Aggregated single value for parked_spends")?,
        recv_count: partitioned_counter_stats_allow_empty(
            recv_count_result,
            "value",
            "10s",
            &["arc", "agent"],
        )
        .context("Counter stats for recv_count")?,
        error_count,
    })
}
