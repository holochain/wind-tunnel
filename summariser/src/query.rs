use crate::frame;
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
    let res = client.json_query(q).await?;
    frame::load_from_response(res)
}

pub async fn query_zome_call_instrument_data(
    client: influxdb::Client,
    summary: &RunSummary,
) -> anyhow::Result<DataFrame> {
    let q = ReadQuery::new(format!(
        r#"SELECT value, zome_name, fn_name FROM "windtunnel"."autogen"."wt.instruments.operation_duration" WHERE run_id = '{}' AND (operation_id = 'app_call_zome' OR operation_id = 'trycp_app_call_zome') AND is_error = 'false'"#,
        summary.run_id
    ));
    let res = client.json_query(q).await?;
    frame::load_from_response(res)
}

pub async fn query_zome_call_instrument_data_errors(
    client: influxdb::Client,
    summary: &RunSummary,
) -> anyhow::Result<DataFrame> {
    let q = ReadQuery::new(format!(
        r#"SELECT value, zome_name, fn_name FROM "windtunnel"."autogen"."wt.instruments.operation_duration" WHERE run_id = '{}' AND (operation_id = 'app_call_zome' OR operation_id = 'trycp_app_call_zome') AND is_error = 'true'"#,
        summary.run_id
    ));
    let res = client.json_query(q).await?;
    frame::load_from_response(res)
}

pub async fn query_custom_data(
    client: influxdb::Client,
    summary: &RunSummary,
    metric: &str,
) -> anyhow::Result<DataFrame> {
    let q = ReadQuery::new(format!(
        r#"SELECT value FROM "windtunnel"."autogen"."{}" WHERE run_id = '{}'"#,
        metric, summary.run_id
    ));
    log::debug!("Querying: {:?}", q);
    let res = client.json_query(q).await?;
    frame::load_from_response(res)
}
