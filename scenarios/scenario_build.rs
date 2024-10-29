use happ_builder::{build_happs, required_tools_available, BuildOptions, HappBuilderResult};

/// Build the required DNA(s) and hApps for the scenario.
/// The built DNA(s) will appear as `/dnas/<scenario-name>/<dna-name>.dna` from the project root.
/// The built hApp(s) will appear as `/happs/<scenario-name>/<happ-name>.happ` from the project root.
fn main() -> HappBuilderResult {
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
