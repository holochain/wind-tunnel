#!/usr/bin/env bash

format_rust() {
    set -eux
    cargo fmt --all
}

format_toml() {
    set -eux
    taplo format
}

format_yaml() {
    set -eux
    yamlfmt -gitignore_excludes .
}

format_all() {
    format_rust
    format_toml
    format_yaml
}
