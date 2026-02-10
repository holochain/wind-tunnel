# Module to build scenarios, including their required hApps, into a single derivation

{ config, inputs', system, pkgs, lib, ... }:
let
  inherit (config.rustHelper) craneLib findCrateVersion;
  inherit (config.workspace) commonArgs cargoArtifacts;
  fetchHappsForScenario = scenarioName:
    let
      # Get the TOML object from the Cargo.toml file for the passed scenario
      cargoToml = lib.importTOML ../../scenarios/${scenarioName}/Cargo.toml;
      # If there are hApps to fetch then return a list of them, otherwise return an empty list
      hAppsToFetch = lib.lists.toList (lib.attrsets.attrByPath [ "package" "metadata" "fetch-required-happ" ] [ ] cargoToml);
      # Convert the passed hApp into a source to be fetched from a URL
      hAppSource = hApp: builtins.fetchurl { inherit (hApp) name url sha256; };
      # Create a derivation that stores all of the fetched hApps
      allFetchedHApps = pkgs.stdenv.mkDerivation {
        name = "${scenarioName}-fetched-hApps";
        dontUnpack = true;
        installPhase = ''
          mkdir -p $out
          ${lib.strings.concatMapStringsSep "\n" (hApp: "cp ${hAppSource hApp} $out/${hApp.name}.happ") hAppsToFetch}
        '';
      };
    in
    # If there are hApps to fetch then return a derivation with them all in, else return `null`
    if hAppsToFetch != [ ] then allFetchedHApps else null;

  mkPackage = { name }: craneLib.buildPackage (commonArgs // {
    pname = name;
    version = findCrateVersion ../../scenarios/${name}/Cargo.toml;

    inherit cargoArtifacts;

    cargoExtraArgs = "--locked -p ${name}";

    # Copy dependencies from commonArgs as `//` doesn't deep copy
    nativeBuildInputs = commonArgs.nativeBuildInputs ++ [
      # Required to build/package DNAs and hApps
      inputs'.holonix.packages.hc
    ];

    preBuild =
      let
        fetchedHapps = fetchHappsForScenario name;
      in
      if fetchedHapps != null then ''
        mkdir -p happs/${name}
        cp ${fetchedHapps}/*.happ happs/${name}
      '' else "";

    postInstall = ''
      # Copy the hApps built via the Rust build script and fetched in preBuild
      if [ -d "happs/${name}" ]; then
          mkdir -p $out/happs
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
          # Copy the scenario binary
          mkdir -p $out/bin
          cp "${scenarioBinary}/bin/${name}" $out/bin/

          # Copy the required hApps for the scenario
          mkdir -p $out/happs
          if [ -d "${scenarioBinary}/happs" ]; then
              cp ${scenarioBinary}/happs/*.happ $out/happs
          fi

          # Zip scenario binary and all required hApps
          cd $out && zip -r ${name}.zip bin happs
        '';
      };
  };
}
