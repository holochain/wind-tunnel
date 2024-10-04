use crate::scenario::summarize_countersigning_two_party;
use anyhow::Context;
use futures::FutureExt;
use scenario::summarize_app_install;
use std::path::PathBuf;
use wind_tunnel_summary_model::load_summary_runs;

mod filter;
mod frame;
mod model;
mod query;
mod scenario;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let summary_runs = load_summary_runs(PathBuf::from("run_summary.jsonl"))
        .expect("Failed to load run summaries");

    // Note that this is just a simple selection strategy. If we have run scenarios with more than
    // one configuration, we might want to select multiple summaries per scenario name.
    let latest_summaries = filter::latest_run_summaries_by_name(&summary_runs);

    let latest_by_config_summaries = filter::latest_run_summaries_by_name_and_config(summary_runs);

    for summary in &latest_summaries {
        println!("{:?}", summary);
    }

    for (name, fingerprint, summary) in &latest_by_config_summaries {
        println!("{:?} {:?} {:?}", name, fingerprint, summary);
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

    let outcome = futures::future::join_all(latest_by_config_summaries.into_iter().filter_map(
        |(name, _, summary)| {
            let client = client.clone();
            match name.as_str() {
                "app_install" => Some(
                    async move {
                        summarize_app_install(client.clone(), summary.clone())
                            .await
                            .context("App install report")
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
                    println!("No report for scenario: {}", name);
                    None
                }
            }
        },
    ))
    .await
    .into_iter()
    .collect::<anyhow::Result<Vec<_>>>()?;
    // .collect::<Vec<_>>()
    // .await

    println!("Outcome: {:?}", outcome);

    Ok(())
}
