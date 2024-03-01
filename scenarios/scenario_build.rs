use anyhow::Context;
use holochain_types::dna::ZomeDependency;
use holochain_types::prelude::Timestamp;
use std::env;
use std::path::{Path, PathBuf};
use toml::Table;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../scenario_build.rs");
    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let package_name = env::var("CARGO_PKG_NAME").unwrap();
    let target_dir = find_target_dir(&manifest_dir)?;
    let out_dir = env::var("OUT_DIR").unwrap();

    let cargo_toml = Path::new(&manifest_dir).join("Cargo.toml");
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

    // Permit a table for a single required DNA.
    if let Some(required_dna) =
        required_dna_section.and_then(|required_dna| required_dna.as_table())
    {
        let dna_name = get_name_from_required_dna_table(&required_dna);
        let zome_names = get_zome_names_from_required_dna_table(&required_dna);

        build_required_dna(&manifest_dir, &package_name, &target_dir, &out_dir, &dna_name, &zome_names)
            .context(format!("Failed to build coordinator DNA - {}", dna_name))?;
    }

    // Expect an array for multiple required DNAs.
    if let Some(required_dna_array) =
        required_dna_section.and_then(|required_dna| required_dna.as_array())
    {
        for required_dna in required_dna_array.into_iter().map(|required_dna| {
            required_dna
                .as_table()
                .expect("Expected required-dna to be a table")
        }) {
            let dna_name = get_name_from_required_dna_table(required_dna);
            let zome_names = get_zome_names_from_required_dna_table(&required_dna);

            build_required_dna(&manifest_dir, &package_name, &target_dir, &out_dir, &dna_name, &zome_names)
                .context(format!("Failed to build coordinator DNA - {}", dna_name))?;
        }
    }

    Ok(())
}

fn find_target_dir(manifest_dir: &str) -> anyhow::Result<PathBuf> {
    let target_dir = Path::new(manifest_dir).join("../../target");
    if !target_dir.exists() {
        anyhow::bail!("Target directory not found at {}", target_dir.display());
    }

    Ok(target_dir)
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
        .into_iter()
        .filter_map(|zome_name| zome_name.as_str().map(|s| s.to_string()))
        .collect()
}

fn build_required_dna(
    scenario_manifest_dir: &str,
    scenario_package_name: &str,
    target_dir: &PathBuf,
    out_dir: &str,
    dna_name: &str,
    zome_names: &[String],
) -> anyhow::Result<()> {
    let mut coordinator_manifests = vec![];
    let mut integrity_manifests = vec![];

    for zome_name in zome_names {
        let zome_dir = Path::new(scenario_manifest_dir)
            .join("../../zomes")
            .join(&zome_name);
        if !zome_dir.exists() {
            anyhow::bail!("Zome directory not found at {}", zome_dir.display());
        }

        let mut integrity_exists = false;
        let integrity_dir = zome_dir.join("integrity");
        if integrity_dir.exists() {
            // Ensure the build script is re-run if the integrity zome changes
            print_rerun_for_package(&integrity_dir);

            build_wasm(&integrity_dir)?;
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

            build_wasm(&coordinator_dir)?;
            let wasm_file = find_wasm(target_dir, dna_name, "coordinator")?;
            coordinator_manifests.push(holochain_types::dna::ZomeManifest {
                name: format!("{}_coordinator", zome_name).into(),
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

    let dna_manifest_workdir = Path::new(out_dir).join(dna_name);
    if !dna_manifest_workdir.exists() {
        std::fs::create_dir(&dna_manifest_workdir).context("Failed to create DNA manifest workdir")?;
    }
    let dna_manifest_path = dna_manifest_workdir.join("dna.yaml");
    let dna_manifest_str =
        serde_yaml::to_string(&manifest).context("Failed to serialize DNA manifest")?;
    std::fs::write(&dna_manifest_path, dna_manifest_str).context("Failed to write DNA manifest")?;

    let dna_out_dir = Path::new(scenario_manifest_dir).join(format!("../../dnas/{}", scenario_package_name));
    if !dna_out_dir.exists() {
        std::fs::create_dir(&dna_out_dir).context("Failed to create DNA out dir")?;
    }
    let mut pack_cmd = std::process::Command::new("hc");
    pack_cmd
        .current_dir(out_dir)
        .arg("dna")
        .arg("pack")
        .arg("--output")
        .arg(dna_out_dir.to_str().unwrap())
        .arg(dna_manifest_workdir.to_str().unwrap());

    if !pack_cmd
        .status()
        .context("Failed run `hc dna pack`")?
        .success() {
        anyhow::bail!("`hc dna pack` command failed");
    }

    Ok(())
}

fn build_wasm(coordinator_dir: &PathBuf) -> anyhow::Result<()> {
    let mut build_cmd = wasm_build_command(&coordinator_dir.to_str().unwrap());
    if !build_cmd
        .status()
        .context("could not run cargo build")?
        .success()
    {
        anyhow::bail!("cargo build command failed");
    }

    Ok(())
}

fn wasm_build_command(build_dir: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new("cargo");

    cmd.current_dir(build_dir)
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_BUILD_RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .arg("build")
        .arg("--release")
        .arg("--target")
        .arg("wasm32-unknown-unknown");

    cmd
}

/// `kind` is either "coordinator" or "integrity"
fn find_wasm(target_dir: &PathBuf, name: &str, kind: &str) -> anyhow::Result<PathBuf> {
    let wasm_path = target_dir
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(format!("{}_{}.wasm", name, kind));
    if !wasm_path.exists() {
        anyhow::bail!("Wasm file not found at {}", wasm_path.display());
    }

    Ok(wasm_path)
}

fn print_rerun_for_package(package_dir: &PathBuf) {
    println!("cargo:rerun-if-changed={}", package_dir.join("Cargo.toml").display());
    walkdir::WalkDir::new(package_dir.join("src"))
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.path().extension().map_or(false, |ext| ext == "rs"))
        .for_each(|e| println!("cargo:rerun-if-changed={}", e.path().display()));
}
