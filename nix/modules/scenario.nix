# Module to build scenarios and their required hApps into a single derivation

{ self, inputs, lib, ... }@flake: {
  perSystem = { config, self', inputs', system, pkgs, ... }:
    let
      inherit (config.rustHelper) craneLib;

      opensslStatic =
        if system == "x86_64-darwin"
        then pkgs.openssl # pkgsStatic is considered a cross build and this is not yet supported
        else pkgs.pkgsStatic.openssl;

      mkPackage = { name }: craneLib.buildPackage {
        pname = name;
        version = config.rustHelper.findCrateVersion ../../scenarios/${name}/Cargo.toml;

        src = craneLib.cleanCargoSource (craneLib.path ./../..);
        strictDeps = true;

        cargoExtraArgs = "-p ${name}";
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
            inherit name;

            # No sources to copy, everything comes from the build inputs
            unpackPhase = "true";

            buildInputs = [ scenarioBinary scenarioHapps ];

            # To tell `nix run` which binary to run. It gets it right anyway because there is only one binary but
            # it prints an annoying warning message.
            meta = {
              mainProgram = name;
            };

            postInstall = ''
              mkdir -p $out/bin
              cp "${scenarioBinary}/bin/${name}" $out/bin/

              mkdir -p $out/happs
              cp ${scenarioHapps}/*.happ $out/happs
            '';
          };
      };
    };
}
