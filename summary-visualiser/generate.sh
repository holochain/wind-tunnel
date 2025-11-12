#!/bin/bash

SCRIPT_DIR=$(dirname "$0")

# Get all the helper templates and make them available to gomplate
# via their unadorned filename
# (e.g., `delta` not `templates/helpers/delta.html.tmpl`).
TEMPLATE_ARGS=()
for helper_file in "$SCRIPT_DIR/templates/helpers/"*.html.tmpl; do
    [ -e "$helper_file" ] || continue
    helper_name=$(basename "$helper_file" .html.tmpl)
    TEMPLATE_ARGS+=(-t "$helper_name=$helper_file")
done

# Now do the same for the scenario templates.
for scenario_file in "$SCRIPT_DIR/templates/scenarios/"*.html.tmpl; do
    [ -e "$scenario_file" ] || continue
    scenario_name=$(basename "$scenario_file" .html.tmpl)
    TEMPLATE_ARGS+=(-t "$scenario_name=$scenario_file")
done

# Accept either a filename or stdin, and add the JS file as a helper template
# so that it can be inlined into the page.
gomplate \
    -c .="${1:-stdin:///in.json}" \
    -t js="$SCRIPT_DIR/assets/windTunnel.js" \
    "${TEMPLATE_ARGS[@]}" \
    -f "$SCRIPT_DIR/templates/page.html.tmpl"
