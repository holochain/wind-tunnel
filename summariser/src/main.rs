use anyhow::Context;
use chrono::Utc;
use std::fs::File;
use std::path::PathBuf;
use wind_tunnel_summary_model::load_summary_runs;

pub(crate) mod filter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    #[cfg(feature = "test_data")]
    log::info!("Test data generation enabled");

    let summary_runs = load_summary_runs(PathBuf::from("run_summary.jsonl"))
        .expect("Failed to load run summaries");

    let latest_by_config_summaries = filter::latest_run_summaries_by_name_and_config(summary_runs);

    for (name, fingerprint, summary) in &latest_by_config_summaries {
        log::debug!(
            "Selected summary for {} ({}): {:?}",
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

    let mut summary_outputs =
        futures::future::join_all(latest_by_config_summaries.into_iter().filter_map(
            |(_, _, summary)| {
                // When the test data feature is enabled, dump the run summary to a file
                #[cfg(feature = "test_data")]
                match holochain_summariser::test_data::insert_run_summary(&summary) {
                    Ok(()) => log::info!("Inserted test data for {}", summary.scenario_name),
                    Err(e) => {
                        use futures::FutureExt;

                        log::error!("Failed to insert test data for {:?}: {:?}", summary, e);
                        return Some(async move { Err(e) }.boxed());
                    }
                }

                holochain_summariser::execute_report_for_run_summary(client.clone(), summary)
            },
        ))
        .await
        .into_iter()
        .collect::<anyhow::Result<Vec<_>>>()?;

    summary_outputs.sort_by_key(|r| r.run_summary.scenario_name.clone());

    #[cfg(feature = "test_data")]
    for output in &summary_outputs {
        holochain_summariser::test_data::insert_summary_output(output, false)?;
    }

    if summary_outputs.is_empty() {
        log::warn!("No reports were generated");
    } else {
        let report = File::create_new(format!("summariser-report-{:?}.json", Utc::now()))?;
        serde_json::to_writer_pretty(report, &summary_outputs)?;
    }

    Ok(())
}
