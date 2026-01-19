use crate::HappBuilderResult;
use crate::manifest::{FetchRequiredHapp, Sha256Hash};
use anyhow::Context;
use sha2::Sha256;
use std::path::Path;
use std::time::Duration;

pub struct HappFetcher<'a> {
    /// Directory to fetch happs into
    pub happ_target_dir: &'a Path,
    /// Happs to fetch
    pub happs: &'a [FetchRequiredHapp],
}

impl<'a> HappFetcher<'a> {
    /// Create a new [`HappFetcher`]
    pub fn new(happ_target_dir: &'a Path, happs: &'a [FetchRequiredHapp]) -> Self {
        Self {
            happ_target_dir,
            happs,
        }
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
            .happ_target_dir
            .join(format!("{name}.happ", name = happ.name));

        // do not fetch if already exists
        if out_path.exists() {
            if let Ok(existing_sha256) = Self::sha256_file(&out_path) {
                if existing_sha256 == happ.sha256 {
                    return Ok(());
                }
            }
        }

        /* requires ed 2024
        if out_path.exists()
            && let Ok(existing_sha256) = Self::sha256_file(&out_path)
            && existing_sha256 == happ.sha256
        {
            return Ok(());
        }
        */

        let agent = ureq::config::Config::builder()
            .timeout_global(Some(Duration::from_secs(60)))
            .build()
            .new_agent();
        let response = agent.get(&happ.url).call()?;

        // get the response body as bytes
        let mut body = response.into_body();

        let mut writer = std::fs::File::create(&out_path).context("failed to create happ file")?;
        if let Err(err) =
            std::io::copy(&mut body.as_reader(), &mut writer).context("failed to write happ file")
        {
            // cleanup partial file
            std::fs::remove_file(&out_path).ok();
            return Err(err);
        }

        // verify downloaded file sha256
        let downloaded_sha256 =
            Self::sha256_file(&out_path).context("failed to compute sha256 of downloaded happ")?;
        if downloaded_sha256 != happ.sha256 {
            // cleanup invalid file
            std::fs::remove_file(&out_path).ok();
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

    const SAMPLE_URL: &str = "https://github.com/holochain/dino-adventure/releases/download/v0.3.0-dev.0/dino-adventure-v0.3.0-dev.0.happ";

    #[test]
    fn should_fetch_happ() {
        let tempdir = tempfile::tempdir().expect("failed to create temp dir");
        let config = vec![FetchRequiredHapp {
            name: "dino-adventure".to_string(),
            url: SAMPLE_URL.to_string(),
            sha256: Sha256Hash::from_str(
                "1eafa0d852d9e96e54f0b6969fb06de83989ece0059bc2b376884ac52fb6a63a",
            )
            .expect("invalid sha256"),
        }];

        let fetcher = HappFetcher::new(tempdir.path(), &config);
        fetcher.fetch_all().expect("failed to fetch happ");

        let fetched_happ_path = tempdir.path().join("dino-adventure.happ");
        assert!(
            fetched_happ_path.exists(),
            "fetched happ file does not exist"
        );
    }

    #[test]
    fn should_compute_sha256() {
        let tempdir = tempfile::tempdir().expect("failed to create temp dir");
        let file_path = tempdir.path().join("test_file.txt");
        std::fs::write(&file_path, b"hello world").expect("failed to write test file");

        let actual_sha256 = HappFetcher::sha256_file(&file_path).expect("failed to compute sha256");
        println!("Actual sha256: {}", actual_sha256);
        let expected_sha256 = Sha256Hash::from_str(
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
        )
        .expect("invalid sha256");

        assert_eq!(
            actual_sha256, expected_sha256,
            "sha256 does not match expected"
        );
    }

    #[test]
    fn should_refetch_if_hash_mismatch() {
        let tempdir = tempfile::tempdir().expect("failed to create temp dir");
        let file_path = tempdir.path().join("dino-adventure.happ");
        std::fs::write(&file_path, b"corrupted data").expect("failed to write test file");

        let config = vec![FetchRequiredHapp {
            name: "dino-adventure".to_string(),
            url: SAMPLE_URL.to_string(),
            sha256: Sha256Hash::from_str(
                "1eafa0d852d9e96e54f0b6969fb06de83989ece0059bc2b376884ac52fb6a63a",
            )
            .expect("invalid sha256"),
        }];

        let fetcher = HappFetcher::new(tempdir.path(), &config);
        fetcher.fetch_all().expect("failed to fetch happ");

        assert!(
            file_path.exists(),
            "fetched file does not exist after hash mismatch"
        );
        // check hash
        let sha256 = HappFetcher::sha256_file(&file_path).expect("failed to compute sha256");
        assert_eq!(
            sha256, config[0].sha256,
            "sha256 does not match expected after re-fetch"
        );
    }

    #[test]
    fn should_fail_if_hash_mismatch_manifest() {
        let tempdir = tempfile::tempdir().expect("failed to create temp dir");
        let config = vec![FetchRequiredHapp {
            name: "dino-adventure".to_string(),
            url: SAMPLE_URL.to_string(),
            sha256: Sha256Hash::from_str(
                "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
            )
            .expect("invalid sha256"),
        }];

        let fetcher = HappFetcher::new(tempdir.path(), &config);
        let result = fetcher.fetch_all();
        assert!(result.is_err(), "expected error due to sha256 mismatch");
    }
}
