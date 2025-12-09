use anyhow::bail;
use anyhow::Context;
use std::env;
use std::path::PathBuf;
use wind_tunnel_runner::prelude::WindTunnelResult;

/// Environment variable to override the path to the Holochain binary used to run conductors.
pub const WT_HOLOCHAIN_PATH_ENV: &str = "WT_HOLOCHAIN_PATH";

/// Get the path to the Holochain binary.
///
/// If the [`WT_HOLOCHAIN_PATH_ENV`] environment variable is set, its value is used as the path to
/// the Holochain binary. If it is not set, the default value "holochain" is used, which assumes that
/// the binary is available in the system's PATH.
pub(crate) fn holochain_path() -> WindTunnelResult<PathBuf> {
    match env::var(WT_HOLOCHAIN_PATH_ENV).ok().as_deref() {
        Some("") => {
            bail!("'{WT_HOLOCHAIN_PATH_ENV}' set to empty string");
        }
        Some("holochain") | None => {
            log::warn!("'{WT_HOLOCHAIN_PATH_ENV}' is not a path so looking in user's 'PATH'");
            // check whether holochain exist in path
            which::which("holochain").with_context(|| {
                format!(
                    "Holochain binary not found in PATH. Please install Holochain or set '{WT_HOLOCHAIN_PATH_ENV}' to the correct path."
                )
            })
        }
        Some(path) => {
            let holochain_path = PathBuf::from(path);
            if !holochain_path.exists() {
                bail!(
                "Path to Holochain binary overwritten with '{WT_HOLOCHAIN_PATH_ENV}={path}' but that path doesn't exist",
                path = holochain_path.display()
            );
            }
            Ok(holochain_path)
        }
    }
}
