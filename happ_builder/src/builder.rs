use crate::HappManagerOptions;
use crate::manifest::{RequiredDna, RequiredHapp};
use anyhow::Context;
use holochain_types::dna::ZomeDependency;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// A builder for hApps specified in a scenario's `Cargo.toml`.
pub struct HappBuilder<'a> {
    options: &'a HappManagerOptions,
    required_dnas: &'a [RequiredDna],
    required_happs: &'a [RequiredHapp],
}

impl<'a> HappBuilder<'a> {
    /// Create a new `HappBuilder` with the given [HappManagerOptions].
    pub fn new(
        options: &'a HappManagerOptions,
        required_dnas: &'a [RequiredDna],
        required_happs: &'a [RequiredHapp],
    ) -> Self {
        Self {
            options,
            required_dnas,
            required_happs,
        }
    }
}

impl HappBuilder<'_> {
    /// Build the happs specified by the `Cargo.toml` found in the `manifest_dir` directory from the [HappManagerOptions].
    /// The built DNA(s) will appear as `/dnas/<scenario-name>/<dna-name>.dna` from the project root.
    /// The built hApp(s) will appear as `/happs/<scenario-name>/<happ-name>.happ` from the project root.
    pub fn build_happs(&self) -> anyhow::Result<()> {
        self.print_rerun_for_package(&self.options.manifest_dir);

        let mut built_dnas = vec![];

        // Permit a table for a single required DNA.

        for dna in self.required_dnas {
            let built_path = self
                .build_required_dna(&dna.name, &dna.zomes)
                .context(format!(
                    "Failed to build coordinator DNA - {name}",
                    name = dna.name
                ))?;

            built_dnas.push((dna.name.clone(), built_path));
        }

        for happ in self.required_happs {
            self.build_required_happ(
                &self.options.happ_target_dir,
                &happ.name,
                &happ.dnas,
                &built_dnas,
            )?;
        }

        Ok(())
    }

    /// Returns the path to the built DNA.
    fn build_required_dna(&self, dna_name: &str, zome_names: &[String]) -> anyhow::Result<PathBuf> {
        let mut coordinator_manifests = vec![];
        let mut integrity_manifests = vec![];

        for zome_name in zome_names {
            let zome_dir = self.options.zomes_dir.join(zome_name).canonicalize()?;

            let mut integrity_exists = false;
            let integrity_dir = zome_dir.join("integrity");
            if integrity_dir.exists() {
                // Ensure the build script is re-run if the integrity zome changes
                self.print_rerun_for_package(&integrity_dir);

                self.build_wasm(&integrity_dir, &self.options.target_dir)?;
                let wasm_file = self.find_wasm(&self.options.target_dir, zome_name, "integrity")?;
                integrity_manifests.push(holochain_types::dna::ZomeManifest {
                    name: format!("{zome_name}_integrity").into(),
                    hash: None,
                    path: wasm_file
                        .canonicalize()
                        .context("Failed to canonicalize wasm file path")?
                        .to_str()
                        .context("Failed to convert wasm file path to str")?
                        .to_string(),
                    dependencies: None,
                });
                integrity_exists = true;
            }

            let coordinator_dir = zome_dir.join("coordinator");
            if coordinator_dir.exists() {
                // Ensure the build script is re-run if the coordinator zome changes
                self.print_rerun_for_package(&coordinator_dir);

                self.build_wasm(&coordinator_dir, &self.options.target_dir)?;
                let wasm_file =
                    self.find_wasm(&self.options.target_dir, zome_name, "coordinator")?;
                coordinator_manifests.push(holochain_types::dna::ZomeManifest {
                    name: zome_name.to_string().into(),
                    hash: None,
                    path: wasm_file
                        .canonicalize()
                        .context("Failed to canonicalize wasm file path")?
                        .to_str()
                        .context("Failed to convert wasm file path to str")?
                        .to_string(),
                    dependencies: integrity_exists.then(|| {
                        vec![ZomeDependency {
                            name: format!("{zome_name}_integrity").into(),
                        }]
                    }),
                });
            }
        }

        let manifest = holochain_types::dna::DnaManifest::V0(holochain_types::dna::DnaManifestV0 {
            name: dna_name.to_string(),
            integrity: holochain_types::dna::IntegrityManifest {
                network_seed: None,
                properties: None,
                zomes: integrity_manifests,
            },
            coordinator: holochain_types::dna::CoordinatorManifest {
                zomes: coordinator_manifests,
            },
        });

        let dna_manifest_workdir = self
            .options
            .dna_target_dir
            .join(&self.options.package_name)
            .join(dna_name);
        if !dna_manifest_workdir.exists() {
            std::fs::create_dir_all(&dna_manifest_workdir)
                .context("Failed to create DNA manifest workdir")?;
        }
        let dna_manifest_path = dna_manifest_workdir.clone().join("dna.yaml");
        let dna_manifest_str =
            serde_yaml::to_string(&manifest).context("Failed to serialize DNA manifest")?;
        std::fs::write(dna_manifest_path, dna_manifest_str)
            .context("Failed to write DNA manifest")?;

        let dna_out_dir = self.options.dna_target_dir.join(&self.options.package_name);
        if !dna_out_dir.exists() {
            std::fs::create_dir_all(&dna_out_dir).context("Failed to create DNA out dir")?;
        }

        let mut pack_cmd = std::process::Command::new("hc");
        pack_cmd
            .current_dir(&self.options.out_dir)
            .arg("dna")
            .arg("pack")
            .arg("--output")
            // Putting files in locations other than `out_dir` is not recommended in build scripts, but `dnas` directory is dedicated to this purpose.
            .arg(dna_out_dir.to_str().unwrap())
            .arg(dna_manifest_workdir.to_str().unwrap());

        if !pack_cmd
            .status()
            .context("Failed to run `hc dna pack`")?
            .success()
        {
            anyhow::bail!("`hc dna pack` command failed");
        }

        println!(
            "cargo:warning=Built DNA '{}' and placed it in {}",
            dna_name,
            dna_out_dir.display()
        );

        Ok(dna_out_dir.join(format!("{dna_name}.dna")))
    }

    fn build_wasm(&self, coordinator_dir: &Path, target_dir: &Path) -> anyhow::Result<()> {
        let mut build_cmd = self.wasm_build_command(coordinator_dir, target_dir);

        let mut child = build_cmd.stderr(std::process::Stdio::piped()).spawn()?;

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stderr handle"))?;
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            println!("cargo:warning={line}");
        }

        if !child.wait().context("could not run cargo build")?.success() {
            anyhow::bail!("cargo build command failed");
        }

        Ok(())
    }

    fn wasm_build_command(&self, build_dir: &Path, target_dir: &Path) -> std::process::Command {
        let mut cmd = std::process::Command::new("cargo");

        cmd.current_dir(build_dir)
            .env_remove("RUSTFLAGS")
            .env_remove("CARGO_BUILD_RUSTFLAGS")
            .env_remove("CARGO_ENCODED_RUSTFLAGS")
            .env("RUSTFLAGS", "--cfg getrandom_backend=\"custom\"")
            .arg("build")
            .arg("--target-dir")
            .arg(target_dir)
            .arg("--locked")
            .arg("--quiet")
            .arg("--release")
            .arg("--target")
            .arg("wasm32-unknown-unknown");

        cmd
    }

    /// Find the built wasm file for the given DNA name and kind.
    ///
    /// `kind` is either "coordinator" or "integrity"
    fn find_wasm(&self, target_dir: &Path, name: &str, kind: &str) -> anyhow::Result<PathBuf> {
        let wasm_path = target_dir
            .join("wasm32-unknown-unknown")
            .join("release")
            .join(format!("{name}_{kind}.wasm"));
        if !wasm_path.exists() {
            anyhow::bail!("Wasm file not found at {}", wasm_path.display());
        }

        Ok(wasm_path)
    }

    fn print_rerun_for_package(&self, package_dir: &Path) {
        println!(
            "cargo:rerun-if-changed={}",
            package_dir.join("Cargo.toml").display()
        );
        walkdir::WalkDir::new(package_dir.join("src"))
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_file() && e.path().extension().is_some_and(|ext| ext == "rs")
            })
            .for_each(|e| println!("cargo:rerun-if-changed={}", e.path().display()));
    }

    fn build_required_happ(
        &self,
        happ_target_dir: &Path,
        happ_name: &str,
        dnas: &[String],
        all_dnas: &[(String, PathBuf)],
    ) -> anyhow::Result<()> {
        let roles = dnas
            .iter()
            .map(|dna_name| {
                let dna = all_dnas
                    .iter()
                    .find(|(name, _)| name == dna_name)
                    .context(format!("DNA not found: {dna_name}"))?;

                let role_manifest = format!(
                    r#"
- name: {dna_name}
  provisioning:
    strategy: create
    deferred: false
  dna:
    path: {path}
    modifiers:
      network_seed: ~
      properties: ~
    installed_hash: ~
    clone_limit: 0
    "#,
                    path = dna.1.display()
                );

                Ok(role_manifest.to_string())
            })
            .collect::<anyhow::Result<Vec<String>>>()?
            .into_iter()
            .fold(String::new(), |acc, role| acc + "\n" + &role);

        let manifest = format!(
            r#"
manifest_version: '0'
name: {happ_name}
description: ~
roles:
{roles}
"#
        );

        let happ_manifest_workdir = happ_target_dir
            .join(&self.options.package_name)
            .join(happ_name);
        if !happ_manifest_workdir.exists() {
            std::fs::create_dir_all(&happ_manifest_workdir)
                .context("Failed to create hApp manifest workdir")?;
        }

        let happ_manifest_path = happ_manifest_workdir.join("happ.yaml");
        std::fs::write(happ_manifest_path, manifest).context("Failed to write hApp manifest")?;

        let happ_out_dir = happ_target_dir.join(&self.options.package_name);
        if !happ_out_dir.exists() {
            std::fs::create_dir_all(&happ_out_dir).context("Failed to create hApp out dir")?;
        }

        let mut pack_cmd = std::process::Command::new("hc");

        pack_cmd
            .current_dir(&self.options.out_dir)
            .arg("app")
            .arg("pack")
            .arg("--output")
            // Putting files in locations other than `out_dir` is not recommended in build scripts, but the `happs` directory is dedicated to this purpose.
            .arg(happ_out_dir.to_str().unwrap())
            .arg(happ_manifest_workdir.to_str().unwrap());

        if !pack_cmd
            .status()
            .context("Failed to run `hc happ pack`")?
            .success()
        {
            anyhow::bail!("`hc happ pack` command failed");
        }

        println!(
            "cargo:warning=Built hApp '{}' and placed it in {}",
            happ_name,
            happ_out_dir.display()
        );

        Ok(())
    }
}
