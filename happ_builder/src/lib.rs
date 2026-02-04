mod builder;
mod fetch;
mod manifest;
mod options;

use self::builder::HappBuilder;
pub use self::options::HappManagerOptions;
use crate::fetch::HappFetcher;
use crate::manifest::Metadata;
use anyhow::Context;
use std::path::Path;

pub type HappBuilderResult = anyhow::Result<()>;

/// Manager for building and fetching hApps
///
/// This struct provides methods to build and fetch hApps based on the provided options.
///
/// Example usage:
///
/// ```rust,no_run
/// use happ_builder::{HappManager, HappManagerOptions};
///
/// HappManager::from(HappManagerOptions::default())
///    .ensure_happs_available()
///    .expect("Failed to fetch and build hApps");
/// ```
#[derive(Default)]
pub struct HappManager {
    options: HappManagerOptions,
}

impl From<HappManagerOptions> for HappManager {
    fn from(options: HappManagerOptions) -> Self {
        Self { options }
    }
}

impl HappManager {
    /// Checks whether the required tools for this builder are available.
    ///
    /// Returns an error if any required tool is missing.
    pub fn required_tools_available(&self) -> HappBuilderResult {
        if let Err(err) = which::which("hc") {
            println!("cargo:warning=Could not find 'hc' in PATH: {}", err);
            anyhow::bail!("Could not find required hc binary");
        }

        if let Err(err) = which::which("cargo") {
            println!("cargo:warning=Could not find 'cargo' in PATH: {}", err);
            anyhow::bail!("Could not find required cargo binary");
        }

        Ok(())
    }

    /// Ensure all hApps defined in the manifest are available.
    ///
    /// Fetch-required hApps are downloaded, and required DNAs/hApps are built; both can be present.
    ///
    /// Build and fetch the happs specified by the `Cargo.toml` found in the `manifest_dir` directory from the [`HappManagerOptions`].
    /// The built DNA(s) will appear as `/dnas/<scenario-name>/<dna-name>.dna` from the project root.
    /// The built or fetched hApp(s) will appear as `/happs/<scenario-name>/<happ-name>.happ` from the project root.
    ///
    /// Example build script integration in `build.rs`:
    ///
    /// ```rust,no_run
    /// use happ_builder::{HappManager, HappManagerOptions};
    ///
    /// HappManager::from(HappManagerOptions::default())
    ///     .ensure_happs_available()
    ///     .expect("Failed to fetch and build hApps");
    /// ```
    ///
    /// hApps can be specified in the Cargo.toml manifest as follows:
    ///
    /// ```toml
    /// # to be fetched
    /// [[package.metadata.fetch-required-happ]]
    /// name = "foo"
    /// url = "https://github.com/holochain/happs/foo.happ"
    /// sha256 = "e3b0c44298fc1c149afbfc6c5d6a8e9b7f4f5c6d78e9f0a1b2c3d4e5f6a7b890"
    ///
    /// # to be built
    /// [[package.metadata.required-dna]]
    /// name = "timed_and_validated"
    /// zomes = ["timed_and_validated"]
    ///
    /// # to be built
    /// [[package.metadata.required-happ]]
    /// name = "timed_and_validated"
    /// dnas = ["timed_and_validated"]
    /// ```
    pub fn ensure_happs_available(&self) -> HappBuilderResult {
        self.init_build_dirs()?;

        // parse manifest metadata
        let metadata = self.parse_manifest_metadata()?;

        // build dependencies
        HappBuilder::new(
            &self.options,
            &metadata.required_dnas(),
            &metadata.required_happs(),
        )
        .build_happs()?;

        // fetch
        HappFetcher::new(&self.options, &metadata.fetch_required_happ()).fetch_all()?;

        Ok(())
    }

    /// Initialize all the build dirs by creating them if they don't exist
    fn init_build_dirs(&self) -> HappBuilderResult {
        self.init_dir(&self.options.out_dir)?;
        self.init_dir(&self.options.target_dir)?;
        self.init_dir(&self.options.zomes_dir)?;
        self.init_dir(&self.options.dna_target_dir)?;
        self.init_dir(&self.options.happ_target_dir)?;

        Ok(())
    }

    /// Initialize a directory by creating it if it doesn't exist
    fn init_dir(&self, dir: &Path) -> HappBuilderResult {
        if !dir.exists() {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }

    /// Load [`Metadata`] from the Cargo manifest file
    fn parse_manifest_metadata(&self) -> anyhow::Result<Metadata> {
        let cargo_toml = self.options.manifest_dir.join("Cargo.toml");
        if !cargo_toml.exists() {
            anyhow::bail!("Cargo.toml not found at {}", cargo_toml.display());
        }

        let manifest_str = std::fs::read_to_string(cargo_toml)?;
        let manifest: manifest::CargoToml =
            toml::from_str(&manifest_str).context("failed to parse Cargo.toml")?;
        Ok(manifest.package.metadata)
    }
}
