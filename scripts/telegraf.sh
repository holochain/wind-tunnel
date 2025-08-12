#!/usr/bin/env bash

WT_METRICS_DIR="$(pwd)/telegraf/metrics"
export WT_METRICS_DIR

start_telegraf() {
    if [[ -z "$INFLUX_TOKEN" ]]; then
        echo "INFLUX_TOKEN is not set, please run \`use_influx\` first"
        return 1
    fi

    telegraf --config "$(pwd)/telegraf/telegraf.local.conf"
}

start_host_metrics_telegraf() {
    telegraf --config "$(pwd)/telegraf/telegraf.host.conf"
}
