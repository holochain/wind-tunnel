use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Duration;

use anyhow::Context;
use influxdb::ReadQuery;
use polars::frame::{DataFrame, UniqueKeepStrategy};
use polars::prelude::{col, lit, IntoLazy};
use wind_tunnel_summary_model::RunSummary;

use crate::analyze::{counter_stats, gauge_stats};
use crate::model::{CpuMetrics, HostMetrics, MemMetrics, NetMetrics};
use crate::query::host_metrics::{
    self as host_metrics_query, Column as _, CpuField, HostMetricField, NetField, SelectFilter,
    TAG_INTERFACE,
};
use crate::query::host_metrics_query;

/// The host metrics aggregator takes care of collecting the [`HostMetrics`] and aggregate them accordingly.
///
/// It is basically a helper around the query and analyze modules for host metrics, summarizing the data
/// and providing a unified interface for accessing it.
pub struct HostMetricsAggregator<'a> {
    client: &'a influxdb::Client,
    select_filter: SelectFilter,
    summary: &'a RunSummary,
}

impl<'a> HostMetricsAggregator<'a> {
    /// Create a new host metrics aggregator.
    pub fn new(client: &'a influxdb::Client, summary: &'a RunSummary) -> Self {
        let filter = if let Some(run_duration) = summary.run_duration {
            SelectFilter::TimeInterval {
                started_at: summary.started_at,
                duration: Duration::from_secs(run_duration),
                run_id: summary.run_id.clone(),
            }
        } else {
            SelectFilter::RunId(summary.run_id.clone())
        };

        Self {
            client,
            select_filter: filter,
            summary,
        }
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
            .query_and_analyze(HostMetricField::Cpu(CpuField::UsageSystem), gauge_stats)
            .await?;
        let usage_user = self
            .query_and_analyze(HostMetricField::Cpu(CpuField::UsageUser), gauge_stats)
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
                gauge_stats,
            )
            .await?;
        let available = self
            .query_and_analyze(
                HostMetricField::Mem(host_metrics_query::MemField::Available),
                gauge_stats,
            )
            .await?;
        let available_percent = self
            .query_and_analyze(
                HostMetricField::Mem(host_metrics_query::MemField::AvailablePercent),
                gauge_stats,
            )
            .await?;
        let free = self
            .query_and_analyze(
                HostMetricField::Mem(host_metrics_query::MemField::Free),
                gauge_stats,
            )
            .await?;
        let inactive = self
            .query_and_analyze(
                HostMetricField::Mem(host_metrics_query::MemField::Inactive),
                gauge_stats,
            )
            .await?;
        let swap_free = self
            .query_and_analyze(
                HostMetricField::Mem(host_metrics_query::MemField::SwapFree),
                gauge_stats,
            )
            .await?;
        let swap_total = self
            .query_and_analyze(
                HostMetricField::Mem(host_metrics_query::MemField::SwapTotal),
                gauge_stats,
            )
            .await?;
        let total = self
            .query_and_analyze(
                HostMetricField::Mem(host_metrics_query::MemField::Total),
                gauge_stats,
            )
            .await?;
        let used = self
            .query_and_analyze(
                HostMetricField::Mem(host_metrics_query::MemField::Used),
                gauge_stats,
            )
            .await?;
        let used_percent = self
            .query_and_analyze(
                HostMetricField::Mem(host_metrics_query::MemField::UsedPercent),
                gauge_stats,
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
        T: Default,
    {
        let data = self.query(field).await?;
        Ok(match collect_stats(data, field.column()) {
            Ok(stats) => stats,
            Err(e) => {
                log::warn!("Failed to collect stats for {field}: {e}");
                T::default()
            }
        })
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
        T: Default,
    {
        let data = self.query(field).await?;

        Ok(self
            .aggregate_data_frame_by_tag(data, tag)
            .await?
            .into_iter()
            .map(|(tag_value, frame)| {
                let stats = match collect_stats(frame, field.column()) {
                    Ok(stats) => stats,
                    Err(e) => {
                        log::warn!("Failed to collect stats for {field} ({tag}={tag_value}): {e}");
                        T::default()
                    }
                };

                (tag_value, stats)
            })
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

    /// Query [`DataFrame`] for the given [`HostMetricField`].
    async fn query(&self, field: HostMetricField) -> anyhow::Result<DataFrame> {
        let query =
            ReadQuery::new(host_metrics_query(field, &self.select_filter).context("Select query")?);
        log::debug!("Querying field {field}: {query:?}");

        #[cfg(feature = "query_test_data")]
        if cfg!(feature = "query_test_data") {
            return crate::frame::parse_time_column(crate::test_data::load_query_result(&query)?);
        }

        let res = self.client.json_query(query.clone()).await?;
        let frame = crate::frame::load_from_response(res).context("Empty query result")?;
        log::trace!("Loaded frame for {field}: {frame:?}");

        #[cfg(feature = "test_data")]
        let frame = {
            let mut frame = frame;
            crate::test_data::insert_query_result(&query, &mut frame)?;
            frame
        };

        Ok(frame)
    }
}
