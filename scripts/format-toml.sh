#!/usr/bin/env bash
# Usage to format: ./scripts/format-toml.sh
# Usage to check: ./scripts/format-toml.sh --check

set -eux

EXTRA_ARG=${1:-}

taplo format "$EXTRA_ARG" ./*.toml
taplo format "$EXTRA_ARG" ./bindings/**/*.toml
taplo format "$EXTRA_ARG" ./framework/**/*.toml
taplo format "$EXTRA_ARG" ./scenarios/**/*.toml
