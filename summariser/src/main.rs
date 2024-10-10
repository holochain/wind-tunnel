use crate::scenario::{
    summarize_countersigning_two_party, summarize_first_call, summarize_local_signals,
    summarize_remote_call_rate, summarize_single_write_many_read,
};
use anyhow::Context;
use chrono::Utc;
use futures::FutureExt;
use scenario::summarize_app_install;
use std::fs::File;
use std::path::PathBuf;
use wind_tunnel_summary_model::load_summary_runs;

mod analyze;
mod filter;
mod frame;
mod model;
mod query;
mod scenario;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let summary_runs = load_summary_runs(PathBuf::from("run_summary.jsonl"))
        .expect("Failed to load run summaries");

    let latest_by_config_summaries = filter::latest_run_summaries_by_name_and_config(summary_runs);

    for (name, fingerprint, summary) in &latest_by_config_summaries {
        log::debug!(
            "Selected summary for {:?}({:?}): {:?}",
            name,
            fingerprint,
            summary
        );
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

    let mut outcome = futures::future::join_all(latest_by_config_summaries.into_iter().filter_map(
        |(name, _, summary)| {
            let client = client.clone();
            match name.as_str() {
                "app_install" => Some(
                    async move {
                        summarize_app_install(client.clone(), summary.clone())
                            .await
                            .context("App install summary")
                    }
                    .boxed(),
                ),
                "first_call" => Some(
                    async move {
                        summarize_first_call(client.clone(), summary.clone())
                            .await
                            .context("First call summary")
                    }
                    .boxed(),
                ),
                "local_signals" => Some(
                    async move {
                        summarize_local_signals(client.clone(), summary.clone())
                            .await
                            .context("Local signals summary")
                    }
                    .boxed(),
                ),
                "remote_call_rate" => Some(
                    async move {
                        summarize_remote_call_rate(client.clone(), summary.clone())
                            .await
                            .context("Remote call rate summary")
                    }
                    .boxed(),
                ),
                "single_write_many_read" => Some(
                    async move {
                        summarize_single_write_many_read(client.clone(), summary.clone())
                            .await
                            .context("Single write many read summary")
                    }
                    .boxed(),
                ),
                "two_party_countersigning" => Some(
                    async move {
                        summarize_countersigning_two_party(client.clone(), summary.clone())
                            .await
                            .context("Countersigning, two party, report")
                    }
                    .boxed(),
                ),
                _ => {
                    log::warn!("No report for scenario: {}", name);
                    None
                }
            }
        },
    ))
    .await
    .into_iter()
    .collect::<anyhow::Result<Vec<_>>>()?;

    outcome.sort_by_key(|r| r.run_summary.scenario_name.clone());

    let report = File::create_new(format!("summariser-report-{:?}.json", Utc::now()))?;
    serde_json::to_writer_pretty(report, &outcome)?;

    Ok(())
}
