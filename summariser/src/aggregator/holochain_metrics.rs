use crate::analyze::{counter_stats, gauge_stats, standard_timing_stats};
use crate::frame::LoadError;
use crate::model::{
    CounterStats, GaugeStats, HolochainDatabaseMetrics, HolochainMetrics, HolochainMetricsConfig,
};
use crate::model::{HolochainDatabaseKind, StandardTimingsStats, StrumVariantSelector};
use crate::partition::partition_by_tags;
use crate::query::query_metrics;
use anyhow::Context;
use std::collections::BTreeMap;
use wind_tunnel_summary_model::RunSummary;

/// Try to aggregate Holochain metrics.
///
/// If it fails returns [`None`] and reports the error in the logs.
pub async fn try_aggregate_holochain_metrics(
    client: &influxdb::Client,
    summary: &RunSummary,
    config: HolochainMetricsConfig,
) -> Option<HolochainMetrics> {
    match aggregate_holochain_metrics(client, summary, config).await {
        Ok(metrics) => Some(metrics),
        Err(e) => {
            log::error!("Failed to aggregate holochain metrics: {e}");
            None
        }
    }
}

async fn aggregate_holochain_metrics(
    client: &influxdb::Client,
    summary: &RunSummary,
    config: HolochainMetricsConfig,
) -> anyhow::Result<HolochainMetrics> {
    log::debug!("Aggregating holochain metrics for run {}", summary.run_id);

    Ok(HolochainMetrics {
        p2p_request_duration: if !config.p2p_request_duration {
            BTreeMap::new()
        } else {
            let res = aggregate_duration(
                client,
                summary,
                "hc.holochain_p2p.request.duration.s",
                &["dna_hash"],
                None,
            )
            .await;
            match res {
                Ok(tree) => tree,
                Err(e) => match e.downcast_ref::<LoadError>() {
                    Some(LoadError::NoSeriesInResult { .. }) => BTreeMap::new(),
                    None => return Err(e).context("Aggregate p2p_request_duration"),
                },
            }
        },
        post_commit_duration: if !config.post_commit_duration {
            BTreeMap::new()
        } else {
            let res = aggregate_duration(
                client,
                summary,
                "hc.conductor.post_commit.duration",
                &["dna_hash", "agent"],
                None,
            )
            .await;
            match res {
                Ok(tree) => tree,
                Err(e) => match e.downcast_ref::<LoadError>() {
                    Some(LoadError::NoSeriesInResult { .. }) => BTreeMap::new(),
                    None => return Err(e).context("Aggregate post_commit_duration"),
                },
            }
        },
        cascade_duration: if !config.cascade_duration {
            None
        } else {
            let res = aggregate_duration(client, summary, "hc.cascade.duration.s", &[], None).await;
            match res {
                Ok(tree) => Some(
                    tree.get("")
                        .expect("None partitioned dataFrame should have empty string key")
                        .clone(),
                ),
                Err(e) => match e.downcast_ref::<LoadError>() {
                    Some(LoadError::NoSeriesInResult { .. }) => None,
                    None => return Err(e).context("Aggregate post_commit_duration"),
                },
            }
        },
        wasm_usage: if !config.wasm_usage {
            BTreeMap::new()
        } else {
            aggregate_counter(
                client,
                summary,
                "hc.ribosome.wasm.usage",
                &["dna", "agent"],
                None,
            )
            .await?
        },
        workflow_duration: {
            let mut result = BTreeMap::new();
            for v in config.workflows.clone().into_str_iter() {
                let res = aggregate_duration(
                    client,
                    summary,
                    "hc.conductor.workflow.duration.s",
                    &["dna_hash"],
                    Some(("workflow", v.as_str())),
                )
                .await?;
                result.insert(v, res);
            }
            result
        },
        database: aggregate_databases(client, summary, config.databases.clone()).await?,
    })
}

async fn aggregate_databases(
    client: &influxdb::Client,
    summary: &RunSummary,
    kinds: StrumVariantSelector<HolochainDatabaseKind>,
) -> anyhow::Result<BTreeMap<String, HolochainDatabaseMetrics>> {
    let mut result = BTreeMap::new();
    for v in kinds.into_str_iter() {
        let utilization = aggregate_gauge(
            client,
            summary,
            "hc.db.pool.utilization",
            &[],
            Some(("kind", v.as_str())),
        )
        .await?
        .get("")
        .expect("None partitioned dataFrame should have empty string key")
        .to_owned();
        let maybe_use_time = aggregate_duration(
            client,
            summary,
            "hc.db.connections.use_time.s",
            &[],
            Some(("kind", v.as_str())),
        )
        .await;
        let connection_use_time = match maybe_use_time {
            Ok(use_time) => Some(
                use_time
                    .get("")
                    .expect("None partitioned dataFrame should have empty string key")
                    .to_owned(),
            ),
            Err(e) => match e.downcast_ref::<LoadError>() {
                Some(LoadError::NoSeriesInResult { .. }) => None,
                None => return Err(e).context("Aggregate connection_use_time"),
            },
        };
        result.insert(
            v,
            HolochainDatabaseMetrics {
                utilization,
                connection_use_time,
            },
        );
    }
    Ok(result)
}

/// Query the measurement with the filter tag, then partition the data by [`partitioning_tags`]
/// and run `standard_timing_stats()` on each partition.
async fn aggregate_duration(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: &str,
    partitioning_tags: &[&str],
    filter_tag: Option<(&str, &str)>,
) -> anyhow::Result<BTreeMap<String, StandardTimingsStats>> {
    log::debug!("Aggregate duration metric {measurement}");
    let data = query_metrics(client, summary, measurement, partitioning_tags, filter_tag).await?;
    partition_by_tags(data, partitioning_tags)?
        .into_iter()
        .map(|(tag_combination, frame)| {
            standard_timing_stats(frame, "value", "10s", None)
                .map(|analysis| (tag_combination, analysis))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()
}

/// Query the measurement with the filter tag, then partition the data by [`partitioning_tags`]
/// and run `counter_stats()` on each partition.
async fn aggregate_counter(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: &str,
    partitioning_tags: &[&str],
    filter_tag: Option<(&str, &str)>,
) -> anyhow::Result<BTreeMap<String, CounterStats>> {
    log::debug!("Aggregate counter metric {measurement}");
    let data = query_metrics(client, summary, measurement, partitioning_tags, filter_tag).await?;
    partition_by_tags(data, partitioning_tags)?
        .into_iter()
        .map(|(tag_combination, frame)| {
            counter_stats(frame, "value").map(|analysis| (tag_combination, analysis))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()
}

/// Query the measurement with the filter tag, then partition the data by [`partitioning_tags`]
/// and run `gauge_stats()` on each partition.
async fn aggregate_gauge(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: &str,
    partitioning_tags: &[&str],
    filter_tag: Option<(&str, &str)>,
) -> anyhow::Result<BTreeMap<String, GaugeStats>> {
    log::debug!("Aggregate gauge metric {measurement}");
    let data = query_metrics(client, summary, measurement, partitioning_tags, filter_tag).await?;
    partition_by_tags(data, partitioning_tags)?
        .into_iter()
        .map(|(tag_combination, frame)| {
            gauge_stats(frame, "value").map(|analysis| (tag_combination, analysis))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()
}
