{ self, inputs, lib, ... }@flake: {
  perSystem = { config, self', inputs', system, pkgs, ... }:
    let
      inherit (config.rustHelper) craneLib;

      opensslStatic =
        if system == "x86_64-darwin"
        then pkgs.openssl # pkgsStatic is considered a cross build and this is not yet supported
        else pkgs.pkgsStatic.openssl;
    in
    {
      packages = {
        workspace = craneLib.buildPackage {
          pname = "workspace";
          version = "0.1.0";

          src = craneLib.cleanCargoSource (craneLib.path ./../..);
          strictDeps = true;

          buildInputs = with pkgs; [
            # Some Holochain crates link against openssl
            openssl
            opensslStatic
          ];

          nativeBuildInputs = with pkgs; [
            # To build openssl-sys
            perl
            pkg-config
            # Because the holochain_client depends on Kitsune/tx5
            go
          ];
        };
      };
    };
}
