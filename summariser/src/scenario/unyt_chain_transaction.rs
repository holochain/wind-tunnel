use crate::analyze::{aggregated_single_value, partitioned_timing_stats, standard_timing_stats};
use crate::model::{AggregatedSingleValue, PartitionedTimingStats, StandardTimingsStats};
use crate::query;
use anyhow::Context;
use polars::prelude::{IntoLazy, col, lit};
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct UnytChainTransactionSummary {
    /// Seconds elapsed from session start until each agent detected the global_definition
    /// propagation. Recorded once per agent. Records the `arc` type as well.
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
    /// Remaining actionable transaction commitments at teardown.
    /// Non-zero sum indicates work left unfinished.
    pub actionable_transaction_commitments: AggregatedSingleValue,
    /// Remaining actionable transaction accepts at teardown.
    /// Non-zero sum indicates work left unfinished.
    pub actionable_transaction_accepts: AggregatedSingleValue,
    /// Completed transaction accepts at teardown.
    pub completed_transaction_accepts: AggregatedSingleValue,
    /// Completed transaction spends at teardown.
    pub completed_transaction_spends: AggregatedSingleValue,
    /// Completed transaction RAVEs at teardown.
    pub completed_transaction_raves: AggregatedSingleValue,
    /// Parked spend counts at teardown. These are pending link-spending transactions.
    pub parked_spends: AggregatedSingleValue,
    /// Duration of the check_agent_exists loop per behaviour iteration (seconds).
    pub check_agent_exists_duration_s: StandardTimingsStats,
    /// Total check_agent_exists calls made across all agents and iterations.
    pub check_agent_exists_total_calls: AggregatedSingleValue,
    /// Total check_agent_exists calls that failed across all agents and iterations.
    pub check_agent_exists_failed_calls: AggregatedSingleValue,
    /// Total check_agent_exists calls that reported a missing agent across all agents and iterations.
    pub check_agent_exists_missing_calls: AggregatedSingleValue,
    /// Duration of the UI action list refresh per behaviour iteration (seconds).
    pub ui_action_list_refresh_duration_s: StandardTimingsStats,
    /// Total UI action list refresh calls that failed across all agents and iterations.
    pub ui_action_list_refresh_failed_calls: AggregatedSingleValue,
    /// Duration of the full UI routine refresh per behaviour iteration (seconds),
    /// including check_agent_exists, action list refresh, and transaction detail refresh.
    pub ui_routine_refresh_duration_s: StandardTimingsStats,
    /// Watchlist size (number of transactions being tracked) per behaviour iteration.
    /// Indicates how many transactions agents are actively monitoring.
    pub ui_routine_refresh_watchlist_count: AggregatedSingleValue,
    /// Duration of refreshing a single transaction detail item per behaviour iteration (seconds).
    pub ui_transaction_detail_item_refresh_duration_s: StandardTimingsStats,
    /// Total primary transaction fetches made during per-item detail refresh across all agents and iterations.
    pub ui_transaction_detail_item_refresh_primary_transaction_total_calls: AggregatedSingleValue,
    /// Total primary transaction fetches that failed during per-item detail refresh across all agents and iterations.
    pub ui_transaction_detail_item_refresh_primary_transaction_failed_calls: AggregatedSingleValue,
    /// Total related transaction fetches made during per-item detail refresh across all agents and iterations.
    pub ui_transaction_detail_item_refresh_related_transaction_total_calls: AggregatedSingleValue,
    /// Total related transaction fetches that failed during per-item detail refresh across all agents and iterations.
    pub ui_transaction_detail_item_refresh_related_transaction_failed_calls: AggregatedSingleValue,
    /// Duration of refreshing all watched transaction details per behaviour iteration (seconds).
    pub ui_transaction_detail_refresh_duration_s: StandardTimingsStats,
    /// Total number of transactions processed during the detail refresh across all agents and iterations.
    pub ui_transaction_detail_refresh_transactions_processed: AggregatedSingleValue,
    /// Total primary transaction fetches made during the full detail refresh across all agents and iterations.
    pub ui_transaction_detail_refresh_primary_transaction_total_calls: AggregatedSingleValue,
    /// Total primary transaction fetches that failed during the full detail refresh across all agents and iterations.
    pub ui_transaction_detail_refresh_primary_transaction_failed_calls: AggregatedSingleValue,
    /// Total related transaction fetches made during the full detail refresh across all agents and iterations.
    pub ui_transaction_detail_refresh_related_transaction_total_calls: AggregatedSingleValue,
    /// Total related transaction fetches that failed during the full detail refresh across all agents and iterations.
    pub ui_transaction_detail_refresh_related_transaction_failed_calls: AggregatedSingleValue,
    /// Counters of all errors that happened during the scenario run.
    pub error_count: usize,
}

pub(crate) async fn summarize_unyt_chain_transaction(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<UnytChainTransactionSummary> {
    assert_eq!(summary.scenario_name, "unyt_chain_transaction");

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

    let check_agent_exists_duration_s = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.check_agent_exists_duration_s",
        &[],
    )
    .await
    .context("Load check_agent_exists_duration_s data")?;

    let check_agent_exists_total_calls = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.check_agent_exists_total_calls",
        &[],
    )
    .await
    .context("Load check_agent_exists_total_calls data")?;

    let check_agent_exists_failed_calls = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.check_agent_exists_failed_calls",
        &[],
    )
    .await
    .context("Load check_agent_exists_failed_calls data")?;

    let check_agent_exists_missing_calls = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.check_agent_exists_missing_calls",
        &[],
    )
    .await
    .context("Load check_agent_exists_missing_calls data")?;

    let ui_action_list_refresh_duration_s = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_action_list_refresh_duration_s",
        &[],
    )
    .await
    .context("Load ui_action_list_refresh_duration_s data")?;

    let ui_action_list_refresh_failed_calls = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_action_list_refresh_failed_calls",
        &[],
    )
    .await
    .context("Load ui_action_list_refresh_failed_calls data")?;

    let ui_routine_refresh_duration_s = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_routine_refresh_duration_s",
        &[],
    )
    .await
    .context("Load ui_routine_refresh_duration_s data")?;

    let ui_routine_refresh_watchlist_count = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_routine_refresh_watchlist_count",
        &[],
    )
    .await
    .context("Load ui_routine_refresh_watchlist_count data")?;

    let ui_transaction_detail_item_refresh_duration_s = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_transaction_detail_item_refresh_duration_s",
        &[],
    )
    .await
    .context("Load ui_transaction_detail_item_refresh_duration_s data")?;

    let ui_transaction_detail_item_refresh_primary_transaction_total_calls =
        query::query_custom_data(
            client.clone(),
            &summary,
            "wt.custom.ui_transaction_detail_item_refresh_primary_transaction_total_calls",
            &[],
        )
        .await
        .context("Load ui_transaction_detail_item_refresh_primary_transaction_total_calls data")?;

    let ui_transaction_detail_item_refresh_primary_transaction_failed_calls =
        query::query_custom_data(
            client.clone(),
            &summary,
            "wt.custom.ui_transaction_detail_item_refresh_primary_transaction_failed_calls",
            &[],
        )
        .await
        .context("Load ui_transaction_detail_item_refresh_primary_transaction_failed_calls data")?;

    let ui_transaction_detail_item_refresh_related_transaction_total_calls =
        query::query_custom_data(
            client.clone(),
            &summary,
            "wt.custom.ui_transaction_detail_item_refresh_related_transaction_total_calls",
            &[],
        )
        .await
        .context("Load ui_transaction_detail_item_refresh_related_transaction_total_calls data")?;

    let ui_transaction_detail_item_refresh_related_transaction_failed_calls =
        query::query_custom_data(
            client.clone(),
            &summary,
            "wt.custom.ui_transaction_detail_item_refresh_related_transaction_failed_calls",
            &[],
        )
        .await
        .context("Load ui_transaction_detail_item_refresh_related_transaction_failed_calls data")?;

    let ui_transaction_detail_refresh_duration_s = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_transaction_detail_refresh_duration_s",
        &[],
    )
    .await
    .context("Load ui_transaction_detail_refresh_duration_s data")?;

    let ui_transaction_detail_refresh_transactions_processed = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_transaction_detail_refresh_transactions_processed",
        &[],
    )
    .await
    .context("Load ui_transaction_detail_refresh_transactions_processed data")?;

    let ui_transaction_detail_refresh_primary_transaction_total_calls = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_transaction_detail_refresh_primary_transaction_total_calls",
        &[],
    )
    .await
    .context("Load ui_transaction_detail_refresh_primary_transaction_total_calls data")?;

    let ui_transaction_detail_refresh_primary_transaction_failed_calls = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_transaction_detail_refresh_primary_transaction_failed_calls",
        &[],
    )
    .await
    .context("Load ui_transaction_detail_refresh_primary_transaction_failed_calls data")?;

    let ui_transaction_detail_refresh_related_transaction_total_calls = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_transaction_detail_refresh_related_transaction_total_calls",
        &[],
    )
    .await
    .context("Load ui_transaction_detail_refresh_related_transaction_total_calls data")?;

    let ui_transaction_detail_refresh_related_transaction_failed_calls = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.ui_transaction_detail_refresh_related_transaction_failed_calls",
        &[],
    )
    .await
    .context("Load ui_transaction_detail_refresh_related_transaction_failed_calls data")?;

    let error_count = query::zome_call_error_count(client, &summary)
        .await
        .context("Load error count")?;

    Ok(UnytChainTransactionSummary {
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
        check_agent_exists_duration_s: standard_timing_stats(
            check_agent_exists_duration_s,
            "value",
            "10s",
            None,
        )
        .context("Timing stats for check_agent_exists_duration_s")?,
        check_agent_exists_total_calls: aggregated_single_value(
            check_agent_exists_total_calls,
            "value",
        )
        .context("Aggregated single value for check_agent_exists_total_calls")?,
        check_agent_exists_failed_calls: aggregated_single_value(
            check_agent_exists_failed_calls,
            "value",
        )
        .context("Aggregated single value for check_agent_exists_failed_calls")?,
        check_agent_exists_missing_calls: aggregated_single_value(
            check_agent_exists_missing_calls,
            "value",
        )
        .context("Aggregated single value for check_agent_exists_missing_calls")?,
        ui_action_list_refresh_duration_s: standard_timing_stats(
            ui_action_list_refresh_duration_s,
            "value",
            "10s",
            None,
        )
        .context("Timing stats for ui_action_list_refresh_duration_s")?,
        ui_action_list_refresh_failed_calls: aggregated_single_value(
            ui_action_list_refresh_failed_calls,
            "value",
        )
        .context("Aggregated single value for ui_action_list_refresh_failed_calls")?,
        ui_routine_refresh_duration_s: standard_timing_stats(
            ui_routine_refresh_duration_s,
            "value",
            "10s",
            None,
        )
        .context("Timing stats for ui_routine_refresh_duration_s")?,
        ui_routine_refresh_watchlist_count: aggregated_single_value(
            ui_routine_refresh_watchlist_count,
            "value",
        )
        .context("Aggregated single value for ui_routine_refresh_watchlist_count")?,
        ui_transaction_detail_item_refresh_duration_s: standard_timing_stats(
            ui_transaction_detail_item_refresh_duration_s,
            "value",
            "10s",
            None,
        )
        .context("Timing stats for ui_transaction_detail_item_refresh_duration_s")?,
        ui_transaction_detail_item_refresh_primary_transaction_total_calls:
            aggregated_single_value(
                ui_transaction_detail_item_refresh_primary_transaction_total_calls,
                "value",
            )
            .context(
                "Aggregated single value for ui_transaction_detail_item_refresh_primary_transaction_total_calls",
            )?,
        ui_transaction_detail_item_refresh_primary_transaction_failed_calls:
            aggregated_single_value(
                ui_transaction_detail_item_refresh_primary_transaction_failed_calls,
                "value",
            )
            .context(
                "Aggregated single value for ui_transaction_detail_item_refresh_primary_transaction_failed_calls",
            )?,
        ui_transaction_detail_item_refresh_related_transaction_total_calls:
            aggregated_single_value(
                ui_transaction_detail_item_refresh_related_transaction_total_calls,
                "value",
            )
            .context(
                "Aggregated single value for ui_transaction_detail_item_refresh_related_transaction_total_calls",
            )?,
        ui_transaction_detail_item_refresh_related_transaction_failed_calls:
            aggregated_single_value(
                ui_transaction_detail_item_refresh_related_transaction_failed_calls,
                "value",
            )
            .context(
                "Aggregated single value for ui_transaction_detail_item_refresh_related_transaction_failed_calls",
            )?,
        ui_transaction_detail_refresh_duration_s: standard_timing_stats(
            ui_transaction_detail_refresh_duration_s,
            "value",
            "10s",
            None,
        )
        .context("Timing stats for ui_transaction_detail_refresh_duration_s")?,
        ui_transaction_detail_refresh_transactions_processed: aggregated_single_value(
            ui_transaction_detail_refresh_transactions_processed,
            "value",
        )
        .context(
            "Aggregated single value for ui_transaction_detail_refresh_transactions_processed",
        )?,
        ui_transaction_detail_refresh_primary_transaction_total_calls: aggregated_single_value(
            ui_transaction_detail_refresh_primary_transaction_total_calls,
            "value",
        )
        .context(
            "Aggregated single value for ui_transaction_detail_refresh_primary_transaction_total_calls",
        )?,
        ui_transaction_detail_refresh_primary_transaction_failed_calls: aggregated_single_value(
            ui_transaction_detail_refresh_primary_transaction_failed_calls,
            "value",
        )
        .context(
            "Aggregated single value for ui_transaction_detail_refresh_primary_transaction_failed_calls",
        )?,
        ui_transaction_detail_refresh_related_transaction_total_calls: aggregated_single_value(
            ui_transaction_detail_refresh_related_transaction_total_calls,
            "value",
        )
        .context(
            "Aggregated single value for ui_transaction_detail_refresh_related_transaction_total_calls",
        )?,
        ui_transaction_detail_refresh_related_transaction_failed_calls: aggregated_single_value(
            ui_transaction_detail_refresh_related_transaction_failed_calls,
            "value",
        )
        .context(
            "Aggregated single value for ui_transaction_detail_refresh_related_transaction_failed_calls",
        )?,
        error_count,
    })
}
