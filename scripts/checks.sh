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

check_go() {
  set -euo pipefail
  cd lp-tool
  go mod tidy
  go build
  go test -v
  ./lp-tool -h
}

check_all() {
  check_scripts
  check_nix_fmt
  check_nix_static
  check_rust_fmt
  check_rust_static
  check_go
}
