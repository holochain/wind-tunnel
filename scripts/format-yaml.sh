#!/usr/bin/env bash
# Usage to format: ./scripts/format-yaml.sh
# Usage to check: ./scripts/format-yaml.sh -lint

set -eux

EXTRA_ARG=${1:-}

yamlfmt -gitignore_excludes "$EXTRA_ARG" .
