#!/usr/bin/env bash

# Configure InfluxDB to store data within the repository
INFLUXD_BOLT_PATH="$(pwd)/influx/.influxdbv2/influxd.bolt"
export INFLUXD_BOLT_PATH
INFLUXD_ENGINE_PATH="$(pwd)/influx/.influxdbv2/engine/"
export INFLUXD_ENGINE_PATH
INFLUXD_CONFIG_PATH="$(pwd)/influx/"
export INFLUXD_CONFIG_PATH

# Configure the InfluxDB CLI to store its config within the repository
INFLUX_CONFIGS_PATH="$(pwd)/influx/influx.toml"
export INFLUX_CONFIGS_PATH

# Configures the current shell to use InfluxDB with Wind Tunnel
use_influx() {
    export INFLUX_HOST="http://localhost:8087"
    export INFLUX_BUCKET="windtunnel"
    INFLUX_TOKEN="$(<"$INFLUX_CONFIGS_PATH" tomlq -r .default.token)"
    export INFLUX_TOKEN
}

# Dev only setup for InfluxDB, this function can be called from inside the dev shell once `influxd` is running
configure_influx() {
    influx setup --host http://localhost:8087 --username windtunnel --password windtunnel --org holo --bucket windtunnel --force
    use_influx

    # Import variables
    find influx/templates/variables/*.json -maxdepth 1 -exec influx apply --host "$INFLUX_HOST" --token "$INFLUX_TOKEN" --org holo --file "{}" -quiet --force yes \;

    # Import dashboards
    find influx/templates/dashboards/*.json -maxdepth 1 -exec influx apply --host "$INFLUX_HOST" --token "$INFLUX_TOKEN" --org holo --file "{}" --quiet --force yes \;
}

# Remove data and config
clear_influx() {
    http "http://localhost:8087/debug/flush"
    rm "$INFLUX_CONFIGS_PATH"
}

import_lp_metrics() {
    if [ -z "${INFLUX_TOKEN:-}" ]; then
        echo "INFLUX_TOKEN is not set"
        return 1
    fi

    local influx_url
    influx_url="${1:-$INFLUX_HOST}"
    local influx_bucket
    influx_bucket="${INFLUX_BUCKET:-windtunnel}"

    set -euo pipefail
    local wt_metrics_dir
    wt_metrics_dir="$(pwd)/telegraf/metrics"

    # get run summary path
    local summary_path
    summary_path=${RUN_SUMMARY_PATH:-"run_summary.jsonl"}

    # Get run id from the latest run summary or set it to ""
    local run_id
    if [ -f "$summary_path" ]; then
        run_id=$(jq --slurp --raw-output 'sort_by(.started_at|tonumber) | last | .run_id' < "$summary_path")
    else
        run_id=""
    fi

    if [ -z "${run_id}" ]; then
        echo "No run ID found, using empty run ID"
    fi

    # for each metric file, import to influx
    local tmp_output_file
    for metric_file in "$wt_metrics_dir"/*.influx; do
        echo "Importing $metric_file"
        tmp_output_file="$(mktemp -u)"
        # Tag metrics with run_id, if set
        if [[ "${run_id:+x}" == "x" ]]; then
            lp-tool -input "$metric_file" -output "$tmp_output_file" -tag run_id="$run_id"
        fi
        # import metrics to influx
        influx write \
            --host "$influx_url" \
            --bucket "$influx_bucket" \
            --org "holo" \
            --file "$tmp_output_file"
        # remove temp file
        rm -f "$tmp_output_file"
        echo "Finished importing $metric_file"
    done

}

# Import Holochain metrics from $HOLOCHAIN_INFLUX_FILE into InfluxDB with added RUN_ID tag
import_hc_metrics_into_influx() {
    set -e

    if [[ -z "${INFLUX_BUCKET:-}" ]] || [[ -z "${INFLUX_TOKEN:-}" ]] || [[ -z "${INFLUX_HOST:-}" ]]; then
      echo "Environment variables INFLUX_BUCKET, INFLUX_TOKEN and INFLUX_HOST have not been set. Run 'use_influx' to have them set." >&2
      return 1
    fi
    if [[ -z "$HOLOCHAIN_INFLUX_FILE" ]]; then
        echo "HOLOCHAIN_INFLUX_FILE variable is not set. Skipping import of Holochain metrics."
        return 1
    fi
    if [ ! -f "$HOLOCHAIN_INFLUX_FILE" ]; then
        echo "HOLOCHAIN_INFLUX_FILE is set, but file is missing: $HOLOCHAIN_INFLUX_FILE. Make sure Holochain is running with this env variable."
        return 1
    fi
    # Determine RUN_ID
    local tmp_run_id="${RUN_ID:-}"
    if [ -z "$tmp_run_id" ]; then
      local summary_path="run_summary.jsonl"
      if [ -n "$RUN_SUMMARY_PATH" ]; then
          summary_path="$RUN_SUMMARY_PATH"
      fi
      if [ -f "$summary_path" ]; then
          echo "Reading RUN_ID from file: $summary_path"
          tmp_run_id=$(jq --slurp --raw-output 'sort_by(.started_at|tonumber) | last | .run_id' < "$summary_path")
      else
          echo "Aborting. RUN_ID not set and run summary file not found: $summary_path" >&2
          return 1
      fi
    fi
    echo "Importing Holochain metrics from '$HOLOCHAIN_INFLUX_FILE' and adding tag run_id=$tmp_run_id"
    # Create new influx file with added run_id tag for each metric
    local tmp_output_file
    tmp_output_file=$(mktemp -u)
    lp-tool -input "$HOLOCHAIN_INFLUX_FILE" -output "$tmp_output_file" -tag run_id="$tmp_run_id"
    # Import to influxDB
    influx write \
    --bucket "$INFLUX_BUCKET" \
    --org holo \
    --file "$tmp_output_file"
    # Clean up temp file
    rm -f "$tmp_output_file"
    echo "Successfully imported Holochain metrics to '$INFLUX_HOST' and deleted file $tmp_output_file"
}
