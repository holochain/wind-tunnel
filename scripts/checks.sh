#!/usr/bin/env bash

set -euxo pipefail

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
  cargo clippy --workspace --all-targets --all-features -- -D warnings
}

check_all() {
  check_scripts
  check_nix_fmt
  check_nix_static
  check_rust_fmt
  check_rust_static
}
