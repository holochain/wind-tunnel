use anyhow::{Context, anyhow};
use chrono::Utc;
use log::debug;
use std::fs::File;
use std::path::PathBuf;
use wind_tunnel_summary_model::load_summary_runs;

pub(crate) mod filter;

/// Environment variable name to set a custom run summary file path
const RUN_SUMMARY_PATH_ENV: &str = "RUN_SUMMARY_PATH";
/// Default path for the run summary file
const DEFAULT_RUN_SUMMARY_PATH: &str = "run_summary.jsonl";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let ignore_errors = std::env::var("IGNORE_SUMMARY_ERRORS").is_ok();

    #[cfg(feature = "test_data")]
    log::info!("Test data generation enabled");

    let summary_path = std::env::var(RUN_SUMMARY_PATH_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_RUN_SUMMARY_PATH));
    debug!("Loading summary from {}", summary_path.display());
    let summary_runs = load_summary_runs(summary_path).expect("Failed to load run summaries");

    let latest_by_config_summaries = filter::latest_run_summaries_by_name_and_config(summary_runs);

    for (name, fingerprint, summary) in &latest_by_config_summaries {
        log::debug!("Selected summary for {name} ({fingerprint}): {summary:?}");
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

    let summary_results =
        futures::future::join_all(latest_by_config_summaries.into_iter().filter_map(
            |(_, _, summary)| {
                // When the test data feature is enabled, dump the run summary to a file
                #[cfg(feature = "test_data")]
                match holochain_summariser::test_data::insert_run_summary(&summary) {
                    Ok(()) => log::info!("Inserted test data for {}", summary.scenario_name),
                    Err(e) => {
                        use futures::{FutureExt, future::ready};

                        log::error!("Failed to insert test data for {summary:?}: {e:?}",);
                        return Some(ready(Err(e)).boxed());
                    }
                }

                holochain_summariser::execute_report_for_run_summary(client.clone(), summary)
            },
        ))
        .await
        .into_iter()
        .collect::<Vec<_>>();

    let total_summaries = summary_results.len();
    let mut errors = vec![];
    let mut summary_outputs = vec![];

    for result in summary_results {
        match result {
            Ok(output) => {
                summary_outputs.push(output);
            }
            Err(e) => {
                errors.push(e);
            }
        }
    }
    summary_outputs.sort_by_key(|r| r.run_summary.scenario_name.clone());

    #[cfg(feature = "test_data")]
    for output in &summary_outputs {
        holochain_summariser::test_data::insert_summary_output(output, false)?;
    }

    let report = File::create_new(format!(
        "summariser-report-{}.json",
        Utc::now().format("%Y-%m-%dT%H.%M.%S%.fZ")
    ))?;

    serde_json::to_writer_pretty(report, &summary_outputs)?;

    // If any of the summaries failed and errors should not explicitly be ignored, return an error
    if !errors.is_empty() {
        let error_message = format!(
            "{} out of {} summaries failed:\n{:#?}",
            errors.len(),
            total_summaries,
            errors
        );

        if ignore_errors {
            log::warn!("{}", error_message);
        } else {
            return Err(anyhow!(error_message));
        }
    }

    Ok(())
}
