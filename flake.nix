{
  description = "Flake for Holochain testing";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=24.05";
    flake-parts.url = "github:hercules-ci/flake-parts";

    holonix = {
      url = "github:holochain/holonix?ref=main";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-parts.follows = "flake-parts";
        crane.follows = "crane";
        rust-overlay.follows = "rust-overlay";
        # TODO: Revert when new release of holochain is made with updated hc-sandbox cli
        holochain.url = "github:holochain/holochain?ref=develop";
      };
    };

    tryorama = {
      url = "github:holochain/tryorama?ref=v0.17.0-dev.5";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        crane.follows = "crane";
        rust-overlay.follows = "rust-overlay";
        holonix.follows = "holonix";
      };
    };

    chc-service = {
      url = "github:holochain/hc-chc-service";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        crane.follows = "crane";
        rust-overlay.follows = "rust-overlay";
        holonix.follows = "holonix";
      };
    };

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
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
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.naersk.follows = "naersk";
    };
  };

  outputs = inputs@{ flake-parts, crane, rust-overlay, nixpkgs, ... }: flake-parts.lib.mkFlake { inherit inputs; } ({ flake-parts-lib, ... }: {
    systems = builtins.attrNames inputs.holonix.devShells;
    perSystem = { inputs', pkgs, system, config, ... }:
      let
        rustMod = flake-parts-lib.importApply ./nix/modules/rust.nix { inherit crane rust-overlay nixpkgs; };
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

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            pkgs.influxdb2-cli
            pkgs.influxdb2-server
            # TODO https://docs.influxdata.com/telegraf/v1/install/#ntp
            pkgs.telegraf
            pkgs.yq
            pkgs.httpie
            pkgs.shellcheck
            pkgs.statix
            inputs'.holonix.packages.holochain
            inputs'.holonix.packages.lair-keystore
            inputs'.holonix.packages.hn-introspect
            inputs'.holonix.packages.rust
            inputs'.tryorama.packages.trycp-server
            inputs'.chc-service.packages.hc-chc-service
            inputs'.amber.packages.default
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
            pkgs.darwin.apple_sdk.frameworks.CoreFoundation
          ];

          shellHook = ''
            source ./scripts/influx.sh
            source ./scripts/telegraf.sh
            source ./scripts/trycp.sh
            source ./scripts/checks.sh
          '';
        };

        devShells.ci = pkgs.mkShell {
          inputsFrom = [ inputs'.holonix.devShells ];

          packages = [
            pkgs.shellcheck
            pkgs.statix
            inputs'.holonix.packages.holochain
            inputs'.holonix.packages.lair-keystore
            inputs'.tryorama.packages.trycp-server
          ];
        };

        packages = {
          default = config.workspace.workspace;
          inherit (config.workspace) workspace;
        };

        checks = {
          inherit (config.workspace) workspace_clippy;
        };
      };
  });
}
