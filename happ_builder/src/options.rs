use std::env;
use std::path::PathBuf;

/// Options for configuring the [`super::HappManager`]
pub struct HappManagerOptions {
    pub package_name: String,
    pub manifest_dir: PathBuf,
    pub out_dir: PathBuf,
    pub target_dir: PathBuf,
    pub zomes_dir: PathBuf,
    pub dna_target_dir: PathBuf,
    pub happ_target_dir: PathBuf,
}

impl Default for HappManagerOptions {
    fn default() -> Self {
        let manifest_dir =
            PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
        let target_dir = manifest_dir.join("../../wasm-target");
        let zomes_dir = manifest_dir.join("../../zomes");
        let dna_target_dir = manifest_dir.join("../../dnas");
        let happ_target_dir = manifest_dir.join("../../happs");

        HappManagerOptions {
            package_name: env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME not set"),
            manifest_dir,
            out_dir: env::var("OUT_DIR").expect("OUT_DIR not set").into(),
            target_dir,
            zomes_dir,
            dna_target_dir,
            happ_target_dir,
        }
    }
}

impl HappManagerOptions {
    /// Set `package_name` option
    pub fn package_name(mut self, name: &str) -> Self {
        self.package_name = name.to_string();
        self
    }

    /// Set `manifest_dir` option
    pub fn manifest_dir(mut self, dir: PathBuf) -> Self {
        self.manifest_dir = dir;
        self
    }

    /// Set `out_dir` option
    pub fn out_dir(mut self, dir: PathBuf) -> Self {
        self.out_dir = dir;
        self
    }

    /// Set `target_dir` option
    pub fn target_dir(mut self, path: PathBuf) -> Self {
        self.target_dir = path;
        self
    }

    /// Set `zomes_dir` option
    pub fn zomes_dir(mut self, path: PathBuf) -> Self {
        self.zomes_dir = path;
        self
    }

    /// Set `dna_target_dir` option
    pub fn dna_target_dir(mut self, path: PathBuf) -> Self {
        self.dna_target_dir = path;
        self
    }

    /// Set `happ_target_dir` option
    pub fn happ_target_dir(mut self, path: PathBuf) -> Self {
        self.happ_target_dir = path;
        self
    }
}
