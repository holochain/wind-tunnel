use crate::analyze::{
    partitioned_counter_stats_allow_empty, partitioned_rate_stats, partitioned_timing_stats,
    round_to_n_dp,
};
use crate::model::{PartitionedCounterStats, PartitionedRateStats, PartitionedTimingStats};
use crate::query;
use crate::query::holochain_p2p_metrics::{HolochainP2pMetrics, query_holochain_p2p_metrics};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TwoPartyCountersigningSummary {
    /// Duration of accepted countersigning sessions per agent (seconds); measures time from
    /// acceptance until the session completes or fails
    accepted_timing: PartitionedTimingStats,
    /// Rate of successfully accepted countersigning sessions per agent (sessions per window)
    accepted_success_rate: PartitionedRateStats,
    /// Cumulative count of failed accepted countersigning sessions; from the direct failure
    /// counter recorded by the scenario
    accepted_failure_count: PartitionedCounterStats,
    /// Fraction of accepted sessions that completed successfully (0–1).
    ///
    /// Computed as accepted_success_rate.mean / total_accepted_rate.mean.
    /// Zero if no sessions were accepted.
    accepted_success_ratio: f64,
    /// Duration of initiated countersigning sessions per agent (seconds); measures time from
    /// initiation until the session completes or fails
    initiated_timing: PartitionedTimingStats,
    /// Rate of successfully initiated countersigning sessions per agent (sessions per window)
    initiated_success_rate: PartitionedRateStats,
    /// Cumulative count of failed initiated countersigning sessions; from the direct failure
    /// counter recorded by the scenario
    initiated_failure_count: PartitionedCounterStats,
    /// Fraction of initiated sessions that completed successfully (0–1).
    ///
    /// Computed as initiated_success_rate.mean / total_initiated_rate.mean.
    /// Zero if no sessions were initiated.
    initiated_success_ratio: f64,
    /// Holochain p2p network metrics for the run
    holochain_p2p_metrics: HolochainP2pMetrics,
}

pub(crate) async fn summarize_countersigning_two_party(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<TwoPartyCountersigningSummary> {
    assert_eq!(summary.scenario_name, "two_party_countersigning");

    let accepted_timing = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.countersigning_session_accepted_duration",
        &["agent"],
    )
    .await
    .context("Accepted duration")?;

    let accepted = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.countersigning_session_accepted",
        &["agent"],
    )
    .await
    .context("Load accepted")?;
    let accepted =
        partitioned_rate_stats(accepted, "value", "10s", &["agent"]).context("Accepted rate")?;

    let accepted_success = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.countersigning_session_accepted_success",
        &["agent"],
    )
    .await
    .context("Load accepted success")?;
    let accepted_success = partitioned_rate_stats(accepted_success, "value", "10s", &["agent"])
        .context("Accepted success rate")?;

    let accepted_failure_result = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.countersigning_session_accepted_failure",
        &["agent"],
    )
    .await;

    let initiated_timing = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.countersigning_session_initiated_duration",
        &["agent"],
    )
    .await
    .context("Initiated duration")?;

    let initiated = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.countersigning_session_initiated",
        &["agent"],
    )
    .await
    .context("Load initiated")?;
    let initiated =
        partitioned_rate_stats(initiated, "value", "10s", &["agent"]).context("Initiated rate")?;

    let initiated_success = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.countersigning_session_initiated_success",
        &["agent"],
    )
    .await
    .context("Load initiated success")?;
    let initiated_success = partitioned_rate_stats(initiated_success, "value", "10s", &["agent"])
        .context("Initiated success rate")?;

    let initiated_failure_result = query::query_custom_data(
        client.clone(),
        &summary,
        "wt.custom.countersigning_session_initiated_failure",
        &["agent"],
    )
    .await;

    // Clamp to [0, 1]: windowing imprecision can cause the success rate to slightly exceed
    // the total rate in some windows, so the ratio must be bounded.
    let accepted_success_ratio = if accepted.mean > 0.0 {
        round_to_n_dp((accepted_success.mean / accepted.mean).clamp(0.0, 1.0), 4)
    } else {
        0.0
    };
    let initiated_success_ratio = if initiated.mean > 0.0 {
        round_to_n_dp((initiated_success.mean / initiated.mean).clamp(0.0, 1.0), 4)
    } else {
        0.0
    };

    Ok(TwoPartyCountersigningSummary {
        accepted_timing: partitioned_timing_stats(accepted_timing, "value", "10s", &["agent"])
            .context("Accepted duration timing")?,
        accepted_success_rate: accepted_success,
        accepted_failure_count: partitioned_counter_stats_allow_empty(
            accepted_failure_result,
            "value",
            "10s",
            &["agent"],
        )
        .context("Accepted failure count")?,
        accepted_success_ratio,
        initiated_timing: partitioned_timing_stats(initiated_timing, "value", "10s", &["agent"])
            .context("Initiated duration timing")?,
        initiated_success_rate: initiated_success,
        initiated_failure_count: partitioned_counter_stats_allow_empty(
            initiated_failure_result,
            "value",
            "10s",
            &["agent"],
        )
        .context("Initiated failure count")?,
        initiated_success_ratio,
        holochain_p2p_metrics: query_holochain_p2p_metrics(&client, &summary).await?,
    })
}
