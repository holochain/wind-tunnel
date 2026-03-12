#!/usr/bin/env bash

run_local_durable_object_store() {
    if [[ -z "${UNYT_DURABLE_OBJECTS_URL:-}" ]]; then
        echo "Error: UNYT_DURABLE_OBJECTS_URL is not set" >&2
        return 1
    fi
    if [[ -z "${UNYT_DURABLE_OBJECTS_SECRET:-}" ]]; then
        echo "Error: UNYT_DURABLE_OBJECTS_SECRET is not set" >&2
        return 1
    fi

    local port=${UNYT_DURABLE_OBJECTS_URL##*:}
    if [[ ! "$port" =~ ^[0-9]+$ ]]; then
        echo "Error: Could not extract valid port from UNYT_DURABLE_OBJECTS_URL" >&2
        return 1
    fi

    local repo_root
    repo_root="$(git rev-parse --show-toplevel)"
    wrangler dev --types --config="$repo_root/durable_object_store/wrangler.jsonc" --local --port "$port" --var "SECRET_KEY:$UNYT_DURABLE_OBJECTS_SECRET"
}
