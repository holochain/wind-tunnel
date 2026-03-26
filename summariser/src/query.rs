pub mod holochain_metrics;
pub mod host_metrics;

use crate::analyze::{counter_stats, gauge_stats, histogram_timing_stats};
use crate::model::{CounterStats, GaugeStats, StandardTimingsStats};
use crate::{
    frame::LoadError,
    query::host_metrics::{HostMetricMeasurement, InfluxSourced as _},
};
use anyhow::Context;
use chrono::DateTime;
use influxdb::ReadQuery;
use itertools::Itertools;
use polars::frame::DataFrame;
use polars::prelude::*;
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
    let mut select_columns: Vec<&str> = vec!["value"];
    select_columns.extend_from_slice(tags);
    let select = select_columns.join(", ");

    let q = ReadQuery::new(format!(
        r#"SELECT {select} FROM "windtunnel"."autogen"."{metric}" WHERE run_id = '{run_id}'"#,
        run_id = summary.run_id
    ));
    log::debug!("Querying: {q:?}");

    #[cfg(feature = "query_test_data")]
    if cfg!(feature = "query_test_data") {
        return crate::frame::parse_time_column(super::test_data::load_query_result(&q).map_err(
            |_| {
                log::debug!("Failed to load test data query result for query: {q:?}");
                LoadError::NoSeriesInResult {
                    table: metric.to_string(),
                    result: serde_json::Value::Null,
                }
            },
        )?);
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

/// Query [`DataFrame`] for an OTel-style metric with explicit field names.
///
/// Unlike [`query_metrics`] which always selects `value`, this function selects the
/// specified `fields` (e.g. `["count", "sum", "min", "max"]` for histograms,
/// `["gauge"]` for gauges, `["sum"]` for counters). Extra tag `columns` are appended
/// after the fields.
pub async fn query_metrics_fields(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: &str,
    fields: &[&str],
    columns: &[&str],
    filter_by_tag: Option<(&str, &str)>,
) -> anyhow::Result<DataFrame> {
    // Double-quote field and column names so that InfluxQL reserved words
    // (e.g. "count", "sum") are treated as field references, not functions.
    let select_clause = fields
        .iter()
        .chain(columns.iter())
        .map(|name| format!(r#""{name}""#))
        .join(", ");
    execute_influx_query(client, summary, measurement, &select_clause, filter_by_tag).await
}

/// Shared query execution: builds the InfluxQL string, handles test-data features,
/// executes against InfluxDB, and returns a parsed DataFrame.
async fn execute_influx_query(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: &str,
    select_clause: &str,
    filter_by_tag: Option<(&str, &str)>,
) -> anyhow::Result<DataFrame> {
    let mut query_str = format!(
        r#"SELECT {select_clause} FROM "windtunnel"."autogen"."{measurement}" WHERE run_id = '{run_id}'"#,
        run_id = summary.run_id
    );
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
        // The tag name is wrapped in double quote marks so it does not conflict with influxql syntax keywords.
        query_str += format!(r#" AND "{tag_name}" = '{tag_value}'"#).as_str();
    };

    let q = ReadQuery::new(&query_str);
    log::debug!("Querying: {q:?}");

    #[cfg(feature = "query_test_data")]
    if cfg!(feature = "query_test_data") {
        return crate::frame::parse_time_column(
            // If we cannot load the test data file, we return a NoSeriesInResult error,
            // so that callers behave the same as receiving empty results from an influxdb query.
            super::test_data::load_query_result(&q).map_err(|_| {
                log::debug!("Failed to load test data query result for query: {q:?}");

                LoadError::NoSeriesInResult {
                    table: measurement.to_string(),
                    result: serde_json::Value::Null,
                }
            })?,
        );
    }

    let res = match client.json_query(q.clone()).await {
        Ok(res) => res,
        Err(influxdb::Error::DeserializationError { error: deser_err }) => {
            // json_query failed to parse the response. Retry with the raw query to recover the
            // actual response body, which is consumed internally by json_query before the error
            // is returned and therefore not available directly.
            let raw_res = client.query(q.clone()).await;
            let raw = match raw_res {
                Ok(body) => body,
                Err(retry_err) => {
                    return Err(anyhow::anyhow!(
                        "InfluxDB query '{}' failed to deserialize (json_query error: {}); \
                         raw retry also failed: {}",
                        query_str,
                        deser_err,
                        retry_err
                    ));
                }
            };
            const MAX_RAW_LEN: usize = 4096;
            let truncated_raw = if raw.len() > MAX_RAW_LEN {
                let mut end = MAX_RAW_LEN;
                while !raw.is_char_boundary(end) {
                    end -= 1;
                }
                format!("{}...(truncated)", &raw[..end])
            } else {
                raw.clone()
            };
            serde_json::from_str::<influxdb::integrations::serde_integration::DatabaseQueryResult>(
                &raw,
            )
            .map_err(|_| {
                anyhow::anyhow!(
                    "InfluxDB returned a non-JSON response for query '{}' \
                     (json_query error: {}). Raw response: {:?}",
                    query_str,
                    deser_err,
                    truncated_raw
                )
            })?
        }
        Err(e) => return Err(anyhow::Error::from(e)),
    };
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

// ---------------------------------------------------------------------------
// OTel metric query helpers — for pre-aggregated histogram / gauge / counter
// ---------------------------------------------------------------------------

/// Query a pre-aggregated OTel histogram and compute [`StandardTimingsStats`].
///
/// Selects the `count`, `sum`, `min`, `max` fields emitted by the OTel SDK.
pub async fn query_histogram_duration(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: &str,
    filter_tag: Option<(&str, &str)>,
) -> anyhow::Result<StandardTimingsStats> {
    let frame = query_metrics_fields(
        client,
        summary,
        measurement,
        &["count", "sum", "min", "max"],
        &[],
        filter_tag,
    )
    .await?;
    histogram_timing_stats(frame, "10s")
}

/// Query an OTel gauge metric (field name `"gauge"` rather than `"value"`).
pub async fn query_otel_gauge(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: &str,
    filter_tag: Option<(&str, &str)>,
    window_duration: &str,
) -> anyhow::Result<GaugeStats> {
    let frame =
        query_metrics_fields(client, summary, measurement, &["gauge"], &[], filter_tag).await?;
    gauge_stats(frame, "gauge", window_duration)
}

/// Query an OTel counter metric (field name `"sum"`, potentially UInt64).
pub async fn query_otel_counter(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: &str,
    filter_tag: Option<(&str, &str)>,
    window_duration: &str,
) -> anyhow::Result<CounterStats> {
    let frame =
        query_metrics_fields(client, summary, measurement, &["sum"], &[], filter_tag).await?;
    counter_stats(frame, "sum", window_duration)
}

/// Return the total observation count from an OTel histogram by summing `count`.
pub async fn query_histogram_total_count(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: &str,
    filter_tag: Option<(&str, &str)>,
) -> anyhow::Result<usize> {
    let frame =
        query_metrics_fields(client, summary, measurement, &["count"], &[], filter_tag).await?;
    let total: u64 = frame
        .column("count")
        .ok()
        .and_then(|c| c.as_materialized_series().cast(&DataType::UInt64).ok())
        .and_then(|s| s.u64().ok().map(|ca| ca.sum().unwrap_or(0)))
        .unwrap_or(0);
    Ok(total as usize)
}

/// Query [`DataFrame`] for the given [`HostMetricMeasurement`].
pub async fn query_host_metrics(
    client: &influxdb::Client,
    summary: &RunSummary,
    measurement: HostMetricMeasurement,
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

    let query = ReadQuery::new(
        build_host_metrics_query(measurement.clone(), &select_filter).context("Select query")?,
    );
    log::debug!("Querying field {measurement:?}: {query:?}");

    #[cfg(feature = "query_test_data")]
    if cfg!(feature = "query_test_data") {
        return crate::frame::parse_time_column(crate::test_data::load_query_result(&query)?);
    }

    let res = client.json_query(query.clone()).await?;
    let frame = crate::frame::load_from_response(measurement.measurement(), res)
        .context("Empty query result")?;
    log::trace!("Loaded frame for {measurement:?}: {frame:?}");

    #[cfg(feature = "test_data")]
    let frame = {
        let mut frame = frame;
        crate::test_data::insert_query_result(&query, &mut frame)?;
        frame
    };

    Ok(frame)
}

/// Build a SELECT query for a [`host_metrics::HostMetricMeasurement`].
///
/// Given a [`host_metrics::HostMetricMeasurement`], it returns the select statement for the field and the relative timestamp
fn build_host_metrics_query(
    measurement: HostMetricMeasurement,
    filter: &host_metrics::SelectFilter,
) -> anyhow::Result<String> {
    // Quote all identifiers to avoid collisions with InfluxQL reserved words
    // (e.g. `name` in diskio). InfluxDB returns column names unquoted in the
    // response regardless of quoting in the query.
    let values = measurement
        .select()
        .iter()
        .map(|v| format!("\"{v}\""))
        .join(",");

    let mut filter_tags = measurement
        .filter_tags()
        .iter()
        .map(|(k, v)| format!(r#""{k}" = '{v}'"#))
        .join(" AND ");
    if !filter_tags.is_empty() {
        filter_tags += " AND ";
    }

    match filter {
        host_metrics::SelectFilter::RunId(run_id) => Ok(format!(
            r#"SELECT {values},time
            FROM "windtunnel"."autogen"."{table}"
            WHERE {filter_tags}run_id = '{run_id}'
    "#,
            table = measurement.measurement(),
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
                WHERE {filter_tags}run_id = '{run_id}' AND time >= '{start_datetime}' AND time <= '{end_datetime}'
                "#,
                table = measurement.measurement()
            ))
        }
    }
}

#[cfg(test)]
mod tests {

    use std::time::Duration;

    use crate::query::host_metrics::{HostMetricMeasurement, NetFieldSet, SelectFilter};

    use super::*;

    #[test]
    fn test_should_get_query_with_run_id_filter() {
        let field = HostMetricMeasurement::Net(NetFieldSet::Default);

        let query =
            build_host_metrics_query(field, &SelectFilter::RunId("test_run_id".to_string()))
                .expect("Failed to build query");
        assert_eq!(
            query,
            r#"SELECT "host","interface","bytes_recv","bytes_sent","packets_recv","packets_sent",time
            FROM "windtunnel"."autogen"."net"
            WHERE run_id = 'test_run_id'
    "#,
        );
    }

    #[test]
    fn test_should_get_query_with_time_filter() {
        let field = HostMetricMeasurement::Net(NetFieldSet::Default);

        let query = build_host_metrics_query(
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
            r#"SELECT "host","interface","bytes_recv","bytes_sent","packets_recv","packets_sent",time
                FROM "windtunnel"."autogen"."net"
                WHERE run_id = 'test_run_id' AND time >= '2025-08-27T13:27:46+00:00' AND time <= '2025-08-27T13:32:46+00:00'
                "#,
        );
    }
}
