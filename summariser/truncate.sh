#!/usr/bin/env bash
#
# Truncate query result arrays to a maximum of 5000 elements to keep test data manageable.
#
# Usage:
#   ./summariser/truncate.sh

set -euo pipefail

MAX_ELEMENTS=5000
DIR="$(git rev-parse --show-toplevel)/summariser/test_data/2_query_results"

for file in "$DIR"/*.json; do
    len=$(jq 'length' "$file")
    if [ "$len" -gt "$MAX_ELEMENTS" ]; then
        echo "Truncating $(basename "$file"): $len -> $MAX_ELEMENTS"
        jq ".[0:$MAX_ELEMENTS]" "$file" > "$file.tmp" && mv "$file.tmp" "$file"
    fi
done

echo "Done."
