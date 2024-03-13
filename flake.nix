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

  outputs = inputs:

    inputs.holochain.inputs.flake-parts.lib.mkFlake { inherit inputs; }
      {
        systems = builtins.attrNames inputs.holochain.devShells;
        imports = [ ./nix/modules/formatter.nix ./nix/modules/happ_builder.nix ./nix/modules/rust.nix ./nix/modules/scenario.nix ./nix/modules/scenarios.nix ];
        perSystem = { lib, config, pkgs, system, self', ... }:
          let
            scenario_package_names = builtins.filter (lib.strings.hasPrefix "scenario_") (builtins.attrNames self'.packages);
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
                pkgs.statix
              ];

              shellHook = ''
                source ./scripts/influx.sh
                source ./scripts/telegraf.sh
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

            apps = builtins.listToAttrs (builtins.map (name: { inherit name; value = { program = self'.packages.${name}; }; }) scenario_package_names);
          };
      };
}
