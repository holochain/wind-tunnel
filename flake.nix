{
  description = "Flake for Holochain testing";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-24.11";
    flake-parts.url = "github:hercules-ci/flake-parts";

    holonix = {
      url = "github:holochain/holonix?ref=main-0.4";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-parts.follows = "flake-parts";
        crane.follows = "crane";
        rust-overlay.follows = "rust-overlay";
      };
    };

    kitsune2 = {
      url = "github:holochain/kitsune2?ref=v0.0.1-alpha6";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        crane.follows = "crane";
      };
    };

    tryorama = {
      url = "github:holochain/tryorama?ref=v0.17.0";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        crane.follows = "crane";
        rust-overlay.follows = "rust-overlay";
        holonix.follows = "holonix";
      };
    };

    crane = {
      url = "github:ipetkov/crane";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # TODO should be followed correctly by amber for nixpkgs, contribute upstream.
    naersk = {
      url = "github:nix-community/naersk?ref=master";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    amber = {
      url = "github:thetasinner/amber?ref=master";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        naersk.follows = "naersk";
        rust-overlay.follows = "rust-overlay";
      };
    };
  };

  outputs = inputs@{ flake-parts, crane, rust-overlay, nixpkgs, ... }: flake-parts.lib.mkFlake { inherit inputs; } ({ flake-parts-lib, ... }: {
    systems = builtins.attrNames inputs.holonix.devShells;
    perSystem = { inputs', pkgs, system, config, ... }:
      let
        unfreePkgs = import nixpkgs { inherit system; config.allowUnfree = true; };
        rustMod = flake-parts-lib.importApply ./nix/modules/rust.nix { inherit crane rust-overlay nixpkgs; };

        # Enable unstable and non-default features that Wind Tunnel tests.
        cargoExtraArgs = "--features chc,unstable-functions,unstable-countersigning";
        # Override arguments passed in to Holochain build with above feature arguments.
        customHolochain = inputs'.holonix.packages.holochain.override { inherit cargoExtraArgs; };
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
              pkgs.perl
              pkgs.rustPlatform.bindgenHook
              pkgs.shellcheck
              pkgs.statix
              pkgs.taplo
              pkgs.yamlfmt
              config.rustHelper.rust
              customHolochain
              inputs'.holonix.packages.lair-keystore
              inputs'.kitsune2.packages.bootstrap-srv
              inputs'.tryorama.packages.trycp-server
            ];
          in
          {
            default = pkgs.mkShell {
              packages = commonPackages ++ [
                pkgs.influxdb2-cli
                pkgs.influxdb2-server
                # TODO https://docs.influxdata.com/telegraf/v1/install/#ntp
                pkgs.telegraf
                pkgs.yq
                pkgs.httpie
                unfreePkgs.nomad
                inputs'.holonix.packages.hn-introspect
                inputs'.amber.packages.default
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
                source ./scripts/trycp.sh
                source ./scripts/checks.sh
              '';
            };

            ci = pkgs.mkShell {
              packages = commonPackages;
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
          ci-telegraf = pkgs.writeShellApplication {
            name = "ci-telegraf";
            runtimeInputs = [ pkgs.telegraf ];
            text = "telegraf --config telegraf/runner-telegraf.conf --once > >(tee logs/telegraf-stdout.log) 2> >(tee logs/telegraf-stderr.log >&2)";
          };
        };

        checks = {
          inherit (config.workspace) workspace_clippy;
        };
      };
  });

  nixConfig = {
    substituters = [ "https://cache.nixos.org" "https://holochain-ci.cachix.org" "https://holochain-wind-tunnel.cachix.org" ];
    trusted-public-keys = [ "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=" "holochain-ci.cachix.org-1:5IUSkZc0aoRS53rfkvH9Kid40NpyjwCMCzwRTXy+QN8=" "holochain-wind-tunnel.cachix.org-1:tnSm+7Y3hDKOc9xLdoVMuInMA2AQ0R/99Ucz5edYGJw=" ];
  };
}
