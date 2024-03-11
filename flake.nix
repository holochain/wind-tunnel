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
        perSystem = { config, pkgs, system, ... }:
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
                openssl opensslStatic
              ]);

              nativeBuildInputs = (with pkgs; [
                perl pkg-config go inputs.holochain.packages.${system}.holochain
              ]);
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
                zome_call_single_value = zome_call_single_value;
            };

            apps = {
                zome_call_single_value = {
                    program = zome_call_single_value;
                };
            };
        };
    };
}
