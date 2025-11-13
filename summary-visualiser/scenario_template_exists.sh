#!/usr/bin/env bash

if [ -z "$1" ]; then
    echo "Error: scenario name argument required" >&2
    exit 1
fi

if [ -f "$SCENARIO_TEMPLATES_DIR/$1.html.tmpl" ]; then
    echo "1"
fi
