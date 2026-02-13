#!/usr/bin/env bash
#
# Capture metrics from a single scenario run for local summariser development.
#
# Usage:
#   ./summariser/capture.sh <scenario_package> [extra cargo args...]
#
# Examples:
#   ./summariser/capture.sh write_read
#   ./summariser/capture.sh app_install -- --behaviour minimal
#   ./summariser/capture.sh two_party_countersigning -- --agents 5 --behaviour initiate:2 --behaviour participate:3

set -euo pipefail

if [ -z "${1:-}" ]; then
    echo "Usage: $0 <scenario_package> [extra cargo args...]" >&2
    exit 1
fi

SCENARIO="$1"
shift

# Must be run from within a nix shell
if [ -z "${IN_NIX_SHELL:-}" ]; then
    echo "Error: This script must be run from within a nix shell." >&2
    exit 1
fi

# Run from repo root (telegraf config uses relative paths)
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/influx.sh
source scripts/influx.sh
# shellcheck source=scripts/telegraf.sh
source scripts/telegraf.sh

use_influx

export HOLOCHAIN_INFLUXIVE_FILE="$WT_METRICS_DIR/holochain.influx"

TELEGRAF_PID=""
cleanup() {
    if [ -n "$TELEGRAF_PID" ]; then
        echo "Stopping telegraf (PID $TELEGRAF_PID)..."
        kill "$TELEGRAF_PID" 2>/dev/null || true
        wait "$TELEGRAF_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT

echo "Starting telegraf..."
start_host_metrics_telegraf &
TELEGRAF_PID=$!

echo "Running scenario: $SCENARIO"
RUST_LOG=info cargo run --package "$SCENARIO" -- --reporter=influx-file "$@"

echo "Uploading metrics..."
nix run .#local-upload-metrics

echo "Capturing test data..."
RUST_LOG=info cargo run --features test_data --package holochain_summariser

echo "Done."
