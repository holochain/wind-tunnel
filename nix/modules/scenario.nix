# Module to build scenarios and their required hApps into a single derivation

{ config, system, pkgs, lib, ... }:
let
  inherit (config.rustHelper) craneLib;

  mkPackage = { name }: craneLib.buildPackage (config.workspace.commonArgs // {
    pname = name;
    version = config.rustHelper.findCrateVersion ../../scenarios/${name}/Cargo.toml;

    inherit (config.workspace) cargoArtifacts;

    cargoExtraArgs = "-p ${name}";
    SKIP_HAPP_BUILD = "1";

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
        scenarioHapps = config.happHelper.mkHapps { configToml = ../../scenarios/${name}/Cargo.toml; };
      in
      pkgs.stdenv.mkDerivation {
        pname = name;
        inherit (scenarioBinary) version;

        # No sources to copy, everything comes from the build inputs
        unpackPhase = "true";

        buildInputs = [ scenarioBinary scenarioHapps pkgs.zip ];

        # To tell `nix run` which binary to run. It gets it right anyway because there is only one binary but
        # it prints an annoying warning message.
        meta = {
          mainProgram = name;
        };

        postInstall = ''
          mkdir -p $out/bin
          cp "${scenarioBinary}/bin/${name}" $out/bin/

          mkdir -p $out/happs
          cp ${scenarioHapps}/.happ-build ${scenarioHapps}/*.happ $out/happs

          cd $out && zip -r ${name}.zip bin happs
        '';
      };
  };
}
