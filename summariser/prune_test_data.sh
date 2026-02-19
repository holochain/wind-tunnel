#!/usr/bin/env bash
#
# Find (and optionally remove) unused query-result test data files.
#
# When the summariser loads a query result from test_data/2_query_results/ it
# updates the file's modification time via set_modified().  Any file that is
# NOT touched during a full test run was never needed by any current test and
# can be safely deleted.
#
# Usage (run from anywhere inside the repo):
#   ./summariser/prune_test_data.sh           # dry run: list unused files
#   ./summariser/prune_test_data.sh --delete  # delete unused files and re-verify

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
QUERY_DIR="$REPO_ROOT/summariser/test_data/2_query_results"
DELETE=false

for arg in "$@"; do
    case "$arg" in
        --delete) DELETE=true ;;
        *) echo "Unknown argument: $arg" >&2; exit 1 ;;
    esac
done

# Create a sentinel file.  After the test run, any 2_query_results file whose
# mtime is <= the sentinel was not loaded and is a candidate for deletion.
SENTINEL=$(mktemp)
trap 'rm -f "$SENTINEL"' EXIT

# Sleep briefly to ensure the sentinel's mtime is before any test-touched files.
sleep 1

echo "Running tests (this touches active query-result files)..."
cargo test -p holochain_summariser

# Collect files not touched during the test run.
mapfile -t UNUSED < <(find "$QUERY_DIR" -maxdepth 1 -name '*.json' -not -newer "$SENTINEL" | sort)

if [ "${#UNUSED[@]}" -eq 0 ]; then
    echo "No unused test data files found."
    exit 0
fi

echo ""
echo "Unused test data files (${#UNUSED[@]}):"
for f in "${UNUSED[@]}"; do
    echo "  $(basename "$f")"
done

if [ "$DELETE" = false ]; then
    echo ""
    echo "Re-run with --delete to remove them and verify the tests still pass."
    exit 0
fi

echo ""
echo "Deleting ${#UNUSED[@]} unused file(s)..."
for f in "${UNUSED[@]}"; do
    git rm -- "$f"
done

echo "Verifying tests still pass after deletion..."
cargo test -p holochain_summariser

echo ""
echo "Done. Deleted ${#UNUSED[@]} unused file(s)."
