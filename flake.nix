{
  description = "Flake for Holochain testing";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-25.11";

    nixpkgsUnstable.url = "github:nixos/nixpkgs?ref=nixos-unstable";

    flake-parts.url = "github:hercules-ci/flake-parts";

    git-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    holonix = {
      url = "github:holochain/holonix?ref=main-0.6";
    };

    crane = {
      url = "github:ipetkov/crane";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ flake-parts, git-hooks, crane, rust-overlay, nixpkgs, nixpkgsUnstable, ... }: flake-parts.lib.mkFlake { inherit inputs; } {
    imports = [
      git-hooks.flakeModule
    ];

    systems = builtins.attrNames inputs.holonix.devShells;

    perSystem = { inputs', pkgs, lib, system, config, ... }:
      let
        unfreeUnstablePkgs = import nixpkgsUnstable { inherit system; config.allowUnfree = true; };
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

        # Threefold cli tool
        tfgridSdkGoVersion = "0.17.5";
        tfgridSdkGoRev = "v${tfgridSdkGoVersion}";
        tfgridSdkGoSrc = pkgs.fetchFromGitHub {
          owner = "threefoldtech";
          repo = "tfgrid-sdk-go";
          rev = tfgridSdkGoRev;
          hash = "sha256-a0So46H3afPtb3mhD+ktzj4z4PX2W+0yaM5NH4i+y4E=";
        };

        tfgridSdkGoBuild =
          { pname
          , modRoot
          , subPackages ? [ "." ]
          , ldflags ? [ ]
          , postInstall ? ""
          , vendorHash
          }:
          pkgs.buildGoModule {
            inherit pname modRoot subPackages ldflags vendorHash;
            version = tfgridSdkGoVersion;
            src = tfgridSdkGoSrc;
            env = {
              CGO_ENABLED = "0";
              GOWORK = "off";
            };
            doCheck = false;
            inherit postInstall;
          };

        tfgrid-sdk-go-tfrobot = tfgridSdkGoBuild {
          pname = "tfgrid-sdk-go-tfrobot";
          modRoot = "tfrobot";
          vendorHash = "sha256-Lyms04JtyCeRmWDlebMc+l3D3hIng05YxjTMTkdu91s=";
          ldflags = [
            "-X github.com/threefoldtech/tfgrid-sdk-go/tfrobot/cmd.version=${tfgridSdkGoRev}"
            "-X github.com/threefoldtech/tfgrid-sdk-go/tfrobot/cmd.commit=${tfgridSdkGoRev}"
          ];
        };

        tfgrid-sdk-go-tf-grid-cmd = tfgridSdkGoBuild {
          pname = "tfgrid-sdk-go-tf-grid-cmd";
          modRoot = "grid-cli";
          vendorHash = "sha256-wR/iIc4mQbXGlCZ38RQaYy13sASHDnkVmMJwtGh4NnY=";
          ldflags = [
            "-X github.com/threefoldtech/tfgrid-sdk-go/grid-cli/cmd.version=${tfgridSdkGoRev}"
            "-X github.com/threefoldtech/tfgrid-sdk-go/grid-cli/cmd.commit=${tfgridSdkGoRev}"
          ];
          postInstall = ''
            if [ -e "$out/bin/grid-cli" ]; then
              mv "$out/bin/grid-cli" "$out/bin/tf-grid-cmd"
              ln -s tf-grid-cmd "$out/bin/tfcmd"
            fi
          '';
        };
      in
      {
        imports = [
          rustMod
          ./nix/modules/scenario.nix
          ./nix/modules/scenarios.nix
          ./nix/modules/workspace.nix
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
              shellcheck = {
                enable = true;
                args = [ "-x" ];
              };
              rustfmt = {
                enable = true;
                packageOverrides.cargo = config.rustHelper.rust;
                packageOverrides.rustfmt = config.rustHelper.rust;
              };
              taplo.enable = true;
              yamlfmt.enable = true;
              check-summary-visualiser-html = {
                enable = true;
                name = "check-summary-visualiser-html";
                entry = "${config.packages.summary-visualiser-smoke-test}/bin/summary-visualiser-smoke-test";
                files = "^(summary-visualiser/.*|flake\.nix)$";
                pass_filenames = false;
              };
            };
          };
        };

        devShells =
          let
            # The packages required in most devShells
            commonPackages = [
              pkgs.cmake
              pkgs.gnugrep
              pkgs.gomplate
              pkgs.html-tidy
              pkgs.netcat-gnu
              pkgs.perl
              pkgs.rustPlatform.bindgenHook
              config.pre-commit.settings.enabledPackages
              config.rustHelper.rust
              customHolochain
              inputs'.holonix.packages.lair-keystore
              inputs'.holonix.packages.hc
              inputs'.holonix.packages.bootstrap-srv
              pkgs.cargo-nextest
            ];
          in
          {
            default = pkgs.mkShell.override { stdenv = pkgs.clangStdenv; } {
              buildInputs = [
                pkgs.go
                lp-tool
              ];

              packages = commonPackages ++ [
                pkgs.influxdb2-cli
                pkgs.influxdb2-server
                # TODO https://docs.influxdata.com/telegraf/v1/install/#ntp
                pkgs.telegraf
                pkgs.httpie
                pkgs.openssl
                pkgs.tomlq
                pkgs.getopt
                pkgs.jq
                unfreeUnstablePkgs.nomad_1_11
                inputs'.holonix.packages.hn-introspect
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
                tfgrid-sdk-go-tfrobot
                tfgrid-sdk-go-tf-grid-cmd
                pkgs.yq-go
              ];
            };

            kitsune = pkgs.mkShell {
              packages = [
                pkgs.cmake
                pkgs.perl
                pkgs.rustPlatform.bindgenHook
                inputs'.holonix.packages.bootstrap-srv
              ];
            };
          };

        packages = {
          default = config.workspace.workspace;
          inherit (config.workspace) workspace;
          inherit lp-tool;
          inherit tfgrid-sdk-go-tfrobot tfgrid-sdk-go-tf-grid-cmd;
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
              pkgs.getopt
            ];
            text = ''
              set -euo pipefail

              # shellcheck disable=SC1091
              ./nomad/scripts/generate_jobs.sh -- "$@"
            '';
          };
          validate-all-nomad-jobs = pkgs.writeShellApplication {
            name = "validate-all-nomad-jobs";
            runtimeInputs = [
              pkgs.gomplate
              unfreeUnstablePkgs.nomad_1_11
              pkgs.getopt
            ];
            text = ''
              set -euo pipefail

              # shellcheck disable=SC1091
              ./nomad/scripts/generate_jobs.sh -- --validate --job-variant-path nomad/job-variants/demo
              ./nomad/scripts/generate_jobs.sh -- --validate --job-variant-path nomad/job-variants/canonical
              ./nomad/scripts/generate_jobs.sh -- --validate --job-variant-path nomad/job-variants/canonical-scaled
            '';
          };
          check-influx-setup-script = pkgs.writeShellApplication {
            name = "check-influx-setup-script";
            runtimeInputs = [
              pkgs.coreutils
              pkgs.influxdb2-cli
              pkgs.influxdb2-server
              pkgs.tomlq
            ];
            text = ''
              set -euo pipefail

              if [ "''${WIND_TUNNEL_SKIP_CLEAN_PROMPT:-}" != "1" ]; then
                echo "This will remove ./influx/influx.toml and ./influx/.influxdbv2/"
                read -r -p "Proceed? [y/N] " answer
                if [ "$answer" != "y" ] && [ "$answer" != "Y" ]; then
                  echo "Aborted."
                  exit 1
                fi
              fi
              rm -rf ./influx/influx.toml ./influx/.influxdbv2/

              # shellcheck disable=SC1091
              source ./scripts/influx.sh
              influxd &
              influxd_pid=$!
              trap 'kill "$influxd_pid" 2>/dev/null || true' EXIT

              # Poll until influxd is ready, timeout after 30s
              ready=false
              for i in $(seq 1 30); do
                  echo "Checking if InfluxDB is ready... (attempt $i/30)"
                  if influx ping --host http://localhost:8087 2>/dev/null; then
                    ready=true
                    break
                  fi
                  sleep 1
              done
              if [ "$ready" != "true" ]; then
                echo "ERROR: InfluxDB did not become ready within 30 seconds" >&2
                exit 1
              fi

              configure_influx
              use_influx

              for var in INFLUX_TOKEN INFLUX_HOST INFLUX_BUCKET; do
                if [ -z "''${!var:-}" ]; then
                  echo "ERROR: $var is not set after use_influx"
                  kill $influxd_pid
                  exit 1
                fi
              done

              # Verify the bucket is reachable via the influx CLI
              if ! influx bucket list --host "$INFLUX_HOST" --token "$INFLUX_TOKEN" --org holo --name "$INFLUX_BUCKET" >/dev/null; then
                echo "ERROR: failed to list bucket '$INFLUX_BUCKET' via influx CLI"
                kill $influxd_pid
                exit 1
              fi
              echo "Influx CLI check passed: bucket '$INFLUX_BUCKET' is accessible"

              kill $influxd_pid
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
          generate-summary-visualiser-with-test-data = pkgs.writeShellApplication {
            name = "generate-summary-visualiser-with-test-data";
            runtimeInputs = [
              pkgs.gomplate
              pkgs.jq
            ];
            text = ''
              set -euo pipefail

              # shellcheck disable=SC1091
              temp_dir=$(mktemp -d)
              jq -s '.' summariser/test_data/3_summary_outputs/*.json > "$temp_dir/summary-visualiser-test-data.json"
              ./summary-visualiser/generate.sh "$temp_dir/summary-visualiser-test-data.json" > "$1"
            '';
          };
          awscli-s3 = pkgs.writeShellApplication {
            name = "awscli-s3";
            runtimeInputs = [
              pkgs.awscli2
            ];
            text = ''
              set -euo pipefail

              # shellcheck disable=SC1091
              aws s3 "$@"
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
          summary-visualiser-smoke-test =
            let
              summaryVisualiser = pkgs.runCommand "summary-visualiser" { } ''
                mkdir -p $out/summary-visualiser
                cp -r ${lib.cleanSource ./summary-visualiser}/* $out/summary-visualiser/

                mkdir -p $out/summariser/test_data/3_summary_outputs
                cp -r ${lib.cleanSource ./summariser/test_data/3_summary_outputs}/* $out/summariser/test_data/3_summary_outputs/
                
                chmod +x $out/summary-visualiser/test.sh
                chmod +x $out/summary-visualiser/generate.sh
                chmod +x $out/summary-visualiser/scenario_template_exists.sh
                patchShebangs $out
              '';
            in
            pkgs.writeShellApplication {
              name = "summary-visualiser-smoke-test";
              runtimeInputs = [
                pkgs.coreutils
                pkgs.gnugrep
                pkgs.gomplate
                pkgs.html-tidy
                pkgs.jq
              ];
              text = ''
                set -euo pipefail
                ${summaryVisualiser}/summary-visualiser/test.sh "$@"
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
