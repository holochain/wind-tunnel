use happ_builder::{HappManager, HappManagerOptions};

const SAMPLE_URL: &str = "https://github.com/holochain/dino-adventure/releases/download/v0.3.0-dev.0/dino-adventure-v0.3.0-dev.0.happ";
const EXPECTED_SHA256: &str = "1eafa0d852d9e96e54f0b6969fb06de83989ece0059bc2b376884ac52fb6a63a";

/// Helper function to create a test Cargo.toml with fetch-required-happ metadata
fn create_test_manifest(dir: &std::path::Path, happ_name: &str, url: &str, sha256: &str) {
    let manifest_content = format!(
        r#"[package]
name = "test-package"
version = "0.1.0"

[[package.metadata.fetch-required-happ]]
name = "{}"
url = "{}"
sha256 = "{}"
"#,
        happ_name, url, sha256
    );
    std::fs::write(dir.join("Cargo.toml"), manifest_content).expect("failed to write test manifest");
}

#[test]
fn should_fetch_happ() {
    let tempdir = tempfile::tempdir().expect("failed to create temp dir");
    let manifest_dir = tempdir.path().to_path_buf();
    let happ_target_dir = tempdir.path().join("happs");

    // Create test manifest
    create_test_manifest(&manifest_dir, "dino-adventure", SAMPLE_URL, EXPECTED_SHA256);

    // Create HappManager with test options
    let options = HappManagerOptions {
        package_name: "test-package".to_string(),
        manifest_dir: manifest_dir.clone(),
        out_dir: tempdir.path().join("out"),
        target_dir: tempdir.path().join("target"),
        zomes_dir: tempdir.path().join("zomes"),
        dna_target_dir: tempdir.path().join("dnas"),
        happ_target_dir: happ_target_dir.clone(),
    };

    let manager = HappManager::from(options);
    manager.ensure_happs_available().expect("failed to fetch happ");

    let fetched_happ_path = happ_target_dir.join("dino-adventure.happ");
    assert!(
        fetched_happ_path.exists(),
        "fetched happ file does not exist"
    );
}

#[test]
fn should_refetch_if_hash_mismatch() {
    let tempdir = tempfile::tempdir().expect("failed to create temp dir");
    let manifest_dir = tempdir.path().to_path_buf();
    let happ_target_dir = tempdir.path().join("happs");

    // Create happs directory and a corrupted file
    std::fs::create_dir_all(&happ_target_dir).expect("failed to create happs dir");
    let file_path = happ_target_dir.join("dino-adventure.happ");
    std::fs::write(&file_path, b"corrupted data").expect("failed to write test file");

    // Create test manifest
    create_test_manifest(&manifest_dir, "dino-adventure", SAMPLE_URL, EXPECTED_SHA256);

    // Create HappManager with test options
    let options = HappManagerOptions {
        package_name: "test-package".to_string(),
        manifest_dir: manifest_dir.clone(),
        out_dir: tempdir.path().join("out"),
        target_dir: tempdir.path().join("target"),
        zomes_dir: tempdir.path().join("zomes"),
        dna_target_dir: tempdir.path().join("dnas"),
        happ_target_dir: happ_target_dir.clone(),
    };

    let manager = HappManager::from(options);
    manager.ensure_happs_available().expect("failed to fetch happ");

    assert!(
        file_path.exists(),
        "fetched file does not exist after hash mismatch"
    );

    // Verify the file was actually refetched by checking its size
    // (corrupted data is much smaller than the actual happ file)
    let metadata = std::fs::metadata(&file_path).expect("failed to get file metadata");
    assert!(
        metadata.len() > 100,
        "file was not refetched, still contains corrupted data"
    );
}

#[test]
fn should_fail_if_hash_mismatch_manifest() {
    let tempdir = tempfile::tempdir().expect("failed to create temp dir");
    let manifest_dir = tempdir.path().to_path_buf();
    let happ_target_dir = tempdir.path().join("happs");

    // Use a wrong SHA256 in the manifest
    let wrong_sha256 = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
    create_test_manifest(&manifest_dir, "dino-adventure", SAMPLE_URL, wrong_sha256);

    // Create HappManager with test options
    let options = HappManagerOptions {
        package_name: "test-package".to_string(),
        manifest_dir: manifest_dir.clone(),
        out_dir: tempdir.path().join("out"),
        target_dir: tempdir.path().join("target"),
        zomes_dir: tempdir.path().join("zomes"),
        dna_target_dir: tempdir.path().join("dnas"),
        happ_target_dir: happ_target_dir.clone(),
    };

    let manager = HappManager::from(options);
    let result = manager.ensure_happs_available();
    assert!(result.is_err(), "expected error due to sha256 mismatch");
}
