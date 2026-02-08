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
REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source scripts/influx.sh
source scripts/telegraf.sh

use_influx

TELEGRAF_PID=""
stop_telegraf() {
    if [ -n "$TELEGRAF_PID" ]; then
        echo "Stopping telegraf (PID $TELEGRAF_PID)..."
        kill "$TELEGRAF_PID" 2>/dev/null || true
        wait "$TELEGRAF_PID" 2>/dev/null || true
    fi
}
trap stop_telegraf EXIT

echo "Starting telegraf..."
telegraf --config "${REPO_ROOT}/telegraf/telegraf.host.conf" &
TELEGRAF_PID=$!

echo "Running scenario: $SCENARIO"
RUST_LOG=info cargo run --package "$SCENARIO" -- --reporter=influx-file "$@"

stop_telegraf

echo "Uploading metrics..."
nix run .#local-upload-metrics

echo "Capturing test data..."
RUST_LOG=info cargo run --features test_data --package holochain_summariser

echo "Done."
