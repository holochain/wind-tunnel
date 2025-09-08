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

# Adds a run_id tag all metrics in $WT_METRICS_DIR and imports them into InfluxDB
import_lp_metrics() {
    if [ -z "${INFLUX_TOKEN:-}" ]; then
        echo "Environment variable INFLUX_TOKEN has not been set. Run 'use_influx' to set it." >&2
        return 1
    fi

    local influx_url
    influx_url="${1:-$INFLUX_HOST}"
    local influx_bucket
    influx_bucket="${INFLUX_BUCKET:-windtunnel}"

    local wt_metrics_dir
    wt_metrics_dir="${WT_METRICS_DIR:-"$(pwd)/telegraf/metrics"}"

    local summary_path
    summary_path=${RUN_SUMMARY_PATH:-"run_summary.jsonl"}

    set -euo pipefail

    # Get run-id from the latest run summary or set it to ""
    local run_id
    if [ -f "$summary_path" ]; then
        run_id=$(jq --slurp --raw-output 'sort_by(.started_at|tonumber) | last | .run_id' < "$summary_path")
    else
        run_id=""
    fi

    if [ -z "${run_id}" ]; then
        echo "No run_id found, using empty run_id"
    else
        echo "Metrics will be imported with tag: run_id=$run_id"
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
