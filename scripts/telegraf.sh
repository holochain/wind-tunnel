#!/usr/bin/env bash

WT_METRICS_DIR="$(pwd)/telegraf/metrics"
export WT_METRICS_DIR

start_host_metrics_telegraf() {
    telegraf --config "$(pwd)/telegraf/telegraf.host.conf"
}
