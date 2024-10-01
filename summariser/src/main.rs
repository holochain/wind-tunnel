use anyhow::Context;
use influxdb::{Query, ReadQuery};
use itertools::Itertools;
use std::path::PathBuf;
use chrono::{DateTime, NaiveDateTime};
use influxdb::integrations::serde_integration::DatabaseQueryResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wind_tunnel_summary_model::{load_summary_runs, RunSummary};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let summary_runs = load_summary_runs(PathBuf::from("run_summary.jsonl"))
        .expect("Failed to load run summaries");

    // Note that this is just a simple selection strategy. If we have run scenarios with more than
    // one configuration, we might want to select multiple summaries per scenario name.
    let latest_summaries = summary_runs
        .into_iter()
        .into_group_map_by(|summary| summary.scenario_name.clone())
        .into_iter()
        .map(|(_, mut summaries)| {
            summaries.sort_by_key(|summary| summary.started_at);

            // Safe to unwrap because there must have been at least one element
            summaries.last().unwrap().clone()
        })
        .collect::<Vec<_>>();

    for summary in &latest_summaries {
        println!("{:?}", summary);
    }

    let client = influxdb::Client::new(
        std::env::var("INFLUX_HOST")
            .context("Cannot read metrics without environment variable `INFLUX_HOST`")?,
        std::env::var("INFLUX_BUCKET")
            .context("Cannot read metrics without environment variable `INFLUX_BUCKET`")?,
    )
    .with_token(
        std::env::var("INFLUX_TOKEN")
            .context("Cannot read metrics without environment variable `INFLUX_TOKEN`")?,
    );
    report_countersigning_two_party(client, latest_summaries[0].clone()).await?;

    Ok(())
}

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

async fn report_countersigning_two_party(client: influxdb::Client, summary: RunSummary) -> anyhow::Result<()> {
    assert_eq!(summary.scenario_name, "two_party_countersigning");

    let mean = query_mean(client.clone(), &summary, "wt.custom.countersigning_session_initiated_duration").await?;
    let spread = query_spread(client.clone(), &summary, "wt.custom.countersigning_session_initiated_duration").await?;
    let std_dev = query_std_dev(client.clone(), &summary, "wt.custom.countersigning_session_initiated_duration").await?;

    println!("Mean: {}, Spread: {}, Std Dev: {}", mean, spread, std_dev);

    Ok(())
}

async fn query_mean(client: influxdb::Client, summary: &RunSummary, metric: &str) -> anyhow::Result<f64> {
    let q = ReadQuery::new(format!(r#"SELECT MEAN(value) FROM "windtunnel"."autogen"."{}" WHERE run_id = '{}'"#, metric, summary.run_id));
    let res = client.json_query(q).await?;
    read_float_result(res)
}

async fn query_spread(client: influxdb::Client, summary: &RunSummary, metric: &str) -> anyhow::Result<f64> {
    let q = ReadQuery::new(format!(r#"SELECT SPREAD(value) FROM "windtunnel"."autogen"."{}" WHERE run_id = '{}'"#, metric, summary.run_id));
    let res = client.json_query(q).await?;
    read_float_result(res)
}

async fn query_std_dev(client: influxdb::Client, summary: &RunSummary, metric: &str) -> anyhow::Result<f64> {
    let q = ReadQuery::new(format!(r#"SELECT STDDEV(value) FROM "windtunnel"."autogen"."{}" WHERE run_id = '{}'"#, metric, summary.run_id));
    let res = client.json_query(q).await?;
    read_float_result(res)
}

fn read_float_result(result: DatabaseQueryResult) -> anyhow::Result<f64> {
    let value = result.results[0].clone();
    let x = serde_json::from_value::<QueryResult>(value)?;
    Ok(x.series[0].values[0][1].as_number().unwrap().as_f64().unwrap())
}

fn read_int_result(result: DatabaseQueryResult) -> anyhow::Result<i64> {
    let value = result.results[0].clone();
    let x = serde_json::from_value::<QueryResult>(value)?;
    Ok(x.series[0].values[0][1].as_number().unwrap().as_i64().unwrap())
}
