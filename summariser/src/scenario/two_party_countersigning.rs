use crate::aggregator::HostMetricsAggregator;
use crate::analyze::{partitioned_rate_stats, partitioned_timing_stats};
use crate::model::{
    PartitionRateStats, PartitionedRateStats, PartitionedTimingStats, StandardRateStats,
    SummaryOutput,
};
use crate::query;
use crate::query::holochain_p2p_metrics::{HolochainP2pMetrics, query_holochain_p2p_metrics};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::ops::Sub;
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TwoPartyCountersigningSummary {
    accepted_timing: PartitionedTimingStats,
    accepted_success_rate: PartitionedRateStats,
    accepted_failure_rate: PartitionedRateStats,
    initiated_timing: PartitionedTimingStats,
    initiated_success_rate: PartitionedRateStats,
    initiated_failure_rate: PartitionedRateStats,
    holochain_p2p_metrics: HolochainP2pMetrics,
}

pub(crate) async fn summarize_countersigning_two_party(
    client: influxdb::Client,
    summary: RunSummary,
) -> anyhow::Result<SummaryOutput> {
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
    .context("Load accepted")?;
    let accepted_success = partitioned_rate_stats(accepted_success, "value", "10s", &["agent"])
        .context("Accepted success rate")?;

    let accepted_failures = accepted - accepted_success.clone();

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

    let initiated_failures = initiated - initiated_success.clone();

    let host_metrics = HostMetricsAggregator::new(&client, &summary)
        .try_aggregate()
        .await;

    SummaryOutput::new(
        summary.clone(),
        TwoPartyCountersigningSummary {
            accepted_timing: partitioned_timing_stats(accepted_timing, "value", "10s", &["agent"])
                .context("Accepted duration timing")?,
            accepted_success_rate: accepted_success,
            accepted_failure_rate: accepted_failures,
            initiated_timing: partitioned_timing_stats(
                initiated_timing,
                "value",
                "10s",
                &["agent"],
            )
            .context("Initiated duration timing")?,
            initiated_success_rate: initiated_success,
            initiated_failure_rate: initiated_failures,
            holochain_p2p_metrics: query_holochain_p2p_metrics(&client, &summary).await?,
        },
        host_metrics,
    )
}

impl Sub for PartitionedRateStats {
    type Output = PartitionedRateStats;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut self_rates = self.rates.clone();
        let mut rhs_rates = rhs.rates.clone();

        if self_rates.len() != rhs_rates.len() {
            panic!("PartitionedRateStats must have the same number of rates");
        }

        self_rates.sort_by_key(|x| x.key.clone());
        rhs_rates.sort_by_key(|x| x.key.clone());

        PartitionedRateStats {
            mean: self.mean - rhs.mean,
            rates: self_rates
                .into_iter()
                .zip(rhs_rates)
                .map(|(l, r)| {
                    if l.key != r.key {
                        panic!("PartitionRateStats must have the same key");
                    }

                    if l.summary_rate.window_duration != r.summary_rate.window_duration {
                        panic!("PartitionRateStats must have the same window_duration");
                    }

                    // The windowing won't be perfect because observations will be made at different times.
                    // Make a best attempt and then adjust the overflow into a nearby window.
                    let mut trend_diff: Vec<i32> = l
                        .summary_rate
                        .trend
                        .into_iter()
                        .zip(r.summary_rate.trend)
                        .map(|(l, r)| l as i32 - r as i32)
                        .collect();

                    for i in 1..trend_diff.len() {
                        if trend_diff[i] < 0 {
                            trend_diff[i - 1] -= trend_diff[i];
                            trend_diff[i] = 0;
                        }
                    }

                    PartitionRateStats {
                        key: l.key,
                        summary_rate: StandardRateStats {
                            // Re-calculate the mean because the difference isn't meaningful
                            mean: trend_diff.iter().sum::<i32>() as f64 / trend_diff.len() as f64,
                            trend: trend_diff.into_iter().map(|x| x as u32).collect(),
                            window_duration: l.summary_rate.window_duration,
                        },
                    }
                })
                .collect(),
        }
    }
}
