#!/usr/bin/env bash

script_dir=$(dirname "$0")

# Get all the helper templates and make them available to gomplate
# via their unadorned filename
# (e.g., `delta` not `templates/helpers/delta.html.tmpl`).
template_args=()
for template_dir in "helpers" "scenarios" ; do
    for helper_file in "$script_dir/templates/$template_dir/"*.html.tmpl; do
        [ -e "$helper_file" ] || continue
        helper_name=$(basename "$helper_file" .html.tmpl)
        template_args+=(-t "$helper_name=$helper_file")
    done
done

# This command does three things worth noting:
# * Accept either a filename or stdin
# * Add the JS file as a helper template so that it can be inlined into the page
# * Set up a helper plugin to test whether a template exists for a given scenario
#   (note: this requires `SCENARIO_TEMPLATES_DIR` to be set)
SCENARIO_TEMPLATES_DIR="$script_dir/templates/scenarios" gomplate \
    -c .="${1:-stdin:///in.json}" \
    -t js="$script_dir/assets/wind_tunnel.js" \
    -t css="$script_dir/assets/wind_tunnel.css" \
    "${template_args[@]}" \
    --plugin scenario_template_exists="$script_dir/scenario_template_exists.sh" \
    -f "$script_dir/templates/page.html.tmpl"
