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
  go mod tidy
  go build -C lp-tool -o lp-tool
  go test -v ./...
  ./lp-tool/lp-tool -h
}

check_all() {
  check_scripts
  check_nix_fmt
  check_nix_static
  check_rust_fmt
  check_rust_static
  check_go
}
