use anyhow::Context;
use holochain_types::dna::ZomeDependency;
use holochain_types::prelude::Timestamp;
use std::env;
use std::io::Read;
use std::path::{Path, PathBuf};
use toml::Table;

pub type HappBuilderResult = anyhow::Result<()>;

pub struct BuildOptions {
    pub package_name: String,
    pub manifest_dir: PathBuf,
    pub out_dir: PathBuf,
    pub target_dir: Option<PathBuf>,
    pub zomes_dir: Option<PathBuf>,
    pub dna_target_dir: Option<PathBuf>,
    pub happ_target_dir: Option<PathBuf>,
}

impl Default for BuildOptions {
    fn default() -> Self {
        BuildOptions {
            package_name: env::var("CARGO_PKG_NAME").unwrap(),
            manifest_dir: env::var("CARGO_MANIFEST_DIR").unwrap().into(),
            out_dir: env::var("OUT_DIR").unwrap().into(),
            target_dir: None,
            zomes_dir: None,
            dna_target_dir: None,
            happ_target_dir: None,
        }
    }
}

/// Build the happs specified by the `Cargo.toml` found in the `manifest_dir` directory from the [BuildOptions].
/// The built DNA(s) will appear as `/dnas/<scenario-name>/<dna-name>.dna` from the project root.
/// The built hApp(s) will appear as `/happs/<scenario-name>/<happ-name>.happ` from the project root.
///
/// Example build script integration in `build.rs`:
/// ```rust,no_run
/// use std::env;
/// use happ_builder::{build_happs, BuildOptions};
///
/// build_happs(BuildOptions::default()).unwrap();
/// ```
pub fn build_happs(build_options: BuildOptions) -> anyhow::Result<()> {
    print_rerun_for_package(&build_options.manifest_dir);

    let target_dir = build_options
        .target_dir
        .unwrap_or_else(|| find_target_dir(&build_options.manifest_dir).unwrap());

    let zomes_dir = build_options
        .zomes_dir
        .unwrap_or_else(|| build_options.manifest_dir.join("../../zomes"));
    let dna_target_dir = build_options
        .dna_target_dir
        .unwrap_or_else(|| build_options.manifest_dir.join("../../dnas"));
    let happ_target_dir = build_options
        .happ_target_dir
        .unwrap_or_else(|| build_options.manifest_dir.join("../../happs"));

    let cargo_toml = build_options.manifest_dir.join("Cargo.toml");
    if !cargo_toml.exists() {
        anyhow::bail!("Cargo.toml not found at {}", cargo_toml.display());
    }

    let toml = std::fs::read_to_string(&cargo_toml)
        .expect("Could not read Cargo.toml")
        .parse::<Table>()
        .expect("Could not parse Cargo.toml as a TOML table");

    let required_dna_section = toml
        .get("package")
        .expect("Cargo.toml is missing a [package] table")
        .get("metadata")
        .and_then(|metadata| metadata.get("required-dna"));

    let mut built_dnas = vec![];

    // Permit a table for a single required DNA.
    if let Some(required_dna) =
        required_dna_section.and_then(|required_dna| required_dna.as_table())
    {
        let dna_name = get_name_from_required_dna_table(required_dna);
        let zome_names = get_zome_names_from_required_dna_table(required_dna);

        let built_path = build_required_dna(
            &zomes_dir,
            &dna_target_dir,
            &build_options.package_name,
            &target_dir,
            &build_options.out_dir,
            &dna_name,
            &zome_names,
        )
        .context(format!("Failed to build coordinator DNA - {}", dna_name))?;

        built_dnas.push((dna_name, built_path));
    }

    // Expect an array for multiple required DNAs.
    if let Some(required_dna_array) =
        required_dna_section.and_then(|required_dna| required_dna.as_array())
    {
        for required_dna in required_dna_array.iter().map(|required_dna| {
            required_dna
                .as_table()
                .expect("Expected required-dna to be a table")
        }) {
            let dna_name = get_name_from_required_dna_table(required_dna);
            let zome_names = get_zome_names_from_required_dna_table(required_dna);

            let built_path = build_required_dna(
                &zomes_dir,
                &dna_target_dir,
                &build_options.package_name,
                &target_dir,
                &build_options.out_dir,
                &dna_name,
                &zome_names,
            )
            .context(format!("Failed to build coordinator DNA - {}", dna_name))?;

            built_dnas.push((dna_name, built_path));
        }
    }

    let required_happs_section = toml
        .get("package")
        .expect("Cargo.toml is missing a [package] table")
        .get("metadata")
        .and_then(|metadata| metadata.get("required-happ"));

    // Permit a table for a single required hApp.
    if let Some(required_happ) =
        required_happs_section.and_then(|required_happ| required_happ.as_table())
    {
        let happ_name = get_name_from_required_happ_table(required_happ);
        let dnas = get_dna_names_from_required_happ_table(required_happ);

        build_required_happ(
            &build_options.out_dir,
            &build_options.package_name,
            &happ_target_dir,
            &happ_name,
            &dnas,
            &built_dnas,
        )?;
    }

    // Expect an array for multiple required hApps.
    if let Some(required_happs_array) =
        required_happs_section.and_then(|required_happ| required_happ.as_array())
    {
        for required_happ in required_happs_array.iter().map(|required_happ| {
            required_happ
                .as_table()
                .expect("Expected required-happ to be a table")
        }) {
            let happ_name = get_name_from_required_happ_table(required_happ);
            let dnas = get_dna_names_from_required_happ_table(required_happ);

            build_required_happ(
                &build_options.out_dir,
                &build_options.package_name,
                &happ_target_dir,
                &happ_name,
                &dnas,
                &built_dnas,
            )?;
        }
    }

    Ok(())
}

fn get_dna_names_from_required_happ_table(required_happ: &Table) -> Vec<String> {
    required_happ
        .get("dnas")
        .and_then(|dnas| dnas.as_array())
        .expect("No dnas specified for required hApp")
        .iter()
        .filter_map(|dna_name| dna_name.as_str().map(|s| s.to_string()))
        .collect::<Vec<String>>()
}

fn get_name_from_required_happ_table(required_happ: &Table) -> String {
    required_happ
        .get("name")
        .and_then(|name| name.as_str().map(|s| s.to_string()))
        .expect("Missing name for required hApp")
}

fn find_target_dir(manifest_dir: &Path) -> anyhow::Result<PathBuf> {
    let target_dir = manifest_dir.join("../../wasm-target");

    if !target_dir.exists() {
        std::fs::create_dir(&target_dir).context("Failed to create target directory")?;
    }

    Ok(target_dir.canonicalize().unwrap())
}

fn get_name_from_required_dna_table(table: &Table) -> String {
    table
        .get("name")
        .and_then(|name| name.as_str().map(|s| s.to_string()))
        .expect("Missing name for required DNA")
}

fn get_zome_names_from_required_dna_table(table: &Table) -> Vec<String> {
    table
        .get("zomes")
        .and_then(|z| z.as_array())
        .expect("No zomes specified for required DNA")
        .iter()
        .filter_map(|zome_name| zome_name.as_str().map(|s| s.to_string()))
        .collect()
}

/// Returns the path to the built DNA.
fn build_required_dna(
    zomes_dir: &Path,
    dna_target_dir: &Path,
    scenario_package_name: &str,
    target_dir: &Path,
    out_dir: &Path,
    dna_name: &str,
    zome_names: &[String],
) -> anyhow::Result<PathBuf> {
    let mut coordinator_manifests = vec![];
    let mut integrity_manifests = vec![];

    for zome_name in zome_names {
        let zome_dir = zomes_dir.join(zome_name).canonicalize().unwrap();
        if !zome_dir.exists() {
            anyhow::bail!("Zome directory not found at {}", zome_dir.display());
        }

        let mut integrity_exists = false;
        let integrity_dir = zome_dir.join("integrity");
        if integrity_dir.exists() {
            // Ensure the build script is re-run if the integrity zome changes
            print_rerun_for_package(&integrity_dir);

            build_wasm(&integrity_dir, target_dir)?;
            let wasm_file = find_wasm(target_dir, dna_name, "integrity")?;
            integrity_manifests.push(holochain_types::dna::ZomeManifest {
                name: format!("{}_integrity", zome_name).into(),
                hash: None,
                location: holochain_types::prelude::ZomeLocation::Bundled(
                    wasm_file
                        .canonicalize()
                        .context("Failed to canonicalize wasm file path")?,
                ),
                dependencies: None,
                dylib: None,
            });
            integrity_exists = true;
        }

        let coordinator_dir = zome_dir.join("coordinator");
        if coordinator_dir.exists() {
            // Ensure the build script is re-run if the coordinator zome changes
            print_rerun_for_package(&coordinator_dir);

            build_wasm(&coordinator_dir, target_dir)?;
            let wasm_file = find_wasm(target_dir, dna_name, "coordinator")?;
            coordinator_manifests.push(holochain_types::dna::ZomeManifest {
                name: zome_name.to_string().into(),
                hash: None,
                location: holochain_types::prelude::ZomeLocation::Bundled(
                    wasm_file
                        .canonicalize()
                        .context("Failed to canonicalize wasm file path")?,
                ),
                dependencies: integrity_exists.then(|| {
                    vec![ZomeDependency {
                        name: format!("{}_integrity", zome_name).into(),
                    }]
                }),
                dylib: None,
            });
        }
    }

    let manifest = holochain_types::dna::DnaManifest::V1(holochain_types::dna::DnaManifestV1 {
        name: dna_name.to_string(),
        integrity: holochain_types::dna::IntegrityManifest {
            network_seed: None,
            properties: None,
            origin_time: Timestamp::now().into(),
            zomes: integrity_manifests,
        },
        coordinator: holochain_types::dna::CoordinatorManifest {
            zomes: coordinator_manifests,
        },
    });

    let dna_manifest_workdir = dna_target_dir.join(scenario_package_name).join(dna_name);
    if !dna_manifest_workdir.exists() {
        std::fs::create_dir_all(&dna_manifest_workdir)
            .context("Failed to create DNA manifest workdir")?;
    }
    let dna_manifest_path = dna_manifest_workdir.clone().join("dna.yaml");
    let dna_manifest_str =
        serde_yaml::to_string(&manifest).context("Failed to serialize DNA manifest")?;
    std::fs::write(dna_manifest_path, dna_manifest_str).context("Failed to write DNA manifest")?;

    let dna_out_dir = dna_target_dir.join(scenario_package_name);
    if !dna_out_dir.exists() {
        std::fs::create_dir_all(&dna_out_dir).context("Failed to create DNA out dir")?;
    }

    let mut pack_cmd = std::process::Command::new("hc");
    pack_cmd
        .current_dir(out_dir)
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

    Ok(dna_out_dir.join(format!("{}.dna", dna_name)))
}

fn build_wasm(coordinator_dir: &Path, target_dir: &Path) -> anyhow::Result<()> {
    let mut build_cmd = wasm_build_command(coordinator_dir.to_str().unwrap(), target_dir);

    let mut child = build_cmd
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let mut buf = [0u8; 1024];
    let mut stderr = child.stderr.take().unwrap();
    while let Ok(amt) = stderr.read(&mut buf) {
        if amt == 0 {
            break;
        }
        println!("cargo:warning={}", std::str::from_utf8(&buf).unwrap());
    }

    if !child.wait().context("could not run cargo build")?.success() {
        anyhow::bail!("cargo build command failed");
    }

    Ok(())
}

fn wasm_build_command(build_dir: &str, target_dir: &Path) -> std::process::Command {
    let mut cmd = std::process::Command::new("cargo");

    cmd.current_dir(build_dir)
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_BUILD_RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
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

/// `kind` is either "coordinator" or "integrity"
fn find_wasm(target_dir: &Path, name: &str, kind: &str) -> anyhow::Result<PathBuf> {
    let wasm_path = target_dir
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(format!("{}_{}.wasm", name, kind));
    if !wasm_path.exists() {
        anyhow::bail!("Wasm file not found at {}", wasm_path.display());
    }

    Ok(wasm_path)
}

fn print_rerun_for_package(package_dir: &Path) {
    println!(
        "cargo:rerun-if-changed={}",
        package_dir.join("Cargo.toml").display()
    );
    walkdir::WalkDir::new(package_dir.join("src"))
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file() && e.path().extension().map_or(false, |ext| ext == "rs")
        })
        .for_each(|e| println!("cargo:rerun-if-changed={}", e.path().display()));
}

fn build_required_happ(
    out_dir: &Path,
    scenario_package_name: &str,
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
                .context(format!("DNA not found: {}", dna_name))?;

            let role_manifest = format!(
                r#"
- name: {}
  provisioning:
    strategy: create
    deferred: false
  dna:
    bundled: {}
    modifiers:
      network_seed: ~
      properties: ~
      origin_time: ~
      quantum_time: ~
    installed_hash: ~
    clone_limit: 0
    "#,
                dna_name,
                dna.1.display()
            );

            Ok(role_manifest.to_string())
        })
        .collect::<anyhow::Result<Vec<String>>>()?
        .into_iter()
        .fold(String::new(), |acc, role| acc + "\n" + &role);

    let manifest = format!(
        r#"
manifest_version: '1'
name: {}
description: ~
roles:
{}
"#,
        happ_name, roles
    );

    let happ_manifest_workdir = happ_target_dir.join(scenario_package_name).join(happ_name);
    if !happ_manifest_workdir.exists() {
        std::fs::create_dir_all(&happ_manifest_workdir)
            .context("Failed to create hApp manifest workdir")
            .unwrap();
    }

    let happ_manifest_path = happ_manifest_workdir.join("happ.yaml");
    std::fs::write(happ_manifest_path, manifest)
        .context("Failed to write hApp manifest")
        .unwrap();

    let happ_out_dir = happ_target_dir.join(scenario_package_name);
    if !happ_out_dir.exists() {
        std::fs::create_dir(&happ_out_dir)
            .context("Failed to create hApp out dir")
            .unwrap();
    }

    let mut pack_cmd = std::process::Command::new("hc");

    pack_cmd
        .current_dir(out_dir)
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
