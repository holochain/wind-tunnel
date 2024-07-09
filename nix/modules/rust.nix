# Module to configure Rust and Crane for use in other modules.

{ self, inputs, lib, ... }@flake: {
  perSystem = { config, self', inputs', system, pkgs, ... }:
    let
      rustPkgs = import inputs.nixpkgs {
        inherit system;
        overlays = [ (import inputs.rust-overlay) ];
      };

      rustVersion = "1.78.0";

      rustWithWasmTarget = rustPkgs.rust-bin.stable."${rustVersion}".minimal.override {
        targets = [ "wasm32-unknown-unknown" ];
        extensions = [ "clippy" ];
      };

      craneLib = (inputs.crane.mkLib rustPkgs).overrideToolchain rustWithWasmTarget;
    in
    {
      options.rustHelper = lib.mkOption { type = lib.types.raw; };

      config.rustHelper = {
        findCrateVersion = cargoToml: (craneLib.crateNameFromCargoToml { inherit cargoToml; }).version;

        inherit craneLib;

        rust = rustWithWasmTarget;
      };
    };
}
