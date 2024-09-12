#!/usr/bin/env bash

# array to store background process PIDs
pids=()

# cleanup function
clean_up() {
    echo "Cleaning up..."
    for pid in "${pids[@]}"; do
        kill "$pid" 2>/dev/null
    done
    exit
}

# set up trap for multiple signals
trap clean_up INT TERM

# start hc-chc-service
hc-chc-service --port 8181 &
pids+=($!)
sleep 1

# start hc-sandbox
hc s clean && echo "1234" | hc s --piped create --chc-url http://localhost:8181 && echo "1234" | hc s --piped -f 8888 run &
pids+=($!)

wait