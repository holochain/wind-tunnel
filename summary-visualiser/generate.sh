#!/bin/bash

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

# Accept either a filename or stdin, and add the JS file as a helper template
# so that it can be inlined into the page.
gomplate \
    -c .="${1:-stdin:///in.json}" \
    -t js="$script_dir/assets/windTunnel.js" \
    "${template_args[@]}" \
    -f "$script_dir/templates/page.html.tmpl"
