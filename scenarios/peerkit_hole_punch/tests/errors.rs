#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::process::Command;

const FAKE_PEERKIT: &str = r#"#!/usr/bin/env bash
identity="${PEERKIT_IDENTITY##*/}"
printf '\nNode session started with agent ID %s\n\npeerkit> ' "${identity%.key}"
printf '\033[1G2026-08-12T10:00:00.000Z [Connected to relay with ID]: relay\npeerkit> '
polls=0
while IFS= read -r command; do
    case "$command" in
        peers)
            polls=$((polls + 1))
            if [ "$FAKE_PEERKIT_FAILURE" = initial ]; then exit 1; fi
            if [ "$polls" -gt 1 ]; then
                if [ "$FAKE_PEERKIT_FAILURE" = upgrade ]; then exit 1; fi
                continue
            fi
            printf '1   [not connected] 0 blob(s)  bbbbbbbb…bbbb\npeerkit> '
            ;;
        "conn 1") printf 'Connected to 1\npeerkit> ' ;;
        exit) exit 0 ;;
    esac
done
"#;

fn error_metrics(failure: &str) -> Vec<String> {
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("peerkit.sh");
    std::fs::write(&script, FAKE_PEERKIT).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let metrics_dir = dir.path().join("metrics");
    let output = Command::new(env!("CARGO_BIN_EXE_peerkit_hole_punch"))
        .args([
            "--relay-dial-addr",
            "unused",
            "--agents",
            "1",
            "--behaviour",
            "node:1",
            "--duration",
            "1",
            "--no-progress",
            "--reporter",
            "influx-file",
        ])
        .env("WT_PEERKIT_PATH", script)
        .env("WT_METRICS_DIR", &metrics_dir)
        .env("RUN_SUMMARY_PATH", dir.path().join("summary.jsonl"))
        .env("TMPDIR", dir.path())
        .env("FAKE_PEERKIT_FAILURE", failure)
        .env("PEERKIT_DIRECT_UPGRADE_TIMEOUT_MS", "10000")
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(output.status.success(), "scenario failed: {output:?}");

    std::fs::read_dir(metrics_dir)
        .unwrap()
        .flat_map(|entry| {
            std::fs::read_to_string(entry.unwrap().path())
                .unwrap()
                .lines()
                .filter(|line| line.starts_with("wt.custom.peerkit_error_count,"))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn initial_peer_poll_failure_is_counted_once() {
    let errors = error_metrics("initial");
    assert_eq!(errors.len(), 1, "metrics: {errors:?}");
    assert!(errors[0].contains("kind=behaviour"));
}

#[test]
fn upgrade_peer_poll_failure_is_counted_once() {
    let errors = error_metrics("upgrade");
    assert_eq!(errors.len(), 1, "metrics: {errors:?}");
    assert!(errors[0].contains("kind=behaviour"));
}

#[test]
fn shutdown_during_peer_poll_is_not_counted_as_an_error() {
    assert!(error_metrics("shutdown").is_empty());
}
