# Module to build scenarios and their required hApps into a single derivation

{ self, inputs, lib, ... }@flake: {
  perSystem = { config, self', inputs', system, pkgs, ... }:
    let
      opensslStatic =
        if system == "x86_64-darwin"
        then pkgs.openssl # pkgsStatic is considered a cross build and this is not yet supported
        else pkgs.pkgsStatic.openssl;

      craneLib = config.rustHelper.mkCraneLib { };

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

      mkHapps = { name }: pkgs.stdenv.mkDerivation {
        name = "${name}-happs";
        src = ./../..; # Really just needs root, zomes and scenarios
        buildInputs = [
          self'.packages.happ_builder
          # Building DNAs requires `cargo`
          config.rustHelper.rust
          # The `happ_builder` needs `hc` provided
          inputs.holochain.packages.${system}.holochain
        ];
        postInstall = ''
          export CARGO_HOME=$(pwd)/.cargo
          hb ${name} $(pwd)/scenarios/${name} $(pwd)/zomes $(pwd)/wasm-target $(pwd)/built

          mkdir -p $out
          cp ./built/happs/${name}/*.happ $out/
        '';
      };
    in
    {
      options.scenarioHelper = lib.mkOption { type = lib.types.raw; };

      config.scenarioHelper = {
        mkScenario = { name }:
          let
            scenarioBinary = mkPackage { inherit name; };
            scenarioHapps = mkHapps { inherit name; };
          in
          pkgs.stdenv.mkDerivation {
            inherit name;
            src = ./../..;

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
