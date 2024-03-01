use std::env;

/// This build script is only used to ensure that the wasm build is only built in release mode and with the wasm target.
fn main() {
    if env::var("PROFILE").unwrap() != "release" {
        panic!("This crate should be built in release mode, please rerun with `--release`");
    }

    if env::var("TARGET").unwrap() != "wasm32-unknown-unknown" {
        panic!("This crate should be built with the wasm target, please rerun with `--target wasm32-unknown-unknown`");
    }
}
