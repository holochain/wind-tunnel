mod holochain_metrics;
mod host_metrics;

use std::time::Duration;

use serde::{Deserialize, Serialize};
use wind_tunnel_summary_model::RunSummary;

pub use self::holochain_metrics::{HolochainDatabaseKind, HolochainWorkflowKind};
pub use self::host_metrics::{
    AnomalyStatus, CpuMetrics, DiskMetrics, DiskSpace, DiskThroughput, HostAnomalies, HostMetrics,
    MemMetrics, NetMetrics, OomRisk, PrimaryNetStats, ProcessMetrics, PsiMetrics, PsiResource,
    PsiStall, Severity, SystemLoadMetrics,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SummaryOutput {
    pub run_summary: RunSummary,
    pub scenario_metrics: serde_json::Value,
    pub host_metrics: Option<HostMetrics>,
}

impl SummaryOutput {
    pub fn new<V>(
        run_summary: RunSummary,
        data: V,
        host_metrics: Option<HostMetrics>,
    ) -> anyhow::Result<Self>
    where
        V: serde::Serialize,
    {
        Ok(Self {
            run_summary,
            scenario_metrics: serde_json::to_value(data)?,
            host_metrics,
        })
    }
}

/// A windowed mean trend.
///
/// Useful for understanding how a metric evolves over time, in a compacted form.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Float64Trend {
    /// Windowed mean values in chronological order.
    pub trend: Vec<f64>,
    /// Duration of each window, e.g. "10 s".
    pub window_duration: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StandardTimingsStats {
    /// Mean operation duration in seconds
    pub mean: f64,
    /// Standard deviation of operation durations in seconds.
    ///
    /// A low std indicates consistent performance, as long as the measured operations are similar.
    pub std: f64,
    /// 50th percentile (median) operation duration in seconds.
    pub p50: f64,
    /// 95th percentile duration in seconds; only 5% of operations exceed this value.
    pub p95: f64,
    /// 99th percentile operation duration in seconds; only 1% of operations exceed this value.
    pub p99: f64,
    /// Windowed mean over time; useful for detecting warming/cooling patterns
    pub trend: Float64Trend,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StandardRateStats {
    /// Mean number of operations per window (not per second — divide by window_duration to get ops/s)
    pub mean: f64,
    /// Count of operations per window over time; first and last windows may be partial
    pub trend: Vec<u32>,
    /// Duration of each window (e.g. "10s")
    pub window_duration: String,
}

/// Gauge statistics for a continuously-sampled metric
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct GaugeStats {
    /// Arithmetic mean across all samples
    pub mean: f64,
    /// Standard deviation; indicates stability of the gauge
    pub std: f64,
    /// 5th percentile (lower bound of the 90% operating range)
    pub p5: f64,
    /// 95th percentile (upper bound of the 90% operating range)
    pub p95: f64,
    /// Windowed mean over time; useful for detecting trends
    pub trend: Float64Trend,
}

/// GaugeStats for a specific partition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PartitionGaugeStats {
    pub key: Vec<PartitionKey>,
    pub gauge_stats: GaugeStats,
}

/// GaugeStats partitioned by tag/key
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PartitionedGaugeStats {
    pub partitions: Vec<PartitionGaugeStats>,
}

/// Stats which increment during time
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct CounterStats {
    /// The difference between the first and last observed value of the counter during the measurement period
    pub count: u64,
    /// Duration of the measurement
    pub measurement_duration: Duration,
    /// Mean rate per second (from instantaneous rate samples)
    pub mean_rate_per_second: f64,
    /// Std deviation of instantaneous rate per second
    pub std_rate_per_second: f64,
    /// 5th percentile rate per second
    pub p5_rate_per_second: f64,
    /// 95th percentile rate per second
    pub p95_rate_per_second: f64,
    /// Peak (max) instantaneous rate per second
    pub peak_rate_per_second: f64,
    /// Windowed rate trend over time
    pub rate_trend: Float64Trend,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
pub struct PartitionKey {
    pub key: String,
    pub value: String,
}

/// Aggregate timing statistics for a metric partitioned by tag (e.g. per agent).
///
/// Per-partition data is not retained. Following the same pattern as host metrics
/// (e.g. `max_host_used_percent` rather than per-host values), three aggregates
/// summarise the distribution across partitions without exposing agent-level detail.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PartitionedTimingStats {
    /// Weighted mean latency across all partitions (mean of per-partition means), seconds
    pub mean: f64,
    /// Average standard deviation across partitions; reflects per-partition consistency.
    ///
    /// Note: this is NOT the std of the combined distribution — it does not capture
    /// cross-partition spread. Compare `max_mean` against `mean` to assess that.
    pub mean_std_dev: f64,
    /// Highest per-partition mean latency (seconds); identifies the worst-case agent.
    ///
    /// A large gap between `max_mean` and `mean` indicates one agent is significantly
    /// slower than the rest.
    pub max_mean: f64,
    /// Lowest per-partition mean latency (seconds); identifies the best-case agent.
    ///
    /// A large gap between `max_mean` and `min_mean` indicates high variance across agents.
    pub min_mean: f64,
    /// Mean latency per window averaged across partitions, in chronological order.
    ///
    /// Each element is the per-partition mean latency in that window averaged across
    /// all partitions. Useful for spotting warm-up or degradation over time.
    pub trend: Vec<f64>,
    /// Duration of each trend window (e.g. "10s")
    pub window_duration: String,
}

/// Aggregate rate statistics for a metric partitioned by tag (e.g. per agent).
///
/// Per-partition data is not retained. Three aggregates characterize the distribution
/// of rates across partitions without providing excessive detail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionedRateStats {
    /// Mean operation rate across all partitions.
    ///
    /// The rate is presented as a count but has been computed over the window duration, so it can
    /// be interpreted as a rate per window (e.g. "100 ops per 10 s").
    pub mean: f64,
    /// Highest per-partition mean rate; detects load concentration on one partition.
    ///
    /// Note that depending on the partition, this may be meaningless.
    pub max_mean: f64,
    /// Lowest per-partition mean rate; detects idle or underperforming partition.
    ///
    /// Note that depending on the partition, this may be meaningless.
    pub min_mean: f64,
    /// Mean count per window across partitions, in chronological order.
    ///
    /// Each element is the average number of events recorded in that time window
    /// across all partitions. This shows how the overall per-agent throughput
    /// evolves over time without exposing individual agent detail.
    ///
    /// Note: windows where only some partitions have data are averaged over
    /// however many partitions contributed to that window.
    pub trend: Vec<f64>,
    /// Duration of each trend window (e.g. "10s")
    pub window_duration: String,
}

/// Statistics for a metric where multiple reading agents observe the chain head of multiple
/// writing agents. The reading dimension is collapsed by taking the maximum observation per
/// writing agent (any successful read counts as propagation), then the writing dimension
/// is summarised with aggregate statistics.
///
/// Used for `chain_len` (must_get_agent_activity scenarios) and
/// `highest_observed_action_seq` (get_agent_activity scenarios).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChainHeadStats {
    /// Mean of the maximum chain head value observed across all write agents.
    ///
    /// Represents how far write agents' chains grew and became observable to readers.
    /// Computed as the mean of `max(observed_value)` per write agent.
    pub mean_max: f64,
    /// Highest chain head value observed for any single write agent.
    pub max: f64,
    /// Number of distinct write agents observed.
    pub write_agent_count: usize,
    /// Per-window mean of per-write-agent maximum chain head values, in chronological order.
    ///
    /// Each element is: for that time window, take max(observed_value) per write agent,
    /// then average those maxes across write agents. Shows how chain advancement progressed
    /// over the run — a flat or slowing trend indicates writing stalled or readers caught up.
    pub trend: Float64Trend,
}

/// Aggregate statistics for a counter metric partitioned by tag (e.g. per agent).
///
/// Per-partition data is not retained. Instead, aggregate statistics across partitions
/// are reported: scale, breadth, and worst-case — following the same pattern as host
/// metrics (e.g. `hosts_above_80_percent` rather than per-host CPU values).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PartitionedCounterStats {
    /// Total event count summed across all partitions over the run
    pub total_count: u64,
    /// Number of distinct partitions (e.g. agents) observed in the data
    pub partition_count: usize,
    /// Number of partitions that recorded at least one event.
    ///
    /// Useful for understanding the mean value because partitions with a 0 value reduce the mean.
    pub partitions_above_zero: usize,
    /// Mean count per partition, rounded to the nearest whole number.
    ///
    /// Equivalent to `total_count / partition_count` (0 when partition_count is 0).
    /// Provided as a pre-computed convenience.
    pub mean_count: u64,
    /// Highest count recorded in any single partition.
    ///
    /// For error metrics: identifies concentrated vs distributed failure.
    /// For volume metrics: detects load imbalance — compare with mean_count.
    pub max_per_partition: u64,
    /// Lowest count recorded in any single partition.
    ///
    /// For volume metrics: detects underperforming agents — compare with mean_count and
    /// max_per_partition to assess spread.
    pub min_per_partition: u64,
    /// Mean delta per window across partitions, in chronological order (4 d.p. precision).
    ///
    /// Each element is the average number of new events in that time window across all
    /// partitions. Counter resets within a window are clamped to zero. Useful for
    /// spotting whether errors or events are concentrated at a particular point in time.
    ///
    /// 4 d.p. precision is used so that low-frequency counters (e.g. rare error events
    /// averaging < 0.01 per window) are not collapsed to zero.
    pub trend: Vec<f64>,
    /// Duration of each trend window (e.g. "10s")
    pub window_duration: String,
}
