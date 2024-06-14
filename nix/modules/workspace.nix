{ self, inputs, lib, ... }@flake: {
  perSystem = { config, self', inputs', system, pkgs, ... }:
    let
      inherit (config.rustHelper) craneLib;

      opensslStatic =
        if system == "x86_64-darwin"
        then pkgs.openssl # pkgsStatic is considered a cross build and this is not yet supported
        else pkgs.pkgsStatic.openssl;

      nonCargoBuildFiles = path: _type: builtins.match ".*yaml$" path != null;
      includeFilesFilter = path: type:
        (craneLib.filterCargoSources path type) || (nonCargoBuildFiles path type);

      commonArgs = {
        pname = "workspace";
        version = "0.1.0";

        src = pkgs.lib.cleanSourceWith {
          src = ./../..;
          filter = includeFilesFilter;
        };
        strictDeps = true;

        cargoExtraArgs = "--locked --workspace";
        SKIP_HAPP_BUILD = "1";

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

      cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
        pname = "${commonArgs.pname}-deps";
      });

      workspace = craneLib.buildPackage (commonArgs // {
        inherit cargoArtifacts;
      });

      workspace_clippy = craneLib.cargoClippy (commonArgs // {
        inherit cargoArtifacts;
        cargoClippyExtraArgs = "--all-targets --all-features -- --deny warnings";
      });
    in
    {
      options.workspace = lib.mkOption { type = lib.types.raw; };

      config.workspace = {
        inherit commonArgs cargoArtifacts workspace workspace_clippy;
      };
    };
}
