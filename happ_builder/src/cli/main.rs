use anyhow::Context;
use happ_builder::{build_happs, BuildOptions};
use std::path::PathBuf;

/// For example: `cargo run --bin hb zome_call_single_value $(pwd)/scenarios/zome_call_single_value $(pwd)/zomes $(pwd)/wasm-target $(pwd)/test_build`
fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);

    let package_name = args.next().context("Argument 1 should be: package name")?;
    let manifest_dir = PathBuf::from(args.next().context("Argument 2 should be: manifest dir")?);
    let zomes_dir = PathBuf::from(args.next().context("Argument 3 should be: zomes dir")?);
    let target_dir = PathBuf::from(args.next().context("Argument 4 should be: target dir")?);

    let out_dir = PathBuf::from(args.next().context("Argument 5 should be: out dir")?);
    if !out_dir.exists() {
        std::fs::create_dir_all(&out_dir).context("Failed to create out dir")?;
    }

    build_happs(BuildOptions {
        package_name,
        manifest_dir,
        out_dir: out_dir.clone(),
        target_dir: Some(target_dir),
        zomes_dir: Some(zomes_dir),
        dna_target_dir: Some(out_dir.join("dnas")),
        happ_target_dir: Some(out_dir.join("happs")),
    })
}
