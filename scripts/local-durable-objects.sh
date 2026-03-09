#!/usr/bin/env bash

run_local_durable_object_store() {
    local port=${UNYT_DURABLE_OBJECTS_URL##*:}

    wrangler dev --types --config="$(pwd)/durable_object_store/wrangler.jsonc" --local --port "$port" --var "SECRET_KEY:$UNYT_DURABLE_OBJECTS_SECRET"
}
