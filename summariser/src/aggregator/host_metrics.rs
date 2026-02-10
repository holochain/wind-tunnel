use std::collections::{BTreeMap, HashMap, HashSet};

use polars::frame::{DataFrame, UniqueKeepStrategy};
use polars::prelude::{IntoLazy, col, lit};
use wind_tunnel_summary_model::RunSummary;

use crate::analyze::{counter_stats, standard_timing_stats};
use crate::model::{CpuMetrics, HostMetrics, MemMetrics, NetMetrics};
use crate::query::host_metrics::{
    self as host_metrics_query, Column as _, CpuField, HostMetricField, NetField, TAG_INTERFACE,
};
use crate::query::query_host_metrics;

/// The host metrics aggregator takes care of collecting the [`HostMetrics`] and aggregate them accordingly.
///
/// It is basically a helper around the query and analyze modules for host metrics, summarizing the data
/// and providing a unified interface for accessing it.
pub struct HostMetricsAggregator<'a> {
    client: &'a influxdb::Client,
    summary: &'a RunSummary,
}

impl<'a> HostMetricsAggregator<'a> {
    /// Create a new host metrics aggregator.
    pub fn new(client: &'a influxdb::Client, summary: &'a RunSummary) -> Self {
        Self { client, summary }
    }
}

impl HostMetricsAggregator<'_> {
    /// Try to aggregate host metrics.
    ///
    /// It returns an [`Option`] of [`HostMetrics`]. If it fails to collect metrics it returns [`None`] and
    /// reports the error in the logs.
    pub async fn try_aggregate(&self) -> Option<HostMetrics> {
        match self.aggregate().await {
            Ok(metrics) => Some(metrics),
            Err(e) => {
                log::error!("Failed to aggregate host metrics: {e}");
                None
            }
        }
    }

    /// Aggregate all [`HostMetrics`] and return them.
    pub async fn aggregate(&self) -> anyhow::Result<HostMetrics> {
        log::debug!("Aggregating host metrics for run {}", self.summary.run_id);

        Ok(HostMetrics {
            cpu: self.aggregate_cpu_metrics().await?,
            memory: self.aggregate_mem_metrics().await?,
            network: self.aggregate_net_metrics().await?,
        })
    }

    /// Aggregate the [`CpuMetrics`] by core.
    async fn aggregate_cpu_metrics(&self) -> anyhow::Result<CpuMetrics> {
        log::debug!("Aggregating CPU metrics");
        // get cpu metrics
        let usage_system = self
            .query_and_analyze(
                HostMetricField::Cpu(CpuField::UsageSystem),
                |frame, column| standard_timing_stats(frame, column, "10s", None),
            )
            .await?;
        let usage_user = self
            .query_and_analyze(
                HostMetricField::Cpu(CpuField::UsageUser),
                |frame, column| standard_timing_stats(frame, column, "10s", None),
            )
            .await?;

        Ok(CpuMetrics {
            usage_user,
            usage_system,
        })
    }

    /// Aggregate the [`MemMetrics`]
    async fn aggregate_mem_metrics(&self) -> anyhow::Result<MemMetrics> {
        log::debug!("Aggregating Memory metrics");
        let active = self
            .query_and_analyze(
                HostMetricField::Mem(host_metrics_query::MemField::Active),
                |frame, column| standard_timing_stats(frame, column, "10s", None),
            )
            .await?;
        let available = self
            .query_and_analyze(
                HostMetricField::Mem(host_metrics_query::MemField::Available),
                |frame, column| standard_timing_stats(frame, column, "10s", None),
            )
            .await?;
        let available_percent = self
            .query_and_analyze(
                HostMetricField::Mem(host_metrics_query::MemField::AvailablePercent),
                |frame, column| standard_timing_stats(frame, column, "10s", None),
            )
            .await?;
        let free = self
            .query_and_analyze(
                HostMetricField::Mem(host_metrics_query::MemField::Free),
                |frame, column| standard_timing_stats(frame, column, "10s", None),
            )
            .await?;
        let inactive = self
            .query_and_analyze(
                HostMetricField::Mem(host_metrics_query::MemField::Inactive),
                |frame, column| standard_timing_stats(frame, column, "10s", None),
            )
            .await?;
        let swap_free = self
            .query_and_analyze(
                HostMetricField::Mem(host_metrics_query::MemField::SwapFree),
                |frame, column| standard_timing_stats(frame, column, "10s", None),
            )
            .await?;
        let swap_total = self
            .query_and_analyze(
                HostMetricField::Mem(host_metrics_query::MemField::SwapTotal),
                |frame, column| standard_timing_stats(frame, column, "10s", None),
            )
            .await?;
        let total = self
            .query_and_analyze(
                HostMetricField::Mem(host_metrics_query::MemField::Total),
                |frame, column| standard_timing_stats(frame, column, "10s", None),
            )
            .await?;
        let used = self
            .query_and_analyze(
                HostMetricField::Mem(host_metrics_query::MemField::Used),
                |frame, column| standard_timing_stats(frame, column, "10s", None),
            )
            .await?;
        let used_percent = self
            .query_and_analyze(
                HostMetricField::Mem(host_metrics_query::MemField::UsedPercent),
                |frame, column| standard_timing_stats(frame, column, "10s", None),
            )
            .await?;

        Ok(MemMetrics {
            active,
            available,
            available_percent,
            free,
            inactive,
            swap_free,
            swap_total,
            total,
            used,
            used_percent,
        })
    }

    /// Aggregate the [`NetMetrics`] by interface.
    async fn aggregate_net_metrics(&self) -> anyhow::Result<NetMetrics> {
        log::debug!("Aggregating Network metrics");
        let bytes_recv = self
            .query_and_aggregate(
                HostMetricField::Net(NetField::BytesRecv),
                TAG_INTERFACE,
                counter_stats,
            )
            .await?;
        let bytes_sent = self
            .query_and_aggregate(
                HostMetricField::Net(NetField::BytesSent),
                TAG_INTERFACE,
                counter_stats,
            )
            .await?;
        let packets_recv = self
            .query_and_aggregate(
                HostMetricField::Net(NetField::PacketsRecv),
                TAG_INTERFACE,
                counter_stats,
            )
            .await?;
        let packets_sent = self
            .query_and_aggregate(
                HostMetricField::Net(NetField::PacketsSent),
                TAG_INTERFACE,
                counter_stats,
            )
            .await?;

        Ok(NetMetrics {
            bytes_recv,
            bytes_sent,
            packets_recv,
            packets_sent,
        })
    }

    /// Query the [`DataFrame`] for the given [`HostMetricField`] and analyze it.
    ///
    /// Stats are collected by a function `collect_stats` which takes (`data_frame`, `field_column`)
    /// and returns a `anyhow::Result<T>` stats.
    async fn query_and_analyze<F, T>(
        &self,
        field: HostMetricField,
        collect_stats: F,
    ) -> anyhow::Result<T>
    where
        F: Fn(DataFrame, &str) -> anyhow::Result<T>,
    {
        let data = query_host_metrics(self.client, self.summary, field).await?;
        match collect_stats(data, field.column()) {
            Ok(stats) => Ok(stats),
            Err(e) => {
                log::warn!("Failed to collect stats for {field}: {e}");
                Err(e)
            }
        }
    }

    /// Query the [`DataFrame`] for the given [`HostMetricField`] and aggregate them by `tag` into a [`HashMap`] where the key
    /// is the tag value and the analyzed metrics as `T` stats.
    ///
    /// Stats are collected by a function `collect_stats` which takes (`data_frame`, `field_column`)
    /// and returns a `anyhow::Result<T>` stats.
    async fn query_and_aggregate<F, T>(
        &self,
        field: HostMetricField,
        tag: &str,
        collect_stats: F,
    ) -> anyhow::Result<BTreeMap<String, T>>
    where
        F: Fn(DataFrame, &str) -> anyhow::Result<T>,
    {
        let data = query_host_metrics(self.client, self.summary, field).await?;

        Ok(self
            .aggregate_data_frame_by_tag(data, tag)
            .await?
            .into_iter()
            .flat_map(
                |(tag_value, frame)| match collect_stats(frame, field.column()) {
                    Ok(stats) => Some((tag_value, stats)),
                    Err(e) => {
                        log::warn!("Failed to collect stats for {field} ({tag}={tag_value}): {e}");
                        None
                    }
                },
            )
            .collect::<BTreeMap<_, _>>())
    }

    /// Aggregate the [`DataFrame`] by tag.
    ///
    /// Given a [`DataFrame`] and a tag, it returns all the sub
    async fn aggregate_data_frame_by_tag(
        &self,
        data_frame: DataFrame,
        tag: &str,
    ) -> anyhow::Result<HashMap<String, DataFrame>> {
        let aggregators = data_frame
            .clone()
            .lazy()
            .select([col(tag)])
            .unique(Some(vec![tag.to_string()]), UniqueKeepStrategy::Any)
            .collect()?;

        let keys: HashSet<&str> = aggregators.column(tag)?.str()?.iter().flatten().collect();

        let mut aggregated = HashMap::with_capacity(keys.len());
        for key in keys {
            log::debug!("Aggregating for {tag}={key}");

            let filtered = data_frame
                .clone()
                .lazy()
                .select([col("*")])
                .filter(col(tag).eq(lit(key)))
                .collect()?;
            aggregated.insert(key.to_string(), filtered);
        }

        Ok(aggregated)
    }
}
