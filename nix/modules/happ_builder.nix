{ self, inputs, lib, ... }@flake: {
  perSystem = { config, self', inputs', system, pkgs, ... }:
    let
      opensslStatic =
        if system == "x86_64-darwin"
        then pkgs.openssl # pkgsStatic is considered a cross build and this is not yet supported
        else pkgs.pkgsStatic.openssl;

      craneLib = config.rustHelper.mkCraneLib { };

      happ_builder = craneLib.buildPackage {
        pname = "happ_builder";
        version = config.rustHelper.findCrateVersion ../../happ_builder/Cargo.toml;

        src = craneLib.cleanCargoSource (craneLib.path ./../..);
        strictDeps = true;

        cargoExtraArgs = "-p happ_builder --bin hb";

        buildInputs = (with pkgs; [
          # Some Holochain crates link against openssl
          openssl
          opensslStatic
        ]);

        nativeBuildInputs = (with pkgs; [
          # To build openssl-sys
          perl
          pkg-config
          # Depends on Kitsune/tx5
          go
        ]);
      };
    in
    {
      packages = {
        inherit happ_builder;
      };
    };
}
