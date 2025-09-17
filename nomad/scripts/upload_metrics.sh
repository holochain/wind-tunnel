#!/usr/bin/env bash

# this script is executed on Nomad clients, after a scenario has been run, to upload metrics with telegraf.
# It also takes care of setting the RUN_ID if unset from env

set -euo pipefail

function check_envset() {
  local var_name="$1"
  if [[ -z "${!var_name:-}" ]]; then
    echo "Environment variable $var_name is not set or is an empty string." >&2
    exit 1
  fi
}

check_envset "WT_METRICS_DIR"
check_envset "INFLUX_TOKEN"
check_envset "INFLUX_HOST"
check_envset "NOMAD_ALLOC_DIR"

influx_bucket="${INFLUX_BUCKET:-windtunnel}"

# if RUN_ID is NOT set, try to get it from run_summary.jsonl
echo "Current RUN_ID: '${RUN_ID:-unset}'"
if [ "${RUN_ID:+x}" != "x" ]; then
    # if is set RUN_SUMMARY_PATH
    summary_path=${RUN_SUMMARY_PATH:-"run_summary.jsonl"}

    if [ -f "$summary_path" ]; then
        RUN_ID=$(jq --slurp --raw-output 'sort_by(.started_at|tonumber) | last | .run_id' < "$summary_path")
    else
        echo "Run summary file not found: $summary_path" >&2
        exit 1
    fi
    export RUN_ID
    echo "RUN_ID: '$RUN_ID'"
else
    echo "RUN_ID is already set to '$RUN_ID'"
fi

# for each metric file, import to influx
for metric_file in "$WT_METRICS_DIR"/*.influx; do
    echo "Importing $metric_file"
    out_file="$NOMAD_ALLOC_DIR/$(basename "$metric_file")"
    # Tag metrics with RUN_ID, if set
    if [[ "${RUN_ID:+x}" == "x" ]]; then
        lp-tool -input "$metric_file" -output "$out_file" -tag run_id="$RUN_ID"
    else
        cp "$metric_file" "$out_file"
    fi
    # import metrics to influx
    influx write \
        --host "$INFLUX_HOST" \
        --bucket "$influx_bucket" \
        --org "holo" \
        --file "$out_file"
    echo "Finished importing $metric_file"
done
