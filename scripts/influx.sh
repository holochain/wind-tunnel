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

# Import Holochain metrics from $HOLOCHAIN_INFLUX_FILE into InfluxDB with added RUN_ID tag
import_hc_metrics_into_influx() {
    if [[ -z "$HOLOCHAIN_INFLUX_FILE" ]]; then
        echo "HOLOCHAIN_INFLUX_FILE variable is not set. Skipping import of Holochain metrics."
        return
    fi
    if [ ! -f "$HOLOCHAIN_INFLUX_FILE" ]; then
        echo "HOLOCHAIN_INFLUX_FILE is set, but file is missing: $HOLOCHAIN_INFLUX_FILE. Make sure Holochain is running with this env variable."
        return
    fi
    # Determine RUN_ID
    local tmp_run_id=$RUN_ID
    if [ -z "$tmp_run_id" ]; then
      summary_path="run_summary.jsonl"
      if [ -n "$RUN_SUMMARY_PATH" ]; then
          summary_path="$RUN_SUMMARY_PATH"
      fi
      if [ -f "$summary_path" ]; then
          echo "Reading RUN_ID from file: $summary_path"
          tmp_run_id=$(jq --slurp --raw-output 'sort_by(.started_at|tonumber) | last | .run_id' < "$summary_path")
      else
          echo "Run summary file not found: $summary_path" >&2
          tmp_run_id=""
      fi
    fi
    echo "Importing Holochain metrics from '$HOLOCHAIN_INFLUX_FILE' and adding tag run_id=$tmp_run_id"
    # Create new file with added run_id tag at end of tag list of each metric
    OUTPUT_FILE=$HOLOCHAIN_INFLUX_FILE.tmp.influx
    sed "s/\([^[:space:]]*\) \(.*\)/\1,run_id=$tmp_run_id \2/" "$HOLOCHAIN_INFLUX_FILE" > "$OUTPUT_FILE"
    # Import to influxDB
    use_influx
    influx write \
    --bucket "$INFLUX_BUCKET" \
    --org holo \
    --token "$INFLUX_TOKEN" \
    --file "$OUTPUT_FILE"
    echo "Done! Processed $(wc -l < "$HOLOCHAIN_INFLUX_FILE") metrics"
    # Clean up temp file
    rm -f "$OUTPUT_FILE"
}
