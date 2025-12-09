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

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt as _;

    use serial_test::serial;
    use tempfile::{NamedTempFile, TempDir};

    use super::*;

    #[test]
    #[serial]
    fn test_should_not_get_holochain_path_if_not_exist() {
        env::set_var(WT_HOLOCHAIN_PATH_ENV, "/non/existent/path/to/holochain");
        let result = holochain_path();
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_should_get_holochain_path_from_env() {
        let temp = NamedTempFile::new().expect("failed to create temp file");
        let test_path = temp.path().to_str().expect("failed to get temp file path");
        env::set_var(WT_HOLOCHAIN_PATH_ENV, test_path);
        let result = holochain_path().expect("failed to get holochain path");
        assert_eq!(result, PathBuf::from(test_path));
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn test_should_get_default_holochain_path() {
        let temp = TempDir::new().expect("failed to create temp file");
        // create holochain file in temp dir
        let holochain_file_path = temp.path().join("holochain");
        std::fs::write(&holochain_file_path, "hello").expect("failed to create holochain file");
        let mut perms = std::fs::metadata(&holochain_file_path)
            .unwrap()
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&holochain_file_path, perms).unwrap();

        // put test_path parent to PATH
        let new_path = format!("{}", temp.path().display());
        env::set_var("PATH", new_path);

        // remove WT_HOLOCHAIN_PATH_ENV to test default behavior
        env::remove_var(WT_HOLOCHAIN_PATH_ENV);

        let result = holochain_path().expect("failed to get holochain path");
        assert_eq!(result, holochain_file_path);
    }

    #[test]
    #[serial]
    fn test_should_not_get_default_holochain_path() {
        // unset PATH
        env::remove_var("PATH");

        // remove WT_HOLOCHAIN_PATH_ENV to test default behavior
        env::remove_var(WT_HOLOCHAIN_PATH_ENV);

        let result = holochain_path();
        println!("{result:?}",);
        assert!(result.is_err());
    }
}
