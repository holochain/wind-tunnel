{ self, inputs, lib, ... }@flake: {
  perSystem = { config, self', inputs', system, pkgs, ... }:
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
          craneLib.buildPackage {
            pname = "${name}_${kind}";
            version = config.rustHelper.findCrateVersion ../../zomes/${name}/${kind}/Cargo.toml;

            src = craneLib.cleanCargoSource (craneLib.path ./../..);
            strictDeps = true;
            doCheck = false;

            cargoExtraArgs = "-p ${packageName} --lib --target wasm32-unknown-unknown";
          };
      };
    };
}
