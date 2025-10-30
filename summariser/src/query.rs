pub mod holochain_metrics;
pub mod host_metrics;

use std::collections::BTreeMap;

use crate::analyze::{counter_stats, gauge_stats, standard_timing_stats};
use crate::model::{CounterStats, GaugeStats, StandardTimingsStats};
use crate::partition::{partition_by_tags, Partition};
use crate::{
    frame::LoadError,
    query::host_metrics::{HostMetricField, Values as _},
};
use anyhow::Context;
use chrono::DateTime;
use influxdb::ReadQuery;
use polars::frame::DataFrame;
use wind_tunnel_summary_model::RunSummary;

pub async fn query_instrument_data(
    client: influxdb::Client,
    summary: &RunSummary,
    operation_id: &str,
) -> anyhow::Result<DataFrame> {
    const TABLE: &str = "wt.instruments.operation_duration";
    let q = ReadQuery::new(format!(
        r#"SELECT value FROM "windtunnel"."autogen"."{TABLE}" WHERE run_id = '{}' AND operation_id = '{}' AND is_error = 'false'"#,
        summary.run_id, operation_id
    ));
    log::debug!("Querying: {q:?}");

    #[cfg(feature = "query_test_data")]
    if cfg!(feature = "query_test_data") {
        return crate::frame::parse_time_column(super::test_data::load_query_result(&q)?);
    }

    let res = client.json_query(q.clone()).await?;
    let frame = crate::frame::load_from_response(TABLE, res)?;

    #[cfg(feature = "test_data")]
    let frame = {
        let mut frame = frame;
        crate::test_data::insert_query_result(&q, &mut frame)?;
        frame
    };

    log::trace!("Loaded frame: {frame}");

    Ok(frame)
}

pub async fn query_zome_call_instrument_data(
    client: influxdb::Client,
    summary: &RunSummary,
) -> anyhow::Result<DataFrame> {
    const TABLE: &str = "wt.instruments.operation_duration";
    let q = ReadQuery::new(format!(
        r#"SELECT value, zome_name, fn_name, agent FROM "windtunnel"."autogen"."{TABLE}" WHERE run_id = '{}' AND (operation_id = 'app_call_zome' OR operation_id = 'trycp_app_call_zome') AND is_error = 'false'"#,
        summary.run_id
    ));
    log::debug!("Querying: {q:?}");

    #[cfg(feature = "query_test_data")]
    if cfg!(feature = "query_test_data") {
        return crate::frame::parse_time_column(super::test_data::load_query_result(&q)?);
    }

    let res = client.json_query(q.clone()).await?;
    let frame = crate::frame::load_from_response(TABLE, res)?;

    #[cfg(feature = "test_data")]
    let frame = {
        let mut frame = frame;
        crate::test_data::insert_query_result(&q, &mut frame)?;
        frame
    };

    log::trace!("Loaded frame: {frame}");

    Ok(frame)
}

pub async fn query_zome_call_instrument_data_errors(
    client: influxdb::Client,
    summary: &RunSummary,
) -> anyhow::Result<DataFrame> {
    const TABLE: &str = "wt.instruments.operation_duration";
    let q = ReadQuery::new(format!(
        r#"SELECT value, zome_name, fn_name FROM "windtunnel"."autogen"."{TABLE}" WHERE run_id = '{}' AND (operation_id = 'app_call_zome' OR operation_id = 'trycp_app_call_zome') AND is_error = 'true'"#,
        summary.run_id
    ));
    log::debug!("Querying: {q:?}");

    #[cfg(feature = "query_test_data")]
    if cfg!(feature = "query_test_data") {
        let frame = super::test_data::load_query_result(&q);
        return match frame {
            Ok(frame) => crate::frame::parse_time_column(frame),
            Err(e) => {
                log::trace!("Failed to load test data, treating as 'no data in response': {e:?}");
                Err(LoadError::NoSeriesInResult {
                    table: TABLE.to_string(),
                    result: serde_json::Value::Null,
                }
                .into())
            }
        };
    }

    let res = client.json_query(q.clone()).await?;
    let frame = crate::frame::load_from_response(TABLE, res)?;

    #[cfg(feature = "test_data")]
    let frame = {
        let mut frame = frame;
        crate::test_data::insert_query_result(&q, &mut frame)?;
        frame
    };

    log::trace!("Loaded frame: {frame}");

    Ok(frame)
}

pub async fn query_custom_data(
    client: influxdb::Client,
    summary: &RunSummary,
    metric: &str,
    tags: &[&str],
) -> anyhow::Result<DataFrame> {
    let mut select_tags = tags.join(", ");
    if !select_tags.is_empty() {
        select_tags = format!(", {select_tags}");
    }

    let q = ReadQuery::new(format!(
        r#"SELECT value{select_tags} FROM "windtunnel"."autogen"."{metric}" WHERE run_id = '{run_id}'"#,
        run_id = summary.run_id
    ));
    log::debug!("Querying: {q:?}");

    #[cfg(feature = "query_test_data")]
    if cfg!(feature = "query_test_data") {
        return crate::frame::parse_time_column(super::test_data::load_query_result(&q)?);
    }

    let res = client.json_query(q.clone()).await?;
    let frame = crate::frame::load_from_response(metric, res)?;

    #[cfg(feature = "test_data")]
    let frame = {
        let mut frame = frame;
        crate::test_data::insert_query_result(&q, &mut frame)?;
        frame
    };

    log::trace!("Loaded frame: {frame}");

    Ok(frame)
}

pub async fn zome_call_error_count(
    client: influxdb::Client,
    summary: &RunSummary,
) -> anyhow::Result<usize> {
    match query_zome_call_instrument_data_errors(client.clone(), summary).await {
        Ok(frame) => Ok(frame.height()),
        Err(e) => match e.downcast_ref::<LoadError>() {
            Some(LoadError::NoSeriesInResult { .. }) => Ok(0),
            None => Err(e).context("Load zome call error data"),
        },
    }
}

/// Query [`DataFrame`] for any given wind-tunnel metric
///
/// Query will filter by time if `summary.run_duration` has been set.
/// Query may also filter for a specific a tag value if provided.
pub async fn query_metrics(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: &str,
    columns: &[&str],
    filter_by_tag: Option<(&str, &str)>,
) -> anyhow::Result<DataFrame> {
    let mut cols = columns.join(", ");
    if !cols.is_empty() {
        cols.insert_str(0, ", ");
    }
    let mut query_str = format!(
        r#"SELECT value{cols} FROM "windtunnel"."autogen"."{measurement}" WHERE run_id = '{run_id}'"#,
        run_id = summary.run_id
    )
    .to_string();
    // Add time filter if there is a run duration
    if let Some(run_duration) = summary.run_duration {
        let duration = std::time::Duration::from_secs(run_duration);
        let ended_at = summary.started_at.saturating_add(duration.as_secs() as i64);
        let start = DateTime::from_timestamp(summary.started_at, 0)
            .context("Failed to convert started_at to DateTime")?
            .to_rfc3339();
        let end = DateTime::from_timestamp(ended_at, 0)
            .context("Failed to convert ended_at to DateTime")?
            .to_rfc3339();
        query_str += format!(r#" AND time >= '{start}' AND time <= '{end}'"#).as_str();
    }
    // Add tag filter if provided
    if let Some((tag_name, tag_value)) = filter_by_tag {
        query_str += format!(r#" AND {tag_name} = '{tag_value}'"#).as_str();
    };

    let q = ReadQuery::new(query_str);
    log::debug!("Querying: {q:?}");

    #[cfg(feature = "query_test_data")]
    if cfg!(feature = "query_test_data") {
        return crate::frame::parse_time_column(super::test_data::load_query_result(&q)?);
    }

    let res = client.json_query(q.clone()).await?;
    let frame = crate::frame::load_from_response(measurement, res)?;
    log::debug!("Rows found: {}", frame.height());

    #[cfg(feature = "test_data")]
    let frame = {
        let mut frame = frame;
        crate::test_data::insert_query_result(&q, &mut frame)?;
        frame
    };

    log::trace!("Loaded frame: {frame}");

    Ok(frame)
}

/// Query the measurement with the filter tag and run [`standard_timing_stats()`].
pub async fn query_duration(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: &str,
    filter_tag: Option<(&str, &str)>,
) -> anyhow::Result<StandardTimingsStats> {
    let frame = query_metrics(client, summary, measurement, &[], filter_tag).await?;
    standard_timing_stats(frame, "value", "10s", None)
}

/// Query the measurement with the filter tag, then partition the data by `partitioning_tags`.
/// and run [`standard_timing_stats()`] on each partition.
pub async fn query_and_partition_duration(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: &str,
    partitioning_tags: &[&str],
    filter_tag: Option<(&str, &str)>,
) -> anyhow::Result<BTreeMap<String, StandardTimingsStats>> {
    if partitioning_tags.is_empty() {
        return Err(anyhow::anyhow!(
            "Cannot partition metric {measurement} without partitioning tags"
        ));
    }
    let data = query_metrics(client, summary, measurement, partitioning_tags, filter_tag).await?;
    let Partition::Partitioned(parts) = partition_by_tags(data, partitioning_tags)? else {
        return Err(anyhow::anyhow!("No partitions found for {measurement}"));
    };
    parts
        .into_iter()
        .map(|(tag_combination, frame)| {
            standard_timing_stats(frame, "value", "10s", None)
                .map(|analysis| (tag_combination, analysis))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()
}

/// Query the measurement with the filter tag and run `counter_stats()`.
pub async fn query_counter(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: &str,
    filter_tag: Option<(&str, &str)>,
) -> anyhow::Result<CounterStats> {
    let frame = query_metrics(client, summary, measurement, &[], filter_tag).await?;
    counter_stats(frame, "value")
}

/// Query and partition the measurement with the filter tag, then partition the data by `partitioning_tags`.
/// and run `counter_stats()` on each partition.
pub async fn query_and_partition_counter(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: &str,
    partitioning_tags: &[&str],
    filter_tag: Option<(&str, &str)>,
) -> anyhow::Result<BTreeMap<String, CounterStats>> {
    if partitioning_tags.is_empty() {
        return Err(anyhow::anyhow!(
            "Cannot partition metric {measurement} without partitioning tags"
        ));
    }
    let data = query_metrics(client, summary, measurement, partitioning_tags, filter_tag).await?;
    let Partition::Partitioned(parts) = partition_by_tags(data, partitioning_tags)? else {
        return Err(anyhow::anyhow!("No partitions found for {measurement}"));
    };
    parts
        .into_iter()
        .map(|(tag_combination, frame)| {
            counter_stats(frame, "value").map(|analysis| (tag_combination, analysis))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()
}

/// Query the measurement with the filter tag and run [`gauge_stats()`].
pub async fn query_gauge(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: &str,
    filter_tag: Option<(&str, &str)>,
) -> anyhow::Result<GaugeStats> {
    let frame = query_metrics(client, summary, measurement, &[], filter_tag).await?;
    gauge_stats(frame, "value")
}

/// Query the measurement with the filter tag, then partition the data by `partitioning_tags`
/// and run `gauge_stats()` on each partition.
pub async fn query_and_partition_gauge(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: &str,
    partitioning_tags: &[&str],
    filter_tag: Option<(&str, &str)>,
) -> anyhow::Result<BTreeMap<String, GaugeStats>> {
    if partitioning_tags.is_empty() {
        return Err(anyhow::anyhow!(
            "Cannot partition metric {measurement} without partitioning tags"
        ));
    }
    let data = query_metrics(client, summary, measurement, partitioning_tags, filter_tag).await?;
    let Partition::Partitioned(parts) = partition_by_tags(data, partitioning_tags)? else {
        return Err(anyhow::anyhow!("No partitions found for {measurement}"));
    };
    parts
        .into_iter()
        .map(|(tag_combination, frame)| {
            gauge_stats(frame, "value").map(|analysis| (tag_combination, analysis))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()
}

/// Query [`DataFrame`] for the given [`HostMetricField`].
pub async fn query_host_metrics(
    client: &influxdb::Client,
    summary: &RunSummary,
    field: HostMetricField,
) -> anyhow::Result<DataFrame> {
    let select_filter = if let Some(run_duration) = summary.run_duration {
        host_metrics::SelectFilter::TimeInterval {
            started_at: summary.started_at,
            duration: std::time::Duration::from_secs(run_duration),
            run_id: summary.run_id.clone(),
        }
    } else {
        host_metrics::SelectFilter::RunId(summary.run_id.clone())
    };

    let query = ReadQuery::new(host_metrics_query(field, &select_filter).context("Select query")?);
    log::debug!("Querying field {field}: {query:?}");

    #[cfg(feature = "query_test_data")]
    if cfg!(feature = "query_test_data") {
        return crate::frame::parse_time_column(crate::test_data::load_query_result(&query)?);
    }

    let res = client.json_query(query.clone()).await?;
    let frame =
        crate::frame::load_from_response(field.measurement(), res).context("Empty query result")?;
    log::trace!("Loaded frame for {field}: {frame:?}");

    #[cfg(feature = "test_data")]
    let frame = {
        let mut frame = frame;
        crate::test_data::insert_query_result(&query, &mut frame)?;
        frame
    };

    Ok(frame)
}

/// Get SELECT query for a [`host_metrics::HostMetricField`].
///
/// Given a [`host_metrics::HostMetricField`], it returns the select statement for the field and the relative timestamp
fn host_metrics_query(
    field: host_metrics::HostMetricField,
    filter: &host_metrics::SelectFilter,
) -> anyhow::Result<String> {
    let values = field.values().join(",");

    match filter {
        host_metrics::SelectFilter::RunId(run_id) => Ok(format!(
            r#"SELECT {values},time
            FROM "windtunnel"."autogen"."{table}"
            WHERE run_id = '{run_id}'
    "#,
            table = field.measurement()
        )),
        host_metrics::SelectFilter::TimeInterval {
            started_at,
            duration,
            run_id,
        } => {
            let ended_at = started_at.saturating_add(duration.as_secs() as i64);

            let start_datetime = DateTime::from_timestamp(*started_at, 0)
                .context("Failed to convert started_at to DateTime")?
                .to_rfc3339();
            let end_datetime = DateTime::from_timestamp(ended_at, 0)
                .context("Failed to convert ended_at to DateTime")?
                .to_rfc3339();

            Ok(format!(
                r#"SELECT {values},time
                FROM "windtunnel"."autogen"."{table}"
                WHERE run_id = '{run_id}' AND time >= '{start_datetime}' AND time <= '{end_datetime}'
                "#,
                table = field.measurement()
            ))
        }
    }
}

#[cfg(test)]
mod tests {

    use std::time::Duration;

    use crate::query::host_metrics::{HostMetricField, NetField, SelectFilter};

    use super::*;

    #[test]
    fn test_should_get_query_with_run_id_filter() {
        let field = HostMetricField::Net(NetField::BytesRecv);

        let query = host_metrics_query(field, &SelectFilter::RunId("test_run_id".to_string()))
            .expect("Failed to build query");
        assert_eq!(
            query,
            r#"SELECT interface,bytes_recv,time
            FROM "windtunnel"."autogen"."net"
            WHERE run_id = 'test_run_id'
    "#,
        );
    }

    #[test]
    fn test_should_get_query_with_time_filter() {
        let field = HostMetricField::Net(NetField::BytesRecv);

        let query = host_metrics_query(
            field,
            &SelectFilter::TimeInterval {
                started_at: 1756301266, // 2025-08-27 01:27:46
                duration: Duration::from_secs(300),
                run_id: "test_run_id".to_string(),
            },
        )
        .expect("Failed to build query");

        assert_eq!(
            query,
            r#"SELECT interface,bytes_recv,time
                FROM "windtunnel"."autogen"."net"
                WHERE run_id = 'test_run_id' AND time >= '2025-08-27T13:27:46+00:00' AND time <= '2025-08-27T13:32:46+00:00'
                "#,
        );
    }
}
