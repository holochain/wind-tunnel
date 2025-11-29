# Module for building zome WASMs

{ config, lib, ... }:
let
  inherit (config.rustHelper) craneLib;
in
{
  options.zomeHelper = lib.mkOption { type = lib.types.raw; };

  config.zomeHelper = {
    mkZome = { name, kind }:
      let
        packageName = if kind == "integrity" then "${name}_${kind}" else name;
      in
      craneLib.buildPackage (config.workspace.commonArgs // {
        pname = "${name}_${kind}";
        version = config.rustHelper.findCrateVersion ../../zomes/${name}/${kind}/Cargo.toml;

        inherit (config.workspace) cargoArtifacts;

        doCheck = false;

        cargoExtraArgs = "-p ${packageName} --lib --target wasm32-unknown-unknown";

        env.RUSTFLAGS = "--cfg getrandom_backend=\"custom\"";
      });
  };
}
