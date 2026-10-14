use crate::analyze::{
    aggregated_single_value, partitioned_counter_stats, partitioned_counter_stats_allow_empty,
    partitioned_timing_stats,
};
use crate::model::{
    AggregatedSingleValue, PartitionedCounterStats, PartitionedGaugeStats, PartitionedTimingStats,
};
use crate::query;
use crate::scenario::unyt::{
    CommitmentAcceptanceSummary, acceptance_directions, load_agent_roles, load_zome_calls,
    open_connections, optional_frame, with_agent_roles,
};
use anyhow::Context;
use polars::prelude::{DataFrame, IntoLazy, col, lit};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use wind_tunnel_summary_model::RunSummary;

/// User outcomes for one storage arc, aggregated without exposing agent identities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct UnytSwapUserSummary {
    /// Registered users, including users that produced no outcome samples.
    agent_count: usize,
    /// Failed zome calls by users of this arc, across all workload operations.
    error_count: usize,
    /// Collected HOT -> wHOT deposits; counters summarize sampled users.
    deposit_raves_collected: PartitionedCounterStats,
    /// Authored wHOT -> HF commitments, including those still outstanding at shutdown.
    swap_commitments_created: PartitionedCounterStats,
    /// Successfully created receipts finalizing accepted swap commitments.
    swap_receipts_created: PartitionedCounterStats,
    /// Seconds from commitment creation to receipt finalization.
    /// No samples means no observed completions, not a zero-second completion.
    swap_completion_duration_s: Option<PartitionedTimingStats>,
    /// Successful incoming-RAVE polling latency, in seconds; absent without samples.
    get_incoming_raves_zome_call: Option<PartitionedTimingStats>,
    /// Successful deposit-collection latency, in seconds; absent without samples.
    collect_from_rave_zome_call: Option<PartitionedTimingStats>,
    /// Successful swap commitment creation latency, in seconds; absent without samples.
    create_commitment_zome_call: Option<PartitionedTimingStats>,
    /// Successful receipt creation latency, in seconds; absent without samples.
    create_receipt_for_accept_zome_call: Option<PartitionedTimingStats>,
}

/// Bridging infrastructure and separate full/zero user swap outcomes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct UnytSwapSummary {
    /// Successful proof-of-deposit link creation latency for bridge agents, in seconds.
    create_parked_link_zome_call: PartitionedTimingStats,
    /// Successful bridging-agreement execution latency for bridge agents, in seconds.
    execute_rave_zome_call: PartitionedTimingStats,
    /// Successful parked-spend creation latency for bridge agents, in seconds.
    create_parked_spend_zome_call: PartitionedTimingStats,
    /// Cumulative proof-of-deposit links completed by bridge agents.
    bridge_parked_links_completed: PartitionedCounterStats,
    /// Cumulative proof-of-deposit link failures recorded by bridge agents.
    bridge_parked_links_failed: PartitionedCounterStats,
    /// RAVE agreement executions recorded across agents at teardown.
    completed_transaction_raves: AggregatedSingleValue,
    /// Successful commitment acceptance latency for swap agents, in seconds.
    create_accept_zome_call: PartitionedTimingStats,
    /// Cumulative accepted swap commitments across swap agents.
    commitments_accepted: PartitionedCounterStats,
    /// Full and zero user groups; registered population is separate from sampled outcomes.
    users_by_arc: BTreeMap<String, UnytSwapUserSummary>,
    /// Open-connection distributions by behaviour; not proof of synchronization readiness.
    open_connections: PartitionedGaugeStats,
    /// Unique accepted commitments by actual author and acceptor storage arcs.
    accepted_commitments: Vec<CommitmentAcceptanceSummary>,
    /// Failed zome calls across workload and infrastructure roles.
    error_count: usize,
}

async fn load_user_metric(
    client: influxdb::Client,
    summary: &RunSummary,
    roles: &DataFrame,
    metric: &str,
) -> anyhow::Result<Option<DataFrame>> {
    optional_frame(query::query_custom_data(client, summary, metric, &["agent"]).await)?
        .map(|frame| with_agent_roles(frame, roles))
        .transpose()
        .with_context(|| format!("Load {metric}"))
}

fn user_counter(
    frame: Option<&DataFrame>,
    behaviour: &str,
    arc: &str,
) -> anyhow::Result<PartitionedCounterStats> {
    let filtered = frame
        .map(|frame| {
            frame
                .clone()
                .lazy()
                .filter(
                    col("behaviour")
                        .eq(lit(behaviour))
                        .and(col("arc").eq(lit(arc))),
                )
                .collect()
        })
        .transpose()?;
    match filtered {
        Some(frame) if frame.height() > 0 => {
            partitioned_counter_stats(frame, "value", "10s", &["agent"])
        }
        _ => Ok(PartitionedCounterStats {
            total_count: 0,
            partition_count: 0,
            partitions_above_zero: 0,
            mean_count: 0,
            max_per_partition: 0,
            min_per_partition: 0,
            trend: vec![],
            window_duration: "10s".to_string(),
        }),
    }
}

fn optional_timing(frame: DataFrame) -> anyhow::Result<Option<PartitionedTimingStats>> {
    if frame.height() == 0 {
        Ok(None)
    } else {
        partitioned_timing_stats(frame, "value", "10s", &["agent"]).map(Some)
    }
}

pub(crate) async fn summarize_unyt_swap(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<UnytSwapSummary> {
    assert_eq!(summary.scenario_name, "unyt_swap");

    let roles = load_agent_roles(client.clone(), &summary).await?;
    let zome_calls = load_zome_calls(client.clone(), &summary, &roles).await?;
    let successful_calls = zome_calls
        .clone()
        .lazy()
        .filter(col("is_error").eq(lit("false")))
        .collect()?;

    let bridge_parked_links_completed = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.bridge_parked_links_completed",
        &["agent"],
    )
    .await;
    let bridge_parked_links_failed = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.bridge_parked_links_failed",
        &["agent"],
    )
    .await;
    let completed_transaction_raves = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.completed_transaction_raves",
        &[],
    )
    .await
    .context("Load completed_transaction_raves")?;
    let commitments_accepted = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.commitments_accepted",
        &["agent"],
    )
    .await;

    let deposit_raves_collected = load_user_metric(
        client.clone(),
        &summary,
        &roles,
        "wt.custom.deposit_raves_collected",
    )
    .await?;
    let swap_commitments_created = load_user_metric(
        client.clone(),
        &summary,
        &roles,
        "wt.custom.swap_commitments_created",
    )
    .await?;
    let swap_receipts_created = load_user_metric(
        client.clone(),
        &summary,
        &roles,
        "wt.custom.swap_receipts_created",
    )
    .await?;
    let swap_completion_duration_s = load_user_metric(
        client.clone(),
        &summary,
        &roles,
        "wt.custom.swap_completion_duration_s",
    )
    .await?;

    let mut users_by_arc = BTreeMap::new();
    for (arc, behaviour) in [("full", "full_user"), ("zero", "zero_user")] {
        let group_filter = col("behaviour")
            .eq(lit(behaviour))
            .and(col("arc").eq(lit(arc)));
        let agent_count = roles
            .clone()
            .lazy()
            .filter(group_filter.clone())
            .collect()?
            .height();
        let user_calls = zome_calls
            .clone()
            .lazy()
            .filter(group_filter.clone())
            .collect()?;
        let error_count = user_calls
            .clone()
            .lazy()
            .filter(col("is_error").eq(lit("true")))
            .collect()?
            .height();
        let user_successes = user_calls.lazy().filter(col("is_error").eq(lit("false")));
        let user_zome_timing = |fn_name: &str| -> anyhow::Result<Option<PartitionedTimingStats>> {
            optional_timing(
                user_successes
                    .clone()
                    .filter(col("fn_name").eq(lit(fn_name)))
                    .collect()?,
            )
            .with_context(|| format!("{arc} user timing for {fn_name}"))
        };
        let completion_timing = swap_completion_duration_s
            .as_ref()
            .map(|frame| optional_timing(frame.clone().lazy().filter(group_filter).collect()?))
            .transpose()?
            .flatten();

        users_by_arc.insert(
            arc.to_string(),
            UnytSwapUserSummary {
                agent_count,
                error_count,
                deposit_raves_collected: user_counter(
                    deposit_raves_collected.as_ref(),
                    behaviour,
                    arc,
                )?,
                swap_commitments_created: user_counter(
                    swap_commitments_created.as_ref(),
                    behaviour,
                    arc,
                )?,
                swap_receipts_created: user_counter(
                    swap_receipts_created.as_ref(),
                    behaviour,
                    arc,
                )?,
                swap_completion_duration_s: completion_timing,
                get_incoming_raves_zome_call: user_zome_timing("get_incoming_raves")?,
                collect_from_rave_zome_call: user_zome_timing("create_collect_from_rave")?,
                create_commitment_zome_call: user_zome_timing("create_commitment")?,
                create_receipt_for_accept_zome_call: user_zome_timing("create_receipt_for_accept")?,
            },
        );
    }

    let infrastructure_timing =
        |behaviour: &str, fn_name: &str| -> anyhow::Result<PartitionedTimingStats> {
            let filtered = successful_calls
                .clone()
                .lazy()
                .filter(
                    col("behaviour")
                        .eq(lit(behaviour))
                        .and(col("fn_name").eq(lit(fn_name))),
                )
                .collect()?;
            partitioned_timing_stats(filtered, "value", "10s", &["agent"])
                .with_context(|| format!("Timing stats for {behaviour} {fn_name}"))
        };

    Ok(UnytSwapSummary {
        create_parked_link_zome_call: infrastructure_timing("bridge_agent", "create_parked_link")?,
        execute_rave_zome_call: infrastructure_timing("bridge_agent", "execute_rave")?,
        create_parked_spend_zome_call: infrastructure_timing(
            "bridge_agent",
            "create_parked_spend",
        )?,
        bridge_parked_links_completed: partitioned_counter_stats_allow_empty(
            bridge_parked_links_completed,
            "value",
            "10s",
            &["agent"],
        )
        .context("Counter stats for bridge_parked_links_completed")?,
        bridge_parked_links_failed: partitioned_counter_stats_allow_empty(
            bridge_parked_links_failed,
            "value",
            "10s",
            &["agent"],
        )
        .context("Counter stats for bridge_parked_links_failed")?,
        completed_transaction_raves: aggregated_single_value(completed_transaction_raves, "value")
            .context("Aggregated value for completed_transaction_raves")?,
        create_accept_zome_call: infrastructure_timing("swap_agent", "create_accept")?,
        commitments_accepted: partitioned_counter_stats_allow_empty(
            commitments_accepted,
            "value",
            "10s",
            &["agent"],
        )
        .context("Counter stats for commitments_accepted")?,
        users_by_arc,
        open_connections: open_connections(client.clone(), &summary).await?,
        accepted_commitments: acceptance_directions(client, &summary, &roles).await?,
        error_count: zome_calls
            .lazy()
            .filter(col("is_error").eq(lit("true")))
            .collect()?
            .height(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use polars::prelude::{DataType, TimeUnit, df};

    #[test]
    fn full_user_success_does_not_mask_observed_zero_user_inactivity() -> anyhow::Result<()> {
        let counters = df!(
            "time" => &[0_i64, 10_000_000_000, 0, 10_000_000_000],
            "agent" => &["full-agent", "full-agent", "zero-agent", "zero-agent"],
            "behaviour" => &["full_user", "full_user", "zero_user", "zero_user"],
            "arc" => &["full", "full", "zero", "zero"],
            "value" => &[0_u64, 7, 0, 0],
        )?
        .lazy()
        .with_column(col("time").cast(DataType::Datetime(TimeUnit::Nanoseconds, None)))
        .collect()?;

        let full = user_counter(Some(&counters), "full_user", "full")?;
        let zero = user_counter(Some(&counters), "zero_user", "zero")?;

        assert_eq!(
            (
                full.total_count,
                zero.total_count,
                zero.partition_count,
                zero.partitions_above_zero
            ),
            (7, 0, 1, 0),
        );
        Ok(())
    }

    #[test]
    fn full_user_samples_do_not_supply_missing_zero_user_outcomes() -> anyhow::Result<()> {
        let counters = df!(
            "behaviour" => &["full_user"],
            "arc" => &["full"],
            "value" => &[7_u64],
        )?;
        let zero = user_counter(Some(&counters), "zero_user", "zero")?;
        let zero_timings = optional_timing(
            counters
                .lazy()
                .filter(col("arc").eq(lit("zero")))
                .collect()?,
        )?;

        assert_eq!((zero.partition_count, zero_timings.is_none()), (0, true));
        Ok(())
    }
}
