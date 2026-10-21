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

/// Outcomes for one user behaviour, aggregated without exposing agent identities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct UnytSwapUserSummary {
    /// Registered users, including users that produced no outcome samples.
    agent_count: usize,
    /// Failed zome calls by users of this arc, across all workload operations.
    error_count: usize,
    /// Collected HOT -> wHOT deposits; absent without counter observations.
    deposit_raves_collected: Option<PartitionedCounterStats>,
    /// Authored wHOT -> HF commitments, including outstanding ones; absent without samples.
    swap_commitments_created: Option<PartitionedCounterStats>,
    /// Receipts finalizing accepted swap commitments; absent without counter observations.
    swap_receipts_created: Option<PartitionedCounterStats>,
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
    /// Duration of `create_parked_link` zome calls per agent (seconds).
    /// The bridge agent's oracle step that records a proof of deposit for each user.
    create_parked_link_zome_call: PartitionedTimingStats,
    /// Duration of `execute_rave` zome calls per agent (seconds).
    /// The bridging-agent step that executes the credit-limit and bridging
    /// agreements, turning a proof of deposit into a deposit RAVE.
    execute_rave_zome_call: PartitionedTimingStats,
    /// Duration of `create_parked_spend` zome calls per agent (seconds).
    /// The bridging-agent step that parks the spend on the bridging agreement
    /// before executing it.
    create_parked_spend_zome_call: PartitionedTimingStats,
    /// Proof-of-deposit parked links the bridge agent completed, as a cumulative
    /// per-agent counter. `total_count` reflects deposit volume offered across the run.
    bridge_parked_links_completed: PartitionedCounterStats,
    /// Proof-of-deposit parked links the bridge agent failed, as a cumulative
    /// per-agent counter. `total_count` reflects deposit volume offered across the run.
    bridge_parked_links_failed: PartitionedCounterStats,
    /// Total RAVE agreement executions across all agents at teardown.
    /// Reflects deposits that executed via the bridging agreement.
    completed_transaction_raves: AggregatedSingleValue,
    /// Duration of `create_accept` zome calls per agent (seconds).
    /// The swap agent's step accepting a swap commitment.
    create_accept_zome_call: PartitionedTimingStats,
    /// Swap commitments the swap agent accepted, as a cumulative per-agent counter.
    /// `total_count` is the run-wide accepted count across all swap agents.
    commitments_accepted: PartitionedCounterStats,
    /// Full and zero user groups; registered population is separate from sampled outcomes.
    workloads_by_behaviour: BTreeMap<String, UnytSwapUserSummary>,
    /// Open-connection distributions by behaviour; not proof of synchronization readiness.
    open_connections: PartitionedGaugeStats,
    /// Unique accepted commitments by actual author and acceptor storage arcs.
    accepted_commitments: Vec<CommitmentAcceptanceSummary>,
    /// Number of zome call errors observed during the run.
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
) -> anyhow::Result<Option<PartitionedCounterStats>> {
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
            partitioned_counter_stats(frame, "value", "10s", &["agent"]).map(Some)
        }
        _ => Ok(None),
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

    let mut workloads_by_behaviour = BTreeMap::new();
    for (arc, behaviour) in [("full", "user"), ("zero", "zero_user")] {
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

        workloads_by_behaviour.insert(
            behaviour.to_string(),
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
        workloads_by_behaviour,
        open_connections: open_connections(client.clone(), &summary).await?,
        accepted_commitments: acceptance_directions(client, &summary, &roles).await?,
        error_count: zome_calls
            .lazy()
            .filter(col("is_error").eq(lit("true")))
            .collect()?
            .height(),
    })
}
