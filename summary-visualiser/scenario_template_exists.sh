#!/usr/bin/env bash

if [ -f "$SCENARIO_TEMPLATES_DIR/$1.html.tmpl" ]; then
    echo "yes $SCENARIO_TEMPLATES_DIR/$1.html.tmpl"
else
    echo "no $SCENARIO_TEMPLATES_DIR/$1.html.tmpl"
fi
