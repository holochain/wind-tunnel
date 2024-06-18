{
  description = "Flake for Holochain testing";

  inputs = {
    versions.url = "github:holochain/holochain?dir=versions/0_3";

    holochain = {
      url = "github:holochain/holochain";
      inputs.versions.follows = "versions";
    };

    tryorama.url = "github:holochain/tryorama/v0.16.0";

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

  outputs = inputs:
    inputs.holochain.inputs.flake-parts.lib.mkFlake { inherit inputs; }
      {
        systems = builtins.attrNames inputs.holochain.devShells;
        imports = [
          ./nix/modules/formatter.nix
          ./nix/modules/happs.nix
          ./nix/modules/rust.nix
          ./nix/modules/scenario.nix
          ./nix/modules/scenarios.nix
          ./nix/modules/workspace.nix
          ./nix/modules/zome.nix
          ./nix/modules/zomes.nix
        ];
        perSystem = { lib, config, pkgs, system, self', ... }:
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
                pkgs.statix
                inputs.tryorama.packages.${system}.trycp-server
              ];

              shellHook = ''
                source ./scripts/influx.sh
                source ./scripts/telegraf.sh
                source ./scripts/trycp.sh
                source ./scripts/checks.sh
              '';
            };

            devShells.ci = pkgs.mkShell {
              inputsFrom = [
                inputs.holochain.devShells.${system}.holochainBinaries
              ];

              packages = [
                pkgs.shellcheck
                pkgs.statix
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
      };
}
