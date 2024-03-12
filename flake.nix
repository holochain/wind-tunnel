{
  description = "Flake for Holochain testing";

  inputs = {
    versions.url = "github:holochain/holochain?dir=versions/0_2";

    versions.inputs.holochain.url = "github:holochain/holochain/holochain-0.2.6";

    holochain = {
      url = "github:holochain/holochain";
      inputs.versions.follows = "versions";
    };

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "holochain/nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "holochain/nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };

    nixpkgs.follows = "holochain/nixpkgs";
  };

  outputs = inputs @ { ... }:

    inputs.holochain.inputs.flake-parts.lib.mkFlake { inherit inputs; }
      {
        systems = builtins.attrNames inputs.holochain.devShells;
        imports = [ ./nix/modules/formatter.nix ./nix/modules/happ_builder.nix ./nix/modules/rust.nix ./nix/modules/scenario.nix ];
        perSystem = { lib, config, pkgs, system, self', ... }:
          let
            opensslStatic =
              if system == "x86_64-darwin"
              then pkgs.openssl # pkgsStatic is considered a cross build
              # and this is not yet supported
              else pkgs.pkgsStatic.openssl;

            rustPkgs = import inputs.nixpkgs {
              inherit system;
              overlays = [ (import inputs.rust-overlay) ];
            };

            rustWithWasmTarget = rustPkgs.rust-bin.stable.latest.default.override {
              targets = [ "wasm32-unknown-unknown" ];
            };

            craneLib = (inputs.crane.mkLib rustPkgs).overrideToolchain rustWithWasmTarget;

            crateInfo = craneLib.crateNameFromCargoToml { cargoToml = ./scenarios/zome_call_single_value/Cargo.toml; };

            zome_call_single_value = craneLib.buildPackage {
              pname = "zome_call_single_value";
              version = crateInfo.version;

              src = craneLib.cleanCargoSource (craneLib.path ./.);
              strictDeps = true;

              cargoExtraArgs = "-p zome_call_single_value";

              buildInputs = (with pkgs; [
                openssl
                opensslStatic # Some Holochain crates link against openssl
              ]);

              nativeBuildInputs = (with pkgs; [
                perl
                pkg-config # To build openssl-sys
                go # Because the holochain_client depends on Kitsune/tx5
                inputs.holochain.packages.${system}.holochain # The build needs `hc` provided
              ]);
            };

            zome_call_single_value_happs = pkgs.stdenv.mkDerivation {
              name = "zome_call_single_value_happs";
              src = lib.fileset.toSource {
                root = ./.;
                fileset = ./happs/zome_call_single_value;
              };
              postInstall = ''
                mkdir -p $out
                cp -R ./happs/zome_call_single_value/ $out/
              '';
            };

            zome_call_single_value_package = derivation {
              name = "zome_call_single_value";
              builder = "/bin/bash";
              system = system;
              buildInputs = [ zome_call_single_value zome_call_single_value_happs ];

              args = [ "-c" "/bin/mkdir -p $out/bin && /bin/cp \"${lib.getBin zome_call_single_value}/bin/zome_call_single_value\" $out/bin/ && /bin/mkdir -p $out/happs && /bin/cp ${zome_call_single_value_happs}/zome_call_single_value/*.happ $out/happs" ];
            };

            single_write_many_read = config.scenarioHelper.mkScenario {
              name = "single_write_many_read";
            };
          in
          {
            devShells.default = pkgs.mkShell {
              inputsFrom = [
                inputs.holochain.devShells.${system}.holonix
              ];
              packages = [
                pkgs.influxdb2-cli
                pkgs.influxdb2-server
                # TODO https://docs.influxdata.com/telegraf/v1/install/#ntp
                pkgs.telegraf
                pkgs.yq
                pkgs.httpie
                pkgs.shellcheck
              ];

              shellHook = ''
                source ./scripts/influx.sh
                source ./scripts/telegraf.sh
              '';
            };

            packages = {
              zome_call_single_value_p = zome_call_single_value_package;
              zome_call_single_value_happs = zome_call_single_value_happs;
              inherit single_write_many_read;
            };

            apps = {
              zome_call_single_value = {
                program = zome_call_single_value_package;
              };
            };
          };
      };
}
