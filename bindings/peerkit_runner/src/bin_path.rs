use anyhow::Context;
use std::path::PathBuf;

/// Environment variable that overrides the path to the `peerkit` executable.
pub const WT_PEERKIT_PATH_ENV: &str = "WT_PEERKIT_PATH";

/// Resolve the `peerkit` executable: `$WT_PEERKIT_PATH` if set, otherwise
/// `peerkit` found on the `PATH`.
pub fn peerkit_bin_path() -> anyhow::Result<PathBuf> {
    if let Ok(path) = std::env::var(WT_PEERKIT_PATH_ENV) {
        return Ok(PathBuf::from(path));
    }
    which::which("peerkit").context("`peerkit` not found: set WT_PEERKIT_PATH or add it to PATH")
}
