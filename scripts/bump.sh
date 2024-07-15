#!/usr/bin/env bash

current_version=$(tomlq -r '.workspace.dependencies.wind_tunnel_core.version' Cargo.toml)

sed -i "s/\", version = \"${current_version}\"/\", version = \"$1\"/g" Cargo.toml
sed -i "s/version = \"${current_version}\"/version = \"$1\"/g" ./framework/**/Cargo.toml ./bindings/**/Cargo.toml
