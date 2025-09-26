#!/usr/bin/env bash

check_scripts() {
  shellcheck scripts/*.sh
}

check_nix_fmt() {
  nix fmt ./nix flake.nix -- --check
}

check_nix_static() {
  statix check ./nix && statix check ./flake.nix
}

check_rust_fmt() {
  cargo fmt --all -- --check
}

check_rust_static() {
  cargo clippy --workspace --all-targets --all-features -- --deny warnings
}

check_toml_fmt() {
  taplo format --check ./*.toml
  taplo format --check ./bindings/**/*.toml
  taplo format --check ./framework/**/*.toml
  taplo format --check ./scenarios/**/*.toml
}

check_yaml_fmt() {
  yamlfmt -gitignore_excludes -lint .
}

check_go() {
  set -euo pipefail
  cd lp-tool
  go mod tidy
  go build
  go test -v
  ./lp-tool -h
  cd -
}

check_all() {
  check_scripts
  check_nix_fmt
  check_nix_static
  check_rust_fmt
  check_rust_static
  check_go
  check_toml_fmt
  check_yaml_fmt
}
