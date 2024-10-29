use crate::frame::LoadError;
use anyhow::Context;
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
