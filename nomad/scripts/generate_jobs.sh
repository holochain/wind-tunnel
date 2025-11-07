#!/usr/bin/env bash

set -euo pipefail
set -x

generate_job() {
  local var_file="$1"
  local job_file="$2"
  gomplate \
    -f "$TEMPLATE" \
    -o "$job_file" \
    -d vars="$var_file"

  echo "Generated job file: $job_file"
}

validate_job() {
  local job_file="$1"
  nomad job validate "$job_file"
  echo "Validated job file: $job_file"
}

SCRIPTS_DIR="$(dirname "$0")"
BASEDIR="$(realpath "$SCRIPTS_DIR/..")"
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
NOMAD=$(command -v nomad || echo "nomad")
if [ -z "$NOMAD" ]; then
  echo "nomad is not installed. Please install it to validate Nomad jobs."
  echo "You can install nomad from the official website: <https://www.nomadproject.io/downloads>"
  exit 1
fi

ARG1="${1:-}"
validate_jobs=false
if [ "$ARG1" == "--validate" ]; then
    validate_jobs=true
    shift
    JOB_NAME="${1:-}"
else
    JOB_NAME="$ARG1"
fi

# generate all
if [ -z "$JOB_NAME" ]; then
    # iter files in vars directory
    for VAR_FILE in "$VARS_DIR"/*.json; do
        BASENAME=$(basename "$VAR_FILE" .json)
        JOB_FILE="$JOBS_DIR/$BASENAME.nomad.hcl"
        generate_job "$VAR_FILE" "$JOB_FILE"
        if [ "$validate_jobs" = true ]; then
            validate_job "$JOB_FILE"
        fi
    done
else
    # generate specific job
    VAR_FILE="$VARS_DIR/$JOB_NAME.json"
    if [ ! -f "$VAR_FILE" ]; then
        echo "Variable file for job '$JOB_NAME' does not exist: $VAR_FILE"
        exit 1
    fi
    JOB_FILE="$JOBS_DIR/$JOB_NAME.nomad.hcl"
    generate_job "$VAR_FILE" "$JOB_FILE"
    if [ "$validate_jobs" = true ]; then
        validate_job "$JOB_FILE"
    fi
fi
