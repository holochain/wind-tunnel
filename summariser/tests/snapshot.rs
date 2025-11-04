use anyhow::Context;
use holochain_summariser::{execute_report_for_run_summary, model::SummaryOutput};
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};
use wind_tunnel_summary_model::load_run_summary;

macro_rules! run_snapshot_test {
    ($summary_fingerprint:literal) => {
        env_logger::try_init().ok();

        let run_summary = find_test_data_file($summary_fingerprint, "1_run_summaries")
            .with_context(|| format!("Run summary not found: {}", $summary_fingerprint))?;
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

#[test]
fn ensure_run_summary_fingerprints_accurate() {
    for entry in WalkDir::new(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("test_data")
            .join("1_run_summaries"),
    )
    .into_iter()
    {
        let entry = entry.unwrap();
        if entry.file_type().is_file() {
            let summary = load_run_summary(
                std::fs::File::open(entry.path()).expect("Failed to load run summary"),
            )
            .unwrap();

            summary.fingerprint();
            std::fs::rename(
                entry.path(),
                entry.path().with_file_name(format!(
                    "{}-{}.json",
                    summary.scenario_name,
                    summary.fingerprint()
                )),
            )
            .expect("Failed to rename file");
        }
    }
}

#[test]
fn ensure_summary_output_fingerprints_accurate() {
    for entry in WalkDir::new(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("test_data")
            .join("3_summary_outputs"),
    )
    .into_iter()
    {
        let entry = entry.unwrap();
        if entry.file_type().is_file() {
            let summary_output = load_summary_output(entry.path().into()).unwrap();

            std::fs::rename(
                entry.path(),
                entry.path().with_file_name(format!(
                    "{}-{}.json",
                    summary_output.run_summary.scenario_name,
                    summary_output.run_summary.fingerprint()
                )),
            )
            .expect("Failed to rename file");
        }
    }
}

#[tokio::test]
async fn app_install_minimal() -> anyhow::Result<()> {
    run_snapshot_test!("8468d686b75c702a7c523c4da615b817c09d20d5ead54f2806513b756652055a");
    Ok(())
}

#[tokio::test]
async fn app_install_large() -> anyhow::Result<()> {
    run_snapshot_test!("52bd6609472c523e0561f09d7daecaac880d348f71241f6fabf71d28795e79ec");
    Ok(())
}

#[tokio::test]
async fn dht_sync_lag() -> anyhow::Result<()> {
    run_snapshot_test!("3a1e33ccf661bd873966c539d4d227e703e1496fb54bb999f7be30a3dd493e51");
    Ok(())
}

#[tokio::test]
async fn first_call() -> anyhow::Result<()> {
    run_snapshot_test!("608026f5fcbb4e5e87cba8d616ae379c20a7c8930138d36f11ddde5134e4e730");
    Ok(())
}

#[tokio::test]
async fn local_signals() -> anyhow::Result<()> {
    run_snapshot_test!("1fd6a6042bf4d93742d9fe912adac0c896d133f8cccb90758db60fefd09a7060");
    Ok(())
}

#[tokio::test]
async fn remote_call_rate() -> anyhow::Result<()> {
    run_snapshot_test!("f92e98962b23bfe104373a735dd9af8eb363e347a0c528902d4a2aaa8351cd74");
    Ok(())
}

#[tokio::test]
async fn single_write_many_read() -> anyhow::Result<()> {
    run_snapshot_test!("53072be05686ee83ae234f248d2791e0576cdc05065954975b89549571613a97");
    Ok(())
}

#[tokio::test]
async fn two_party_countersigning() -> anyhow::Result<()> {
    run_snapshot_test!("3cdc5a29d42fbe93e971508e7cd8856367465e047c870b60283ed86a5bf5687c");
    Ok(())
}

#[tokio::test]
async fn validation_receipts() -> anyhow::Result<()> {
    run_snapshot_test!("7ec48e9c40fde50ceb5fccc5cfdcaabd69eebd148eb9c3590fda2e2419152f70");
    Ok(())
}

#[tokio::test]
async fn write_query() -> anyhow::Result<()> {
    run_snapshot_test!("d785bb3d4bdb5e2ae9405c5e1899cb49c9a6fbe34e8baa8e6df8819dee5c7233");
    Ok(())
}

#[tokio::test]
async fn write_read() -> anyhow::Result<()> {
    run_snapshot_test!("2beac84ddff6d46234ece3c1227d9c19ef1d1bc0e78e4c0aac2e0e4eacf6ef1c");
    Ok(())
}

#[tokio::test]
async fn write_validated() -> anyhow::Result<()> {
    run_snapshot_test!("5d3ab06e123d1245b15c348d1512b7dd0f6feff44f0b0a3b79b1b72ca3e4b2b7");
    Ok(())
}

#[tokio::test]
async fn zome_call_single_value() -> anyhow::Result<()> {
    run_snapshot_test!("4e74c47aa5158bbdc0c010fdc44de565c8b6592e472026f0bca9464d69e1e99b");
    Ok(())
}

#[tokio::test]
async fn write_get_agent_activity() -> anyhow::Result<()> {
    run_snapshot_test!("80fc1af2a7172fe34aee5ff222cfd2e8a09b7190f05722fd508c3d24199ddee7");
    Ok(())
}

#[tokio::test]
async fn write_validated_must_get_agent_activity() -> anyhow::Result<()> {
    run_snapshot_test!("f11ec2e0490ac1fba9137f7a3c7ec261262d8305478d22844f3a2df60573f14b");
    Ok(())
}

#[tokio::test]
async fn zero_arc_create_data() -> anyhow::Result<()> {
    run_snapshot_test!("c90e09f59fccb19cacaff231094f9fa49d3da999d6fa7c169328c1030ed1537d");
    Ok(())
}

fn find_test_data_file(summary_fingerprint: &str, stage: &str) -> Option<DirEntry> {
    let all_matches = WalkDir::new(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("test_data")
            .join(stage),
    )
    .into_iter()
    .filter_map(|entry| entry.ok())
    .filter(|entry| {
        entry
            .file_name()
            .to_str()
            .map(|name| name.contains(summary_fingerprint))
            .unwrap_or(false)
    })
    .collect::<Vec<_>>();

    if all_matches.len() == 1 {
        Some(all_matches[0].clone())
    } else {
        panic!(
            "Expected exactly one match, this indicates a fingerprint collision: {all_matches:?}"
        );
    }
}

/// Load summary output from a file
pub fn load_summary_output(path: PathBuf) -> anyhow::Result<SummaryOutput> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    serde_json::from_reader(reader).context("Failed to load summary output")
}
