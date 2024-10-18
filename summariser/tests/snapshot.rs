use anyhow::Context;
use holochain_summariser::{execute_report_for_run_summary, model::SummaryOutput};
use std::path::Path;
use walkdir::{DirEntry, WalkDir};
use wind_tunnel_summary_model::load_run_summary;

macro_rules! run_snapshot_test {
    ($summary_fingerprint:literal) => {
        env_logger::try_init().ok();

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

        if option_env!("UPDATE_SNAPSHOTS") == Some("1") {
            holochain_summariser::test_data::insert_summary_output(&output, true)?;
        } else {
            pretty_assertions::assert_eq!(expected, output, "Snapshot mismatch, run with `UPDATE_SNAPSHOTS=1 cargo test --test snapshot` to update");
        }
    };
}

#[tokio::test]
async fn app_install_minimal() -> anyhow::Result<()> {
    run_snapshot_test!("f8a3fe811d284d42c923571701e31ffdf01cfeaaa11561e34d3712aedb2a95ae");
    Ok(())
}

#[tokio::test]
async fn app_install_large() -> anyhow::Result<()> {
    run_snapshot_test!("9b7bade717a222157887e8b2973cee491ffcc98c7417d37dec8fc27ce2dfe305");
    Ok(())
}

#[tokio::test]
async fn first_call() -> anyhow::Result<()> {
    run_snapshot_test!("1c1782ff0b6a1fce9640342be79bc23970833bc535cba493f9e494e65919d436");
    Ok(())
}

#[tokio::test]
async fn local_signals() -> anyhow::Result<()> {
    run_snapshot_test!("1c1782ff0b6a1fce9640342be79bc23970833bc535cba493f9e494e65919d436");
    Ok(())
}

#[tokio::test]
async fn remote_call_rate() -> anyhow::Result<()> {
    run_snapshot_test!("3ca57c491607c8639ba04caef533f49833549bf7a2fab9851ba2cb9494d16fe2");
    Ok(())
}

#[tokio::test]
async fn trycp_write_validated() -> anyhow::Result<()> {
    run_snapshot_test!("3ca57c491607c8639ba04caef533f49833549bf7a2fab9851ba2cb9494d16fe2");
    Ok(())
}

#[tokio::test]
async fn two_party_countersigning() -> anyhow::Result<()> {
    run_snapshot_test!("8f5f4e70e7852399484a024dcfec72909b3778c7edb642e6750aec5772bc2fc0");
    Ok(())
}

#[tokio::test]
async fn validation_receipts() -> anyhow::Result<()> {
    run_snapshot_test!("3265009665eab80d8b796d448aa6ae1739a7b416f4f98fda7e37c9fc5d898729");
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
