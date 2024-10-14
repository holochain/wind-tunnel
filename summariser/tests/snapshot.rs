use anyhow::Context;
use holochain_summariser::{execute_report_for_run_summary, model::SummaryOutput};
use std::path::Path;
use walkdir::{DirEntry, WalkDir};
use wind_tunnel_summary_model::load_run_summary;

macro_rules! run_snapshot_test {
    ($summary_fingerprint:literal) => {
        let run_summary = find_test_data_file($summary_fingerprint, "1_run_summaries")
            .context("Run summary not found")?;
        let run_summary = load_run_summary(
            std::fs::File::open(run_summary.path()).context("Failed to load run summary")?,
        )?;

        let expected = find_test_data_file($summary_fingerprint, "3_summary_outputs")
            .context("Summary output not found")?;
        let expected = serde_json::from_reader::<_, SummaryOutput>(
            std::fs::File::open(expected.path())
                .context("Failed to load expected summary output")?,
        )?;

        let output = execute_report_for_run_summary(
            influxdb::Client::new("http://never-connect", "test"),
            run_summary,
        )
        .expect("No reporter configured for this scenario")
        .await
        .expect("Failed to execute report");

        pretty_assertions::assert_eq!(expected, output);
    };
}

#[tokio::test]
async fn app_install_minimal() -> anyhow::Result<()> {
    run_snapshot_test!("f8a3fe811d284d42c923571701e31ffdf01cfeaaa11561e34d3712aedb2a95ae");
    Ok(())
}

fn find_test_data_file(summary_fingerprint: &str, stage: &str) -> Option<DirEntry> {
    WalkDir::new(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("test_data")
            .join(stage),
    )
    .into_iter()
    .filter_map(|entry| entry.ok())
    .find(|entry| {
        entry
            .file_name()
            .to_str()
            .map(|name| name.contains(summary_fingerprint))
            .unwrap_or(false)
    })
}
