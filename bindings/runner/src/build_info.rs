use anyhow::{bail, Context};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Information about the [`HolochainBuildInfo`] git info
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HolochainBuildGitInfo {
    pub rev: String,
    pub dirty: bool,
}

/// Information about the Holochain build used in the run
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HolochainBuildInfo {
    pub cargo_pkg_version: String,
    pub hdk_version_req: String,
    pub hdi_version_req: String,
    pub lair_keystore_version_req: String,
    pub timestamp: DateTime<Utc>,
    pub hostname: String,
    pub host: String,
    pub target: String,
    pub rustc_version: String,
    pub rustflags: String,
    pub profile: String,
    pub git_info: Option<HolochainBuildGitInfo>,
}

/// Get the build info of the Holochain binary by running `holochain --build-info`.
pub(crate) fn holochain_build_info(bin_path: PathBuf) -> anyhow::Result<HolochainBuildInfo> {
    let output = std::process::Command::new(bin_path)
        .arg("--build-info")
        .output()
        .context("Failed to execute 'holochain --build-info' command")?;
    if !output.status.success() {
        bail!(
            "'holochain --build-info' command failed with exit code: {status}",
            status = output.status
        );
    }

    let output = String::from_utf8(output.stdout)
        .context("Failed to parse output of 'holochain --build-info' command as UTF-8")?
        .trim()
        .to_string();

    serde_json::from_str(&output)
        .context("Failed to parse JSON output of 'holochain --build-info' command")
}
