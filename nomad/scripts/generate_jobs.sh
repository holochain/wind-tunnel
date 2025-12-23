#!/usr/bin/env bash

set -euo pipefail

generate_job() {
    local var_file="$1"
    local job_file="$2"
    gomplate \
        -f "nomad/run_scenario.tpl.hcl" \
        -o "$job_file" \
        -d vars="$var_file"

    echo "Generated job file: $job_file"
}

validate_job() {
    local job_file="$1"
    nomad job validate "$job_file"
    echo "Validated job file: $job_file"
}


if ! command -v gomplate &> /dev/null; then
    echo "gomplate is not installed. Please install it to generate Nomad jobs."
    echo "You can install gomplate from the release page: <https://github.com/hairyhenderson/gomplate/releases>"
    exit 1
fi

###  Parse CLI Args ###
OPTIONS=$(getopt --longoptions validate,job-name:,job-variant-path: --name "$0" -- "$@")
eval set -- "$OPTIONS"

VALIDATE=false
JOB_NAME=""
JOB_VARIANT_PATH=""
while true; do
    case "$1" in
        --validate)
            VALIDATE=true
            shift
            ;;
        --job-name)
            JOB_NAME="$2"
            shift 2
            ;;
        --job-variant-path)
            JOB_VARIANT_PATH="$2"
            shift 2
            ;;
        --)
            shift
            break
            ;;
        *) echo "Invalid Arg: $1"; exit 1 ;;
    esac
done

if [ "$JOB_VARIANT_PATH" = "" ]; then
    echo "Argument --job-variant-path is required"
    exit 1
fi

if [ ! -d "$JOB_VARIANT_PATH" ]; then
    echo "Error: --job-variant-path directory does not exist: $JOB_VARIANT_PATH"
    exit 1
fi

if [ "$VALIDATE" = true ] && ! command -v nomad &> /dev/null; then
    echo "nomad is not installed. Please install it to validate Nomad jobs."
    echo "You can install nomad from the official website: <https://www.nomadproject.io/downloads>"
    exit 1
fi

VARS_DIR="$JOB_VARIANT_PATH/vars"
JOBS_DIR="$JOB_VARIANT_PATH/jobs"

if [ ! -d "$VARS_DIR" ]; then
    echo "Error: vars directory does not exist: $VARS_DIR"
    exit 1
fi

### Run Script ###
# Clean jobs output directory
mkdir -p "$JOBS_DIR"
rm -rf "$JOBS_DIR"/*.nomad.hcl || true

# Generate job(s)
if [ -z "$JOB_NAME" ]; then
    # Generate all jobs for job variant
    for VAR_FILE in "$VARS_DIR"/*.json; do
        BASENAME=$(basename "$VAR_FILE" .json)
        JOB_FILE="$JOBS_DIR/$BASENAME.nomad.hcl"
        generate_job "$VAR_FILE" "$JOB_FILE"
        if [ "$VALIDATE" = true ]; then
            validate_job "$JOB_FILE"
        fi
    done
else
    # Generate specific job for job variant
    VAR_FILE="$VARS_DIR/$JOB_NAME.json"
    if [ ! -f "$VAR_FILE" ]; then
        echo "Variable file for job '$JOB_NAME' does not exist: $VAR_FILE"
        exit 1
    fi
    JOB_FILE="$JOBS_DIR/$JOB_NAME.nomad.hcl"
    generate_job "$VAR_FILE" "$JOB_FILE"
    if [ "$VALIDATE" = true ]; then
        validate_job "$JOB_FILE"
    fi
fi
