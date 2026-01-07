#!/usr/bin/env bash

# This script is used as a helper in the `gomplate` command in ./generate.sh
# to check whether a scenario template file exists for a given scenario name
# found in the input JSON.
# If one doesn't exist, `templates/helpers/scenarios_loop.html.tmpl` will skip over it
# with the message that a template couldn't be found.

if [ -z "$1" ]; then
    echo "Error: scenario name argument required" >&2
    exit 1
fi

if [ -f "$SCENARIO_TEMPLATES_DIR/$1.html.tmpl" ]; then
    echo "1"
fi
