{
  description = "Flake for Holochain testing";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-25.05";
    unstable.url = "github:NixOS/nixpkgs?ref=nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";

    holonix = {
      url = "github:holochain/holonix?ref=main-0.5";
    };

    kitsune2 = {
      url = "github:holochain/kitsune2?ref=v0.1.8";
    };

    crane = {
      url = "github:ipetkov/crane";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ flake-parts, crane, rust-overlay, nixpkgs, unstable, ... }: flake-parts.lib.mkFlake { inherit inputs; } ({ flake-parts-lib, ... }: {
    systems = builtins.attrNames inputs.holonix.devShells;
    perSystem = { inputs', pkgs, system, config, ... }:
      let
        unfreePkgs = import nixpkgs { inherit system; config.allowUnfree = true; };
        unstablePkgs = import unstable { inherit system; };
        rustMod = flake-parts-lib.importApply ./nix/modules/rust.nix { inherit crane rust-overlay nixpkgs; };

        # Enable unstable and non-default features that Wind Tunnel tests.
        cargoExtraArgs = "--features chc,unstable-functions,unstable-countersigning";
        # Override arguments passed in to Holochain build with above feature arguments.
        customHolochain = inputs'.holonix.packages.holochain.override { inherit cargoExtraArgs; };

        lp-tool = pkgs.buildGoModule {
          pname = "lp-tool";
          version = "0.1.0";
          src = ./lp-tool;
          vendorHash = "sha256-7IGJGP2K0H0eKYU+gveykhGYt9ZufJNBUEv3jM66Wt0=";
        };
      in
      {
        imports = [
          ./nix/modules/formatter.nix
          ./nix/modules/happs.nix
          rustMod
          ./nix/modules/scenario.nix
          ./nix/modules/scenarios.nix
          ./nix/modules/workspace.nix
          ./nix/modules/zome.nix
          ./nix/modules/zomes.nix
        ];

        devShells =
          let
            # The packages required in most devShells
            commonPackages = [
              pkgs.cmake
              pkgs.gomplate
              pkgs.perl
              pkgs.rustPlatform.bindgenHook
              pkgs.shellcheck
              pkgs.statix
              pkgs.taplo
              pkgs.yamlfmt
              pkgs.netcat-gnu
              config.rustHelper.rust
              customHolochain
              inputs'.holonix.packages.lair-keystore
              inputs'.holonix.packages.hc
              inputs'.kitsune2.packages.bootstrap-srv
            ];
          in
          {
            default = pkgs.mkShell {
              buildInputs = [
                pkgs.go
                lp-tool
              ];

              packages = commonPackages ++ [
                pkgs.influxdb2-cli
                pkgs.influxdb2-server
                # TODO https://docs.influxdata.com/telegraf/v1/install/#ntp
                unstablePkgs.telegraf
                pkgs.yq
                pkgs.httpie
                unfreePkgs.nomad
                inputs'.holonix.packages.hn-introspect
              ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
                pkgs.darwin.apple_sdk.frameworks.Security
                pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
                pkgs.darwin.apple_sdk.frameworks.CoreFoundation
              ];

              NOMAD_ADDR = "https://nomad-server-01.holochain.org:4646";
              NOMAD_CACERT = ./nomad/server-ca-cert.pem;

              shellHook = ''
                source ./scripts/influx.sh
                source ./scripts/telegraf.sh
                source ./scripts/checks.sh
              '';
            };

            ci = pkgs.mkShell {
              packages = commonPackages ++ [
                pkgs.go
              ];
            };

            kitsune = pkgs.mkShell {
              packages = [
                pkgs.cmake
                pkgs.perl
                pkgs.rustPlatform.bindgenHook
                inputs'.kitsune2.packages.bootstrap-srv
              ];
            };
          };

        packages = {
          default = config.workspace.workspace;
          inherit (config.workspace) workspace;
          inherit lp-tool;
          local-telegraf = pkgs.writeShellApplication {
            name = "local-telegraf";
            runtimeInputs = [
              lp-tool
              pkgs.gnused
              pkgs.influxdb2-cli
              pkgs.jq
              pkgs.yq
              unstablePkgs.telegraf
            ];
            text = ''
              set -euo pipefail

              # shellcheck disable=SC1091
              source ./scripts/influx.sh
              
              use_influx
              import_lp_metrics

              rm -f ./telegraf/metrics/*.influx 2>/dev/null || true
            '';
          };
          ci-telegraf = pkgs.writeShellApplication {
            name = "ci-telegraf";
            runtimeInputs = [
              lp-tool
              pkgs.gnused
              pkgs.influxdb2-cli
              pkgs.jq
              pkgs.yq
              unstablePkgs.telegraf
            ];
            text = ''
              set -euo pipefail

              # shellcheck disable=SC1091
              source ./scripts/influx.sh
              
              import_lp_metrics "https://ifdb.holochain.org"

              rm -f ./telegraf/metrics/*.influx 2>/dev/null || true
            '';
          };
        };

        checks = {
          inherit (config.workspace) workspace_clippy;
        };
      };
  });

  nixConfig = {
    substituters = [ "https://cache.nixos.org" "https://holochain-ci.cachix.org" ];
    trusted-public-keys = [ "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=" "holochain-ci.cachix.org-1:5IUSkZc0aoRS53rfkvH9Kid40NpyjwCMCzwRTXy+QN8=" ];
  };
}
