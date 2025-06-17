# Module to configure Rust and Crane for use in other modules.

{ crane, nixpkgs, rust-overlay }:
{ config, self', inputs', system, pkgs, lib, ... }:
let
  rustPkgs = import nixpkgs {
    inherit system;
    overlays = [ (import rust-overlay) ];
  };

  rustVersion = "1.87.0";

  rustWithWasmTarget = rustPkgs.rust-bin.stable."${rustVersion}".minimal.override {
    targets = [ "wasm32-unknown-unknown" ];
    extensions = [ "clippy" "rustfmt" ];
  };

  craneLib = (crane.mkLib rustPkgs).overrideToolchain rustWithWasmTarget;
in
{
  options.rustHelper = lib.mkOption { type = lib.types.raw; };

  config.rustHelper = {
    findCrateVersion = cargoToml: (craneLib.crateNameFromCargoToml { inherit cargoToml; }).version;

    inherit craneLib;

    rust = rustWithWasmTarget;
  };
}
