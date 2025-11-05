#!/bin/bash

set -euo pipefail

SCRIPT_DIR=$(dirname "$0")
# Read either from a filename or stdin.
INPUT_JSON=$(cat "${1:-/dev/stdin}")
# The scenario name is pulled from the JSON itself.
# We'll need this later to fetch the right template.
SCENARIO_NAME=$(echo "$INPUT_JSON" | jq -r ".run_summary.scenario_name")

# Get all the helper templates and make them available to gomplate
# via their unadorned filename
# (e.g., `delta` not `templates/helpers/delta.html.tmpl`).
HELPER_TEMPLATE_ARGS=()
for helper_file in "$SCRIPT_DIR/templates/helpers/"*.html.tmpl; do
    [ -e "$helper_file" ] || continue
    helper_name=$(basename "$helper_file" .html.tmpl)
    HELPER_TEMPLATE_ARGS+=(-t "$helper_name=$helper_file")
done

echo "$INPUT_JSON" | gomplate \
    -c .="stdin:///in.json" \
    -t page="$SCRIPT_DIR/templates/page.html.tmpl" \
    -t js="$SCRIPT_DIR/assets/windTunnel.js" \
    "${HELPER_TEMPLATE_ARGS[@]}" \
    -f "$SCRIPT_DIR/templates/scenarios/$SCENARIO_NAME.html.tmpl"