use anyhow::Context;
use holochain_summariser::{execute_report_for_run_summary, model::SummaryOutput};
use std::collections::BTreeMap;
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
            let expected = find_test_data_file($summary_fingerprint, "3_summary_outputs")
                .context("Summary output not found")?;
            let expected = serde_json::from_reader::<_, SummaryOutput>(
                std::fs::File::open(expected.path())
                    .context("Failed to load expected summary output")?,
            )?;

            // Normalize scenario_metrics JSON key ordering so diffs are readable.
            // serde_json::Value PartialEq is order-independent, but pretty_assertions
            // diffs the Debug output which renders keys in insertion order.
            let mut expected_norm = expected.clone();
            expected_norm.scenario_metrics = normalize_json(&expected_norm.scenario_metrics);
            let mut output_norm = output.clone();
            output_norm.scenario_metrics = normalize_json(&output_norm.scenario_metrics);

            pretty_assertions::assert_eq!(expected_norm, output_norm, "Snapshot mismatch, run with `UPDATE_SNAPSHOTS=1 cargo test --test snapshot` to update");
        }
    };
}

/// Recursively sort JSON object keys so that Debug output is deterministic.
fn normalize_json(v: &serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(map) => {
            let sorted: BTreeMap<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), normalize_json(v)))
                .collect();
            serde_json::to_value(sorted).unwrap()
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(normalize_json).collect())
        }
        other => other.clone(),
    }
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
            let summary_output = load_summary_output(entry.path().into())
                .map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to load summary output from {:?}: {}",
                        entry.path(),
                        e
                    )
                })
                .unwrap();

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
    run_snapshot_test!("8c1297e90c9ec664ec04a0d05d6c19af7481b22af40984591ce0d9469a5953a1");
    Ok(())
}

#[tokio::test]
async fn app_install_large() -> anyhow::Result<()> {
    run_snapshot_test!("534e3e768c95de8a61cde145cc84c32ed7d1f3e9e6f1a72747a7e03b89ed3bae");
    Ok(())
}

#[tokio::test]
async fn dht_sync_lag() -> anyhow::Result<()> {
    run_snapshot_test!("0c97cc426147debdd0ff2c7bfdeec11c73b24c8201fb0f4e1fca2ee146774e3b");
    Ok(())
}

#[tokio::test]
async fn first_call() -> anyhow::Result<()> {
    run_snapshot_test!("bfff96e7068c6a7b58d48724a776768f06e6fca4679d38ea463c574628edc347");
    Ok(())
}

#[tokio::test]
async fn full_arc_create_validated_zero_arc_read() -> anyhow::Result<()> {
    run_snapshot_test!("5a298210eab56da8b40e261ac0f85ec2a8243d3ac026b1b09b3dc6640e728743");
    Ok(())
}

#[tokio::test]
async fn local_signals() -> anyhow::Result<()> {
    run_snapshot_test!("252e222d376aaea0d14f4999a97efa27c8371303e7133c408aa2208c8b76f577");
    Ok(())
}

#[tokio::test]
async fn two_party_countersigning() -> anyhow::Result<()> {
    run_snapshot_test!("4a3fd866e9466e8b6ef5a9f022009d275b559fd360bd4547fbff100941059252");
    Ok(())
}

#[tokio::test]
async fn mixed_arc_get_agent_activity() -> anyhow::Result<()> {
    run_snapshot_test!("65dfd11cf1bdd5554d6db6f231f8e67d601ecceda6644c89172897668630e903");
    Ok(())
}

#[tokio::test]
async fn mixed_arc_must_get_agent_activity() -> anyhow::Result<()> {
    run_snapshot_test!("31e5966739534d11f1de7b43b7bafcd72c519ab4fe7e5d17492c89648131fa5d");
    Ok(())
}

#[tokio::test]
async fn remote_call_rate() -> anyhow::Result<()> {
    run_snapshot_test!("32f40bfc9ea4993f23f6e411ee690d0b5054142787b970eeb5214f3786bba9be");
    Ok(())
}

#[tokio::test]
async fn remote_signals() -> anyhow::Result<()> {
    run_snapshot_test!("12206e3db385a10a86c24324b1b9419b7592c31c22cf12800ca4da459d74990e");
    Ok(())
}

#[tokio::test]
async fn single_write_many_read() -> anyhow::Result<()> {
    run_snapshot_test!("2a1ef5165c7c4a0b58ed7865bb6130ca816ed26eb931edd85a4ca650715a52f9");
    Ok(())
}

#[tokio::test]
async fn validation_receipts() -> anyhow::Result<()> {
    run_snapshot_test!("d8e2bf9989e959ca9116ef9f3d6ec6a6912e0eb042e4a90b2d520c1ed9dd199b");
    Ok(())
}

#[tokio::test]
async fn write_query() -> anyhow::Result<()> {
    run_snapshot_test!("e0cfb2ee2b09c8a9de9a0af951101154372403f6046540062f7ddf73fd6cbc26");
    Ok(())
}

#[tokio::test]
async fn write_read() -> anyhow::Result<()> {
    run_snapshot_test!("283f11e10b610015d93fa8df25dee38b2cfbf69723ca7e45327df7fab0dfffc2");
    Ok(())
}

#[tokio::test]
async fn write_validated() -> anyhow::Result<()> {
    run_snapshot_test!("41da0e6566122eeb699af670ea3505e42d882ef5f960edae52fb8c1d56ca1b03");
    Ok(())
}

#[tokio::test]
async fn zome_call_single_value() -> anyhow::Result<()> {
    run_snapshot_test!("9d35fd5d9ce8ea81dda778a57c119fc43e7ddd801e86ea2647f1bde10093bfa3");
    Ok(())
}

#[tokio::test]
async fn write_get_agent_activity() -> anyhow::Result<()> {
    run_snapshot_test!("a89bb3a78861fbd7f52638571c24c17f9f931107447f0ba71c77b17cbddc9f4c");
    Ok(())
}

#[tokio::test]
async fn write_validated_must_get_agent_activity() -> anyhow::Result<()> {
    run_snapshot_test!("1e10b15f872e7ddb4f1d42d64f0a0afd2aebb34f350636f54fbd53e38f81f332");
    Ok(())
}

#[tokio::test]
async fn zero_arc_create_data() -> anyhow::Result<()> {
    run_snapshot_test!("df98bec92a3f70fe30e9fb23871784f13a6f2979c3e19885227ca971e8b827cd");
    Ok(())
}

#[tokio::test]
async fn zero_arc_create_data_validated() -> anyhow::Result<()> {
    run_snapshot_test!("fe64cf7275e429e3c0af337bd5e3ef7ae84d1e3c5feea9569757dfe30e729155");
    Ok(())
}

#[tokio::test]
async fn zero_arc_create_and_read() -> anyhow::Result<()> {
    run_snapshot_test!("52a7fd7068b14a8b64ed436515c89f41cdb0fef4885f88143bfc00c4df51b8a3");
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
