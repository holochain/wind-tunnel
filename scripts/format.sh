#!/usr/bin/env bash

format_rust() {
    set -eux
    cargo fmt --all
}

format_toml() {
    set -eux
    taplo format ./*.toml
    taplo format ./bindings/**/*.toml
    taplo format ./framework/**/*.toml
    taplo format ./scenarios/**/*.toml
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
