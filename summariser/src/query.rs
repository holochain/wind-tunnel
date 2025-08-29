pub mod host_metrics;

use crate::{frame::LoadError, query::host_metrics::Values as _};
use anyhow::Context;
use chrono::DateTime;
use influxdb::ReadQuery;
use polars::frame::DataFrame;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wind_tunnel_summary_model::RunSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueryResult {
    statement_id: u32,
    series: Vec<QuerySeries>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QuerySeries {
    name: String,
    columns: Vec<String>,
    values: Vec<Vec<Value>>,
}

pub async fn query_instrument_data(
    client: influxdb::Client,
    summary: &RunSummary,
    operation_id: &str,
) -> anyhow::Result<DataFrame> {
    let q = ReadQuery::new(format!(
        r#"SELECT value FROM "windtunnel"."autogen"."wt.instruments.operation_duration" WHERE run_id = '{}' AND operation_id = '{}' AND is_error = 'false'"#,
        summary.run_id, operation_id
    ));
    log::debug!("Querying: {:?}", q);

    #[cfg(feature = "query_test_data")]
    if cfg!(feature = "query_test_data") {
        return crate::frame::parse_time_column(super::test_data::load_query_result(&q)?);
    }

    let res = client.json_query(q.clone()).await?;
    let frame = crate::frame::load_from_response(res)?;

    #[cfg(feature = "test_data")]
    let frame = {
        let mut frame = frame;
        crate::test_data::insert_query_result(&q, &mut frame)?;
        frame
    };

    log::trace!("Loaded frame: {}", frame);

    Ok(frame)
}

pub async fn query_zome_call_instrument_data(
    client: influxdb::Client,
    summary: &RunSummary,
) -> anyhow::Result<DataFrame> {
    let q = ReadQuery::new(format!(
        r#"SELECT value, zome_name, fn_name, agent FROM "windtunnel"."autogen"."wt.instruments.operation_duration" WHERE run_id = '{}' AND (operation_id = 'app_call_zome' OR operation_id = 'trycp_app_call_zome') AND is_error = 'false'"#,
        summary.run_id
    ));
    log::debug!("Querying: {:?}", q);

    #[cfg(feature = "query_test_data")]
    if cfg!(feature = "query_test_data") {
        return crate::frame::parse_time_column(super::test_data::load_query_result(&q)?);
    }

    let res = client.json_query(q.clone()).await?;
    let frame = crate::frame::load_from_response(res)?;

    #[cfg(feature = "test_data")]
    let frame = {
        let mut frame = frame;
        crate::test_data::insert_query_result(&q, &mut frame)?;
        frame
    };

    log::trace!("Loaded frame: {}", frame);

    Ok(frame)
}

pub async fn query_zome_call_instrument_data_errors(
    client: influxdb::Client,
    summary: &RunSummary,
) -> anyhow::Result<DataFrame> {
    let q = ReadQuery::new(format!(
        r#"SELECT value, zome_name, fn_name FROM "windtunnel"."autogen"."wt.instruments.operation_duration" WHERE run_id = '{}' AND (operation_id = 'app_call_zome' OR operation_id = 'trycp_app_call_zome') AND is_error = 'true'"#,
        summary.run_id
    ));
    log::debug!("Querying: {:?}", q);

    #[cfg(feature = "query_test_data")]
    if cfg!(feature = "query_test_data") {
        let frame = super::test_data::load_query_result(&q);
        return match frame {
            Ok(frame) => crate::frame::parse_time_column(frame),
            Err(e) => {
                log::trace!(
                    "Failed to load test data, treating as 'no data in response': {:?}",
                    e
                );
                Err(LoadError::NoSeriesInResult {
                    result: serde_json::Value::Null,
                }
                .into())
            }
        };
    }

    let res = client.json_query(q.clone()).await?;
    let frame = crate::frame::load_from_response(res)?;

    #[cfg(feature = "test_data")]
    let frame = {
        let mut frame = frame;
        crate::test_data::insert_query_result(&q, &mut frame)?;
        frame
    };

    log::trace!("Loaded frame: {}", frame);

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
        select_tags = format!(", {}", select_tags);
    }

    let q = ReadQuery::new(format!(
        r#"SELECT value{} FROM "windtunnel"."autogen"."{}" WHERE run_id = '{}'"#,
        select_tags, metric, summary.run_id
    ));
    log::debug!("Querying: {:?}", q);

    #[cfg(feature = "query_test_data")]
    if cfg!(feature = "query_test_data") {
        return crate::frame::parse_time_column(super::test_data::load_query_result(&q)?);
    }

    let res = client.json_query(q.clone()).await?;
    let frame = crate::frame::load_from_response(res)?;

    #[cfg(feature = "test_data")]
    let frame = {
        let mut frame = frame;
        crate::test_data::insert_query_result(&q, &mut frame)?;
        frame
    };

    log::trace!("Loaded frame: {}", frame);

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

/// Get SELECT query for a [`host_metrics::HostMetricField`].
///
/// Given a [`host_metrics::HostMetricField`], it returns the select statement for the field and the relative timestamp
pub fn host_metrics_query(
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
