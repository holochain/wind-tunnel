#!/usr/bin/env bash
#
# Capture metrics from all scenarios using the capture.sh script.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
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

./capture.sh app_install \
  --duration 30 --behaviour minimal

./capture.sh app_install \
  --duration 30 --behaviour large

./capture.sh dht_sync_lag \
  --duration 60 --agents 2 --behaviour write:1 --behaviour record_lag:1

./capture.sh first_call \
  --duration 30

./capture.sh full_arc_create_validated_zero_arc_read \
  --duration 60 --agents 3 --behaviour zero:1 --behaviour full:2

./capture.sh local_signals \
  --duration 30

./capture.sh mixed_arc_get_agent_activity \
  --duration 60 --agents 6 --behaviour zero_read:3 --behaviour zero_write:1 --behaviour full_write:2

./capture.sh mixed_arc_must_get_agent_activity \
  --duration 60 --agents 6 --behaviour zero_must_get_agent_activity:3 --behaviour zero_write:1 --behaviour full_write:2

MIN_AGENTS=2 ./capture.sh remote_call_rate \
  --duration 30 --agents 2

MIN_AGENTS=2 ./capture.sh remote_signals \
  --duration 30 --agents 2

./capture.sh single_write_many_read \
  --duration 60

NO_VALIDATION_COMPLETE=1 MIN_AGENTS=10 ./capture.sh validation_receipts \
  --duration 60 --agents 10

MIN_AGENTS=2 ./capture.sh write_get_agent_activity \
  --duration 60 --agents 2 --behaviour write:1 --behaviour get_agent_activity:1

./capture.sh write_query \
  --duration 60

./capture.sh write_read \
  --duration 60

./capture.sh write_validated \
  --duration 60

MIN_AGENTS=2 ./capture.sh write_validated_must_get_agent_activity \
  --duration 60 --agents 2 --behaviour write:1 --behaviour must_get_agent_activity:1

./capture.sh zero_arc_create_and_read \
  --duration 500 --agents 4 --behaviour zero_read:1 --behaviour zero_write:1 --behaviour full:2

./capture.sh zero_arc_create_data \
  --duration 500 --agents 3 --behaviour zero:1 --behaviour full:2

./capture.sh zero_arc_create_data_validated \
  --duration 500 --agents 3 --behaviour zero:1 --behaviour full:2

./capture.sh zome_call_single_value \
  --duration 30
