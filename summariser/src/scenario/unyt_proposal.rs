use std::collections::BTreeMap;

use super::unyt::{self, CommitmentAcceptanceSummary};
use crate::analyze::{aggregated_single_value, partitioned_timing_stats, standard_timing_stats};
use crate::model::{
    AggregatedSingleValue, PartitionedGaugeStats, PartitionedTimingStats, StandardTimingsStats,
};
use crate::query;
use anyhow::Context;
use polars::prelude::{DataFrame, IntoLazy, col, lit};
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

const WORKLOAD_ROLES: [&str; 4] = [
    "full_propose",
    "zero_propose",
    "full_respond",
    "zero_respond",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct UnytProposalSummary {
    /// Separate aggregate metrics for the four proposer/responder storage-arc roles.
    workloads_by_behaviour: BTreeMap<String, UnytProposalWorkloadSummary>,
    /// Failed zome calls across workload and initialization roles.
    error_count: usize,
    /// Open-connection distributions by behaviour; not proof of DHT integration.
    open_connections: PartitionedGaugeStats,
    /// Unique accepted commitments attributed to their actual authors and acceptors.
    accepted_commitments: Vec<CommitmentAcceptanceSummary>,
}

/// Metrics are filtered to one registered behaviour before any aggregation.
/// Missing observations remain absent, including missing teardown observations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct UnytProposalWorkloadSummary {
    /// Registered agents, including those with no observed workload outcomes.
    agent_count: usize,
    /// Distinct agents with observed accepted round trips, not the number of transactions.
    proposal_round_trip_accepted_agent_count: usize,
    /// Distinct agents with observed rejected round trips, not the number of transactions.
    proposal_round_trip_rejected_agent_count: usize,
    /// Failed zome calls for this workload role, across all operations.
    error_count: usize,
    /// Seconds since session start until network initialization was first observed.
    global_definition_propagation_time: Option<StandardTimingsStats>,
    /// Accepted proposal round-trip duration, in seconds; absent without completed samples.
    proposal_round_trip_accepted: Option<PartitionedTimingStats>,
    /// Rejected proposal round-trip duration, in seconds; absent without completed samples.
    proposal_round_trip_rejected: Option<PartitionedTimingStats>,
    /// Observed negotiation-history lengths before commitments; absent without samples.
    negotiation_rounds: Option<StandardTimingsStats>,
    /// Proposal publish-to-observation delay, in seconds, including possible clock skew.
    sync_lag_proposal: Option<PartitionedTimingStats>,
    /// Commitment publish-to-observation delay, in seconds, including possible clock skew.
    sync_lag_commitment: Option<PartitionedTimingStats>,
    /// Accept publish-to-observation delay, in seconds, including possible clock skew.
    sync_lag_accept: Option<PartitionedTimingStats>,
    /// Reject publish-to-observation delay, in seconds, including possible clock skew.
    sync_lag_reject: Option<PartitionedTimingStats>,
    /// Successful proposal creation latency, in seconds; absent without samples.
    create_proposal_zome_call: Option<PartitionedTimingStats>,
    /// Successful counter-proposal creation latency, in seconds; absent without samples.
    create_counter_proposal_zome_call: Option<PartitionedTimingStats>,
    /// Successful commit-to-proposal latency, in seconds; absent without samples.
    create_commit_to_proposal_zome_call: Option<PartitionedTimingStats>,
    /// Successful commitment acceptance latency, in seconds; absent without samples.
    create_accept_zome_call: Option<PartitionedTimingStats>,
    /// Successful rejection creation latency, in seconds; absent without samples.
    create_reject_proposal_zome_call: Option<PartitionedTimingStats>,
    /// Successful receipt creation latency, in seconds; absent without samples.
    create_receipt_for_accept_zome_call: Option<PartitionedTimingStats>,
    /// Successful balance reclaim latency, in seconds; absent without samples.
    create_reclaim_balance_zome_call: Option<PartitionedTimingStats>,
    /// Successful actionable-transaction polling latency, in seconds; absent without samples.
    get_actionable_transactions_zome_call: Option<PartitionedTimingStats>,
    /// Accept transactions in sampled agent histories at teardown.
    completed_transaction_accepts: Option<AggregatedSingleValue>,
    /// Spend transactions in sampled agent histories at teardown.
    completed_transaction_spends: Option<AggregatedSingleValue>,
    /// RAVE executions in sampled agent histories at teardown.
    completed_transaction_raves: Option<AggregatedSingleValue>,
    /// Proposals still actionable at teardown; absent if no agents reported teardown.
    actionable_transaction_proposals: Option<AggregatedSingleValue>,
    /// Commitments still actionable at teardown; absent if no agents reported teardown.
    actionable_transaction_commitments: Option<AggregatedSingleValue>,
    /// Accepts still actionable at teardown; absent if no agents reported teardown.
    actionable_transaction_accepts: Option<AggregatedSingleValue>,
    /// Rejections still actionable at teardown; absent if no agents reported teardown.
    actionable_transaction_rejects: Option<AggregatedSingleValue>,
}

pub(crate) async fn summarize_unyt_proposal(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<UnytProposalSummary> {
    assert_eq!(summary.scenario_name, "unyt_proposal");
    let roles = unyt::load_agent_roles(client.clone(), &summary).await?;
    let zome_calls = unyt::load_zome_calls(client.clone(), &summary, &roles).await?;
    let propagation = load_metric(
        client.clone(),
        &summary,
        &roles,
        "global_definition_propagation_time",
        &[],
    )
    .await?;
    let round_trips = load_metric(
        client.clone(),
        &summary,
        &roles,
        "proposal_round_trip_time",
        &["outcome"],
    )
    .await?;
    let negotiations =
        load_metric(client.clone(), &summary, &roles, "negotiation_rounds", &[]).await?;
    let sync_lag = load_metric(client.clone(), &summary, &roles, "sync_lag", &["tx_type"]).await?;

    let mut teardown = BTreeMap::new();
    for metric in [
        "completed_transaction_accepts",
        "completed_transaction_spends",
        "completed_transaction_raves",
        "actionable_transaction_proposals",
        "actionable_transaction_commitments",
        "actionable_transaction_accepts",
        "actionable_transaction_rejects",
    ] {
        teardown.insert(
            metric,
            load_metric(client.clone(), &summary, &roles, metric, &[]).await?,
        );
    }

    let mut workloads_by_behaviour = BTreeMap::new();
    for behaviour in WORKLOAD_ROLES {
        let calls = filter(&zome_calls, "behaviour", behaviour)?;
        let successful_calls = filter(&calls, "is_error", "false")?;
        let round_trips = filter_optional(round_trips.as_ref(), "behaviour", behaviour)?;
        let accepted = filter_optional(round_trips.as_ref(), "outcome", "accepted")?;
        let rejected = filter_optional(round_trips.as_ref(), "outcome", "rejected")?;
        let lag = filter_optional(sync_lag.as_ref(), "behaviour", behaviour)?;
        let zome_timing =
            |name| partitioned(filter_optional(Some(&successful_calls), "fn_name", name)?);
        let lag_timing = |tx_type| partitioned(filter_optional(lag.as_ref(), "tx_type", tx_type)?);
        let teardown_value = |metric: &str| -> anyhow::Result<Option<AggregatedSingleValue>> {
            filter_optional(teardown[metric].as_ref(), "behaviour", behaviour)?
                .map(|frame| aggregated_single_value(frame, "value"))
                .transpose()
        };
        workloads_by_behaviour.insert(
            behaviour.to_string(),
            UnytProposalWorkloadSummary {
                agent_count: filter(&roles, "behaviour", behaviour)?.height(),
                proposal_round_trip_accepted_agent_count: observed_agents(accepted.as_ref())?,
                proposal_round_trip_rejected_agent_count: observed_agents(rejected.as_ref())?,
                error_count: filter(&calls, "is_error", "true")?.height(),
                global_definition_propagation_time: standard(filter_optional(
                    propagation.as_ref(),
                    "behaviour",
                    behaviour,
                )?)?,
                proposal_round_trip_accepted: partitioned(accepted)?,
                proposal_round_trip_rejected: partitioned(rejected)?,
                negotiation_rounds: standard(filter_optional(
                    negotiations.as_ref(),
                    "behaviour",
                    behaviour,
                )?)?,
                sync_lag_proposal: lag_timing("proposal")?,
                sync_lag_commitment: lag_timing("commitment")?,
                sync_lag_accept: lag_timing("accept")?,
                sync_lag_reject: lag_timing("reject")?,
                create_proposal_zome_call: zome_timing("create_proposal")?,
                create_counter_proposal_zome_call: zome_timing("create_counter_proposal")?,
                create_commit_to_proposal_zome_call: zome_timing("create_commit_to_proposal")?,
                create_accept_zome_call: zome_timing("create_accept")?,
                create_reject_proposal_zome_call: zome_timing("create_reject_proposal")?,
                create_receipt_for_accept_zome_call: zome_timing("create_receipt_for_accept")?,
                create_reclaim_balance_zome_call: zome_timing("create_reclaim_balance")?,
                get_actionable_transactions_zome_call: zome_timing("get_actionable_transactions")?,
                completed_transaction_accepts: teardown_value("completed_transaction_accepts")?,
                completed_transaction_spends: teardown_value("completed_transaction_spends")?,
                completed_transaction_raves: teardown_value("completed_transaction_raves")?,
                actionable_transaction_proposals: teardown_value(
                    "actionable_transaction_proposals",
                )?,
                actionable_transaction_commitments: teardown_value(
                    "actionable_transaction_commitments",
                )?,
                actionable_transaction_accepts: teardown_value("actionable_transaction_accepts")?,
                actionable_transaction_rejects: teardown_value("actionable_transaction_rejects")?,
            },
        );
    }
    Ok(UnytProposalSummary {
        workloads_by_behaviour,
        error_count: filter(&zome_calls, "is_error", "true")?.height(),
        open_connections: unyt::open_connections(client.clone(), &summary).await?,
        accepted_commitments: unyt::acceptance_directions(client, &summary, &roles).await?,
    })
}

async fn load_metric(
    client: influxdb::Client,
    summary: &RunSummary,
    roles: &DataFrame,
    metric: &str,
    extra_tags: &[&str],
) -> anyhow::Result<Option<DataFrame>> {
    let mut tags = vec!["agent"];
    tags.extend_from_slice(extra_tags);
    let result =
        query::query_custom_data(client, summary, &format!("wt.custom.{metric}"), &tags).await;
    unyt::optional_frame(result)
        .with_context(|| format!("Load {metric}"))?
        .map(|frame| unyt::with_agent_roles(frame, roles))
        .transpose()
}

fn filter(frame: &DataFrame, column: &str, value: &str) -> anyhow::Result<DataFrame> {
    Ok(frame
        .clone()
        .lazy()
        .filter(col(column).eq(lit(value)))
        .collect()?)
}

fn filter_optional(
    frame: Option<&DataFrame>,
    column: &str,
    value: &str,
) -> anyhow::Result<Option<DataFrame>> {
    Ok(frame
        .map(|frame| filter(frame, column, value))
        .transpose()?
        .filter(|frame| frame.height() > 0))
}

fn observed_agents(frame: Option<&DataFrame>) -> anyhow::Result<usize> {
    match frame {
        Some(frame) => Ok(frame.column("agent")?.n_unique()?),
        None => Ok(0),
    }
}

fn standard(frame: Option<DataFrame>) -> anyhow::Result<Option<StandardTimingsStats>> {
    frame
        .map(|frame| standard_timing_stats(frame, "value", "10s", None))
        .transpose()
}

fn partitioned(frame: Option<DataFrame>) -> anyhow::Result<Option<PartitionedTimingStats>> {
    frame
        .map(|frame| partitioned_timing_stats(frame, "value", "10s", &["agent"]))
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use polars::df;

    #[test]
    fn workload_outcomes_do_not_merge_full_and_zero_agents() -> anyhow::Result<()> {
        let outcomes = df![
            "agent" => ["full-a", "full-a", "zero-a", "zero-b"],
            "behaviour" => ["full_propose", "full_propose", "zero_propose", "zero_propose"],
            "outcome" => ["accepted", "accepted", "rejected", "rejected"],
            "value" => [1i64, 3, 8, 12],
        ]?;
        let full = filter_optional(Some(&outcomes), "behaviour", "full_propose")?;
        let zero = filter_optional(Some(&outcomes), "behaviour", "zero_propose")?;
        let full_accepted = filter_optional(full.as_ref(), "outcome", "accepted")?;
        let zero_accepted = filter_optional(zero.as_ref(), "outcome", "accepted")?;
        let zero_rejected = filter_optional(zero.as_ref(), "outcome", "rejected")?;
        assert_eq!(observed_agents(full_accepted.as_ref())?, 1);
        assert_eq!(observed_agents(zero_rejected.as_ref())?, 2);
        assert!(zero_accepted.is_none());
        assert!(partitioned(zero_accepted)?.is_none());
        assert_eq!(
            aggregated_single_value(full_accepted.unwrap(), "value")?.sum,
            4
        );
        assert_eq!(
            aggregated_single_value(zero_rejected.unwrap(), "value")?.sum,
            20
        );
        Ok(())
    }

    #[test]
    fn missing_teardown_is_distinct_from_observed_zero() -> anyhow::Result<()> {
        let teardown = df![
            "agent" => ["full-a"],
            "behaviour" => ["full_respond"],
            "value" => [0i64],
        ]?;
        let full = filter_optional(Some(&teardown), "behaviour", "full_respond")?;
        let zero = filter_optional(Some(&teardown), "behaviour", "zero_respond")?;
        let observed = aggregated_single_value(full.unwrap(), "value")?;
        assert_eq!(observed.sum, 0);
        assert_eq!(observed.partition_count, 1);
        assert!(zero.is_none());
        Ok(())
    }
}
