# Module to build scenarios, including their required hApps, into a single derivation

{ config, inputs', system, pkgs, lib, ... }:
let
  inherit (config.rustHelper) craneLib;

  mkPackage = { name }: craneLib.buildPackage (config.workspace.commonArgs // {
    pname = name;
    version = config.rustHelper.findCrateVersion ../../scenarios/${name}/Cargo.toml;

    inherit (config.workspace) cargoArtifacts;

    cargoExtraArgs = "--locked -p ${name}";

    nativeBuildInputs = with pkgs; [
      # To build openssl-sys
      perl

      # Required to build/package DNAs and hApps
      inputs'.holonix.packages.hc
    ];

    postInstall = ''
      # Copy the hApps built via the Rust build script
      mkdir -p $out/happs
      if [ -d "happs/${name}" ]; then
          cp happs/${name}/*.happ $out/happs
      fi
    '';

    # When built from an x86_64-linux system, modify the executable to specify the standard linux
    # system path for `ld` as its interpreter.
    #
    # Otherwise it will specify the nix store path from the system it was built on,
    # and thus will not run on any other system.
    #
    # Even though our target wind-tunnel-runner systems should have have the library dependencies installed
    # from nixpkgs, their flake lock files may be out of sync, and thus the nix store path for `ld` may be different.
    #
    # As long as the target system makes ld available on the standard linux system path, the executable should run.
    # On NixOS systems, this requires enabling nix-ld.
    postFixup = lib.optionalString (system == "x86_64-linux") ''
      patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 $out/bin/${name}
    '';
  });
in
{
  options.scenarioHelper = lib.mkOption { type = lib.types.raw; };

  config.scenarioHelper = {
    mkScenario = { name }:
      let
        scenarioBinary = mkPackage { inherit name; };
      in
      pkgs.stdenv.mkDerivation {
        pname = name;
        inherit (scenarioBinary) version;

        # No sources to copy, everything comes from the build inputs
        unpackPhase = "true";

        buildInputs = [ scenarioBinary pkgs.zip ];

        # To tell `nix run` which binary to run. It gets it right anyway because there is only one binary but
        # it prints an annoying warning message.
        meta = {
          mainProgram = name;
        };

        postInstall = ''
          mkdir -p $out/bin
          cp "${scenarioBinary}/bin/${name}" $out/bin/

          # Copy the hApps from the scenario
          if [ -d "${scenarioBinary}/happs" ]; then
              mkdir -p $out/happs
              cp ${scenarioBinary}/happs/*.happ $out/happs
          fi

          cd $out && zip -r ${name}.zip bin happs
        '';
      };
  };
}
