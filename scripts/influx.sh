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
