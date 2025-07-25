#!/usr/bin/env bash

set -euo pipefail
set -x

BASEDIR="$(dirname "$0")"
VARS_DIR="$BASEDIR/vars"
JOBS_DIR="$BASEDIR/jobs"
TEMPLATE="$BASEDIR/run_scenario.tpl.hcl"

mkdir -p "$JOBS_DIR"
rm -rf "$JOBS_DIR"/*.nomad.hcl || true

GOMPLATE=$(command -v gomplate || echo "gomplate")
if [ -z "$GOMPLATE" ]; then
  echo "gomplate is not installed. Please install it to generate Nomad jobs."
  echo "You can install gomplate from the release page: <https://github.com/hairyhenderson/gomplate/releases>"
  exit 1
fi

# iter files in vars directory
for VAR_FILE in "$VARS_DIR"/*.json; do
    BASENAME=$(basename "$VAR_FILE" .json)
    JOB_FILE="$JOBS_DIR/$BASENAME.nomad.hcl"
    gomplate \
        -f "$TEMPLATE" \
        -o "$JOB_FILE" \
        -d vars="$VAR_FILE"

    echo "Generated job file: $JOB_FILE"
done
