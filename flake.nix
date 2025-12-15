{
  description = "Flake for Holochain testing";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-25.05";

    flake-parts.url = "github:hercules-ci/flake-parts";

    git-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

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

  outputs = inputs@{ flake-parts, git-hooks, crane, rust-overlay, nixpkgs, ... }: flake-parts.lib.mkFlake { inherit inputs; } {
    imports = [
      git-hooks.flakeModule
    ];

    systems = builtins.attrNames inputs.holonix.devShells;

    perSystem = { inputs', pkgs, system, config, ... }:
      let
        unfreePkgs = import nixpkgs { inherit system; config.allowUnfree = true; };
        rustMod = inputs.flake-parts.lib.importApply ./nix/modules/rust.nix { inherit crane rust-overlay nixpkgs; };

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
          ./nix/modules/happs.nix
          rustMod
          ./nix/modules/scenario.nix
          ./nix/modules/scenarios.nix
          ./nix/modules/workspace.nix
          ./nix/modules/zome.nix
          ./nix/modules/zomes.nix
        ];

        _module.args.pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        formatter = pkgs.nixpkgs-fmt;

        pre-commit = {
          check.enable = true;
          settings = {
            hooks = {
              nixpkgs-fmt.enable = true;
              statix.enable = true;
              shellcheck.enable = true;
              rustfmt = {
                enable = true;
                packageOverrides.cargo = config.rustHelper.rust;
                packageOverrides.rustfmt = config.rustHelper.rust;
              };
              taplo.enable = true;
              yamlfmt.enable = true;
            };
          };
        };

        devShells =
          let
            # The packages required in most devShells
            commonPackages = [
              pkgs.cmake
              pkgs.gomplate
              pkgs.netcat-gnu
              pkgs.perl
              pkgs.rustPlatform.bindgenHook
              config.pre-commit.settings.enabledPackages
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
                pkgs.telegraf
                pkgs.yq
                pkgs.httpie
                pkgs.openssl
                pkgs.tomlq
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
                ${config.pre-commit.installationScript}
                source ${./scripts/influx.sh}
                source ${./scripts/telegraf.sh}
                source ${./scripts/checks.sh}
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
          local-upload-metrics = pkgs.writeShellApplication {
            name = "local-upload-metrics";
            runtimeInputs = [
              lp-tool
              pkgs.gnused
              pkgs.influxdb2-cli
              pkgs.jq
              pkgs.yq
            ];
            text = ''
              set -euo pipefail

              # shellcheck disable=SC1091
              source ${./scripts/influx.sh}

              use_influx
              import_lp_metrics

              rm -f ./telegraf/metrics/*.influx 2>/dev/null || true
            '';
          };
          check-scripts = pkgs.writeShellApplication {
            name = "check-scripts";
            runtimeInputs = [
              pkgs.shellcheck
            ];
            text = ''
              set -euo pipefail

              # shellcheck disable=SC1091
              source ${./scripts/checks.sh}

              check_scripts
            '';
          };
          check-nix-fmt = pkgs.writeShellApplication {
            name = "check-nix-fmt";
            runtimeInputs = [
              pkgs.nix
            ];
            text = ''
              set -euo pipefail

              # shellcheck disable=SC1091
              source ${./scripts/checks.sh}

              check_nix_fmt
            '';
          };
          check-nix-lint = pkgs.writeShellApplication {
            name = "check-nix-lint";
            runtimeInputs = [
              pkgs.statix
            ];
            text = ''
              set -euo pipefail

              # shellcheck disable=SC1091
              source ${./scripts/checks.sh}

              check_nix_static
            '';
          };
          check-rust-fmt = pkgs.writeShellApplication {
            name = "check-rust-fmt";
            runtimeInputs = [
              config.rustHelper.rust
            ];
            text = ''
              set -euo pipefail

              # shellcheck disable=SC1091
              source ${./scripts/checks.sh}

              check_rust_fmt
            '';
          };
          check-rust-lint = pkgs.writeShellApplication {
            name = "check-rust-lint";
            runtimeInputs = [
              config.rustHelper.rust
              pkgs.perl
              pkgs.gnumake
              pkgs.cmake
              pkgs.rustPlatform.bindgenHook
            ];
            text = ''
              set -euo pipefail

              export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"

              # shellcheck disable=SC1091
              source ${./scripts/checks.sh}

              check_rust_static
            '';
          };
          check-go = pkgs.writeShellApplication {
            name = "check-go";
            runtimeInputs = [
              pkgs.go
            ];
            text = ''
              set -euo pipefail

              # shellcheck disable=SC1091
              source ${./scripts/checks.sh}

              check_go
            '';
          };
          check-toml-fmt = pkgs.writeShellApplication {
            name = "check-toml-fmt";
            runtimeInputs = [
              pkgs.taplo
            ];
            text = ''
              set -euo pipefail

              # shellcheck disable=SC1091
              source ${./scripts/checks.sh}

              check_toml_fmt
            '';
          };
          check-yaml-fmt = pkgs.writeShellApplication {
            name = "check-yaml-fmt";
            runtimeInputs = [
              pkgs.yamlfmt
            ];
            text = ''
              set -euo pipefail

              # shellcheck disable=SC1091
              source ${./scripts/checks.sh}

              check_yaml_fmt
            '';
          };
          check-all = pkgs.writeShellApplication {
            name = "check-all";
            runtimeInputs = [
              pkgs.go
              pkgs.rustPlatform.bindgenHook
              config.rustHelper.rust
              pkgs.perl
              pkgs.gnumake
              pkgs.cmake
              pkgs.shellcheck
              pkgs.taplo
              pkgs.yamlfmt
              pkgs.statix
            ];
            text = ''
              set -euo pipefail

              export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"

              # shellcheck disable=SC1091
              source ${./scripts/checks.sh}

              check_all
            '';
          };
          format-rust = pkgs.writeShellApplication {
            name = "format-rust";
            runtimeInputs = [
              config.rustHelper.rust
            ];
            text = ''
              set -euo pipefail

              # shellcheck disable=SC1091
              source ${./scripts/format.sh}

              format_rust
            '';
          };
          format-toml = pkgs.writeShellApplication {
            name = "format-toml";
            runtimeInputs = [
              pkgs.taplo
            ];
            text = ''
              set -euo pipefail

              # shellcheck disable=SC1091
              source ${./scripts/format.sh}

              format_toml
            '';
          };
          format-yaml = pkgs.writeShellApplication {
            name = "format-yaml";
            runtimeInputs = [
              pkgs.yamlfmt
            ];
            text = ''
              set -euo pipefail

              # shellcheck disable=SC1091
              source ${./scripts/format.sh}

              format_yaml
            '';
          };
          format-all = pkgs.writeShellApplication {
            name = "format-all";
            runtimeInputs = [
              pkgs.yamlfmt
              pkgs.taplo
              config.rustHelper.rust
            ];
            text = ''
              set -euo pipefail

              # shellcheck disable=SC1091
              source ${./scripts/format.sh}

              format_all
            '';
          };
          generate-nomad-jobs = pkgs.writeShellApplication {
            name = "generate-nomad-jobs";
            runtimeInputs = [
              pkgs.gomplate
            ];
            text = ''
              set -euo pipefail

              # shellcheck disable=SC1091
              ./nomad/scripts/generate_jobs.sh "$@" nomad/job-variants/demo
              ./nomad/scripts/generate_jobs.sh "$@" nomad/job-variants/canonical
            '';
          };
          validate-nomad-jobs = pkgs.writeShellApplication {
            name = "validate-nomad-jobs";
            runtimeInputs = [
              pkgs.gomplate
              unfreePkgs.nomad
            ];
            text = ''
              set -euo pipefail

              # shellcheck disable=SC1091
              ./nomad/scripts/generate_jobs.sh --validate "$@" nomad/job-variants/demo
              ./nomad/scripts/generate_jobs.sh --validate "$@" nomad/job-variants/canonical
            '';
          };
          generate-summary-visualiser = pkgs.writeShellApplication {
            name = "generate-summary-visualiser";
            runtimeInputs = [
              pkgs.gomplate
            ];
            text = ''
              set -euo pipefail

              # shellcheck disable=SC1091
              ./summary-visualiser/generate.sh "$1" > "$2"
            '';
          };
          awscli-s3-cp = pkgs.writeShellApplication {
            name = "awscli-s3-cp";
            runtimeInputs = [
              pkgs.awscli2
            ];
            text = ''
              set -euo pipefail

              # shellcheck disable=SC1091
              aws s3 cp "$1" "$2"
            '';
          };
          rust-unit-tests = pkgs.writeShellApplication {
            name = "rust-unit-tests";
            runtimeInputs = [
              config.rustHelper.rust
              pkgs.perl
              pkgs.gnumake
              pkgs.cmake
              pkgs.rustPlatform.bindgenHook
            ];
            text = ''
              set -euo pipefail

              export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"

              cargo test --workspace --all-targets
            '';
          };
          rust-smoke-test = pkgs.writeShellApplication {
            name = "rust-smoke-test";
            runtimeInputs = [
              config.rustHelper.rust
              customHolochain
              inputs'.holonix.packages.hc
              pkgs.perl
              pkgs.gnumake
              pkgs.cmake
              pkgs.rustPlatform.bindgenHook
            ];
            text = ''
              set -euo pipefail

              export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"

              RUST_LOG=info cargo run "$@"
            '';
          };
          summary-visualiser-smoke-test = pkgs.writeShellApplication {
            name = "summary-visualiser-smoke-test";
            runtimeInputs = [
              pkgs.gomplate
            ];
            text = ''
              set -euo pipefail
              ./summary-visualiser/test.sh
            '';
          };
        };

        checks = {
          inherit (config.workspace) workspace_clippy;
        };
      };
  };

  nixConfig = {
    substituters = [ "https://cache.nixos.org" "https://holochain-ci.cachix.org" ];
    trusted-public-keys = [ "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=" "holochain-ci.cachix.org-1:5IUSkZc0aoRS53rfkvH9Kid40NpyjwCMCzwRTXy+QN8=" ];
  };
}
