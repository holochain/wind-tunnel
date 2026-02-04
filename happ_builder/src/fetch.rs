use crate::manifest::{FetchRequiredHapp, Sha256Hash};
use crate::{HappBuilderResult, HappManagerOptions};
use anyhow::Context;
use sha2::Sha256;
use std::path::Path;
use std::time::Duration;

pub struct HappFetcher<'a> {
    /// The options for configuring the hApp manager and details from the manifest
    options: &'a HappManagerOptions,
    /// Happs to fetch
    pub happs: &'a [FetchRequiredHapp],
}

impl<'a> HappFetcher<'a> {
    /// Create a new [`HappFetcher`]
    pub fn new(options: &'a HappManagerOptions, happs: &'a [FetchRequiredHapp]) -> Self {
        Self { options, happs }
    }
}

impl HappFetcher<'_> {
    /// Fetch all happs
    pub fn fetch_all(&self) -> HappBuilderResult {
        for happ in self.happs {
            self.fetch_happ(happ)?;
        }

        Ok(())
    }

    /// Fetch a single happ and save it to the target directory
    fn fetch_happ(&self, happ: &FetchRequiredHapp) -> HappBuilderResult {
        let out_path = self
            .options
            .happ_target_dir
            .join(&self.options.package_name)
            .join(format!("{name}.happ", name = &happ.name));

        if out_path.exists()
            && let Ok(existing_sha256) = Self::sha256_file(&out_path)
            && existing_sha256 == happ.sha256
        {
            return Ok(());
        }

        let agent = ureq::config::Config::builder()
            .timeout_global(Some(Duration::from_secs(60)))
            .build()
            .new_agent();
        let response = agent.get(&happ.url).call()?;

        // get the response body as bytes
        let mut body = response.into_body();

        let cleanup_happ_and_dir = || {
            // Try to cleanup the hApp file but ignore errors
            std::fs::remove_file(&out_path).ok();
            // Try to remove the output directory but ignore errors
            std::fs::remove_dir(out_path.parent().unwrap()).ok();
        };

        std::fs::create_dir_all(out_path.parent().expect("no hApp download directory"))?;
        let mut writer = std::fs::File::create(&out_path).context("failed to create happ file")?;
        if let Err(err) =
            std::io::copy(&mut body.as_reader(), &mut writer).context("failed to write happ file")
        {
            cleanup_happ_and_dir();
            return Err(err);
        }

        // verify downloaded file sha256
        let downloaded_sha256 =
            Self::sha256_file(&out_path).context("failed to compute sha256 of downloaded happ")?;
        if downloaded_sha256 != happ.sha256 {
            cleanup_happ_and_dir();
            anyhow::bail!(
                "sha256 mismatch for downloaded happ: expected {}, got {}",
                happ.sha256,
                downloaded_sha256,
            );
        }

        Ok(())
    }

    /// Compute the sha256 of a file
    fn sha256_file(p: &Path) -> anyhow::Result<Sha256Hash> {
        use sha2::Digest;
        use std::io::Read;

        let mut file = std::fs::File::open(p).context("failed to open file for sha256")?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let n = file
                .read(&mut buffer)
                .context("failed to read file for sha256")?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        Ok(hasher.finalize().as_slice().try_into()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Sha256Hash;
    use std::str::FromStr as _;

    #[test]
    fn should_compute_sha256() {
        let tempdir = tempfile::tempdir().expect("failed to create temp dir");
        let file_path = tempdir.path().join("test_file.txt");
        std::fs::write(&file_path, b"hello world").expect("failed to write test file");

        let actual_sha256 = HappFetcher::sha256_file(&file_path).expect("failed to compute sha256");
        let expected_sha256 = Sha256Hash::from_str(
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
        )
        .expect("invalid sha256");

        assert_eq!(
            actual_sha256, expected_sha256,
            "sha256 does not match expected"
        );
    }
}
