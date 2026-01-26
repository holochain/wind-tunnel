use happ_builder::{BuildOptions, HappBuilderResult, build_happs, required_tools_available};
use std::{path::PathBuf, str::FromStr};

/// Build the required DNA(s) and hApps for the scenario.
/// The built DNA(s) will appear as `/dnas/<scenario-name>/<dna-name>.dna` from the project root.
/// The built hApp(s) will appear as `/happs/<scenario-name>/<happ-name>.happ` from the project root.
fn main() -> HappBuilderResult {
    let scenario_dir = PathBuf::from_str(std::env!("CARGO_MANIFEST_DIR")).unwrap();
    let dir_name = scenario_dir.file_name().unwrap().to_str().unwrap();
    let package_name = std::env!("CARGO_PKG_NAME");
    if package_name != dir_name {
        // The directory name is used as the Nix package name which is in turn used with `cargo run`
        // inside the Nix package. Therefore, panic during the build if the directory name does not
        // match the package name.
        panic!(
            "The package name of the scenario '{package_name}' does not match the directory name '{dir_name}'",
        );
    }

    println!("cargo:rerun-if-env-changed=SKIP_HAPP_BUILD");
    if std::env::var("SKIP_HAPP_BUILD").is_ok() {
        return Ok(());
    }

    if !required_tools_available() {
        println!("cargo:warning=Missing required tools for building hApps. Skipping hApp build.");
        return Ok(());
    }

    build_happs(BuildOptions::default())
}
