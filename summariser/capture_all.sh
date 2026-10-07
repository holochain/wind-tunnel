#!/usr/bin/env bash
#
# Capture metrics from all scenarios using the capture.sh script.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
SUMMARY_FILE="$REPO_ROOT/run_summary.jsonl"

TEST_DATA_DIR="$REPO_ROOT/summariser/test_data"
if [ -n "$(git -C "$REPO_ROOT" status --porcelain "$TEST_DATA_DIR")" ]; then
    echo "Warning: summariser/test_data has uncommitted changes or untracked files:"
    git -C "$REPO_ROOT" status --short "$TEST_DATA_DIR"
    read -rp "[c]lean up / [y] continue anyway / [N] abort? " answer
    case "$answer" in
        [Cc])
            echo "Cleaning up summariser/test_data..."
            git -C "$REPO_ROOT" checkout -- "$TEST_DATA_DIR"
            git -C "$REPO_ROOT" clean -fd "$TEST_DATA_DIR"
            echo "Clean."
            ;;
        [Yy])
            ;;
        *)
            echo "Aborting." >&2
            exit 1
            ;;
    esac
fi

if [ -f "$SUMMARY_FILE" ]; then
    read -rp "run_summary.jsonl exists. Remove it before starting? [y/N] " answer
    if [[ "$answer" =~ ^[Yy]$ ]]; then
        rm "$SUMMARY_FILE"
        echo "Removed run_summary.jsonl"
    else
        echo "Aborting." >&2
        exit 1
    fi
fi

read -rp "Remove existing test data before starting? [y/N] " answer
if [[ "$answer" =~ ^[Yy]$ ]]; then
    echo "Removing existing test data..."
    rm -rf "$TEST_DATA_DIR"/1_run_summaries/*.json "$TEST_DATA_DIR"/2_query_results/*.json "$TEST_DATA_DIR"/3_summary_outputs/*.json
    echo "Removed."
fi

"$REPO_ROOT"/summariser/capture.sh app_install \
  --duration 30 --behaviour minimal

"$REPO_ROOT"/summariser/capture.sh app_install \
  --duration 30 --behaviour large

WT_CELL_COUNT=10 WT_RESTART_INTERVAL=3 "$REPO_ROOT"/summariser/capture.sh conductor_startup \
  --duration 30

"$REPO_ROOT"/summariser/capture.sh dht_sync_lag \
  --duration 60 --agents 2 --behaviour write:1 --behaviour record_lag:1

"$REPO_ROOT"/summariser/capture.sh first_call \
  --duration 30

"$REPO_ROOT"/summariser/capture.sh full_arc_create_validated_zero_arc_read \
  --duration 60 --agents 3 --behaviour zero:1 --behaviour full:2

"$REPO_ROOT"/summariser/capture.sh local_signals \
  --duration 30

"$REPO_ROOT"/summariser/capture.sh mixed_arc_get_agent_activity \
  --duration 60 --agents 6 --behaviour zero_read:3 --behaviour zero_write:1 --behaviour full_write:2

"$REPO_ROOT"/summariser/capture.sh mixed_arc_must_get_agent_activity \
  --duration 60 --agents 6 --behaviour zero_must_get_agent_activity:3 --behaviour zero_write:1 --behaviour full_write:2

# peerkit_hole_punch needs a Peerkit relay and the `peerkit` CLI from the `.#peerkit` dev shell.
PEERKIT_RELAY_LOG="$(mktemp)"
PEERKIT_RELAY_PID=""
stop_peerkit_relay() {
    if [ -n "$PEERKIT_RELAY_PID" ]; then
        kill "$PEERKIT_RELAY_PID" 2>/dev/null || true
        wait "$PEERKIT_RELAY_PID" 2>/dev/null || true
    fi
    rm -f "$PEERKIT_RELAY_LOG"
}
trap stop_peerkit_relay EXIT

# Resolve the binary once and run it directly: backgrounding `nix develop -c` would make the
# PID the wrapper's, and killing it might leave the relay running.
PEERKIT_BIN="$(nix develop "$REPO_ROOT#peerkit" -c bash -c 'command -v peerkit')"
"$PEERKIT_BIN" relay 127.0.0.1:9910 > "$PEERKIT_RELAY_LOG" 2>&1 &
PEERKIT_RELAY_PID=$!
for _ in $(seq 1 120); do
    grep -q 'Relay address: ' "$PEERKIT_RELAY_LOG" && break
    sleep 1
done
PEERKIT_RELAY_ADDR="$(sed -n 's/^Relay address: //p' "$PEERKIT_RELAY_LOG" | head -n 1)"
if [ -z "$PEERKIT_RELAY_ADDR" ]; then
    echo "Peerkit relay did not print its dial address:" >&2
    cat "$PEERKIT_RELAY_LOG" >&2
    exit 1
fi

WT_PEERKIT_PATH="$PEERKIT_BIN" \
  PEERKIT_DIRECT_UPGRADE_TIMEOUT_MS=2000 "$REPO_ROOT"/summariser/capture.sh peerkit_hole_punch \
  --relay-dial-addr "$PEERKIT_RELAY_ADDR" --duration 60 --agents 3 --behaviour node:3

stop_peerkit_relay
trap - EXIT

MIN_AGENTS=2 "$REPO_ROOT"/summariser/capture.sh remote_call_rate \
  --duration 30 --agents 2

MIN_AGENTS=2 "$REPO_ROOT"/summariser/capture.sh remote_signals \
  --duration 30 --agents 2

"$REPO_ROOT"/summariser/capture.sh single_write_many_read \
  --duration 60

"$REPO_ROOT"/summariser/capture.sh two_party_countersigning \
  --duration 120 --agents 5 --behaviour initiate:2 --behaviour participate:3

MIN_AGENTS=5 "$REPO_ROOT"/summariser/capture.sh unyt_proposal \
  --agents 5 --behaviour initiate:1 --behaviour propose:2 --behaviour respond:2 --duration 300

NO_VALIDATION_COMPLETE=1 MIN_AGENTS=10 "$REPO_ROOT"/summariser/capture.sh validation_receipts \
  --duration 60 --agents 10

MIN_AGENTS=2 "$REPO_ROOT"/summariser/capture.sh write_get_agent_activity \
  --duration 60 --agents 2 --behaviour write:1 --behaviour get_agent_activity:1

MIN_AGENTS=2 "$REPO_ROOT"/summariser/capture.sh write_get_agent_activity_volatile \
  --duration 180 --agents 2 --behaviour write:1 --behaviour get_agent_activity_volatile:1

"$REPO_ROOT"/summariser/capture.sh write_query \
  --duration 60

"$REPO_ROOT"/summariser/capture.sh write_read \
  --duration 60

"$REPO_ROOT"/summariser/capture.sh write_validated \
  --duration 60

MIN_AGENTS=2 "$REPO_ROOT"/summariser/capture.sh write_validated_must_get_agent_activity \
  --duration 60 --agents 2 --behaviour write:1 --behaviour must_get_agent_activity:1

"$REPO_ROOT"/summariser/capture.sh zero_arc_create_and_read \
  --duration 500 --agents 4 --behaviour zero_read:1 --behaviour zero_write:1 --behaviour full:2

"$REPO_ROOT"/summariser/capture.sh zero_arc_create_data \
  --duration 500 --agents 3 --behaviour zero:1 --behaviour full:2

"$REPO_ROOT"/summariser/capture.sh zero_arc_create_data_validated \
  --duration 500 --agents 3 --behaviour zero:1 --behaviour full:2

"$REPO_ROOT"/summariser/capture.sh zome_call_single_value \
  --duration 30

UNYT_DURABLE_OBJECTS_URL=http://localhost:8787 "$REPO_ROOT"/summariser/capture.sh unyt_chain_transaction \
  --duration 300 --agents 5 --behaviour initiate:1 --behaviour spend:4

UNYT_DURABLE_OBJECTS_URL=http://localhost:8787 "$REPO_ROOT"/summariser/capture.sh unyt_chain_transaction_zero_arc \
  --duration 300 --agents 7 --behaviour initiate:1 --behaviour zero_spend:2 --behaviour zero_smart_agreements:2 --behaviour full_observer:1 --behaviour zero_observer:1

UNYT_DURABLE_OBJECTS_URL=http://localhost:8787 "$REPO_ROOT"/summariser/capture.sh unyt_swap \
  --duration 300 --agents 5 --behaviour initiate:1 --behaviour bridge_agent:1 --behaviour swap_agent:1 --behaviour user:2

"$REPO_ROOT"/summariser/truncate.sh
