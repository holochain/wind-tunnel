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
check_envset "TELEGRAF_CONFIG_PATH"

# read RUN_ID if not set in the environment
if [ -z "$RUN_ID" ]; then
    # if is set RUN_SUMMARY_PATH
    summary_path="run_summary.jsonl"
    if [ ! -z "$RUN_SUMMARY_PATH" ]; then
        summary_path="$RUN_SUMMARY_PATH"
    fi

    if [ -f "$summary_path" ]; then
        RUN_ID=$(jq --slurp --raw-output 'sort_by(.started_at|tonumber) | last | .run_id' < "$summary_path")
    else
        echo "Run summary file not found: $summary_path" >&2
        RUN_ID=""
    fi
    export RUN_ID
fi

echo "RUN_ID: '$RUN_ID'"

telegraf --once
