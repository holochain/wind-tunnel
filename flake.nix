{
  description = "Flake for Holochain testing";

  inputs = {
    holonix.url = "github:holochain/holonix/main";

    nixpkgs.follows = "holonix/nixpkgs";
    flake-parts.follows = "holonix/flake-parts";

    tryorama = {
      url = "github:holochain/tryorama/v0.17.0-dev.1";
      inputs = {
        nixpkgs.follows = "holonix/nixpkgs";

      };
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "holonix/nixpkgs";
      };
    };

    amber = {
      url = "github:Ph0enixKM/Amber";
      inputs.nixpkgs.follows = "holonix/nixpkgs";
    };
  };

  outputs = inputs@{ flake-parts, holonix, ... }: flake-parts.lib.mkFlake { inherit inputs; } ({ flake-parts-lib, ... }: {
    systems = builtins.attrNames inputs.holonix.devShells;
    perSystem = { inputs', pkgs, system, config, ... }:
      let
        rustMod = flake-parts-lib.importApply ./nix/modules/rust.nix { inherit (holonix.inputs) crane; inherit (inputs) nixpkgs rust-overlay; };
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
          inputsFrom = [ inputs'.holonix.devShells ];

          packages = (with inputs'.holonix.packages; [
            holochain
            rust # For Rust development, with the WASM target included for zome builds
          ]) ++ (with pkgs; [
            pkgs.influxdb2-cli
            pkgs.influxdb2-server
            # TODO https://docs.influxdata.com/telegraf/v1/install/#ntp
            pkgs.telegraf
            pkgs.yq
            pkgs.httpie
            pkgs.shellcheck
            pkgs.statix
            inputs.tryorama.packages.${system}.trycp-server
            # inputs.amber.packages.${system}.default
          ]);

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
            inputs.tryorama.packages.${system}.trycp-server
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
