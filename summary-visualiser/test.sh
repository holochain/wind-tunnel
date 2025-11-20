#!/usr/bin/env bash

# This test tries to run the script and look for an expected HTML element that
# shows that a template could be found for the scenario type given in the JSON.

script_dir=$(dirname "$0")

test_output() {
    html_output=$(echo "$1" | "$script_dir/generate.sh")
    expected_element_in_output=$(echo "$html_output" | grep '<section class="scenario scenario-dht-sync-lag">')
    if [ -n "$expected_element_in_output" ]; then
        echo "Found expected .scenario-dht-sync-lag element in output for $2"
    else
        echo "Couldn't find expected .scenario-dht-sync-lag element in output for $2"
        exit 1
    fi
}

test_output "[$(cat "$script_dir/../summariser/test_data/3_summary_outputs/dht_sync_lag-3a1e33ccf661bd873966c539d4d227e703e1496fb54bb999f7be30a3dd493e51.json")]" "well-populated sample JSON - dht_sync_lag"
test_output "$(cat "$script_dir/test_data/dht_sync_lag.json")" "real JSON snapshot with some null metrics - dht_sync_lag"

test_output "[$(cat "$script_dir/..summariser/test_data/3_summary_outputs/remote_call_rate-f92e98962b23bfe104373a735dd9af8eb363e347a0c528902d4a2aaa8351cd74.json")]" "well-populated sample JSON - remote_call_rate"
test_output "$(cat "$script_dir/test_data/remote_call_rate.json")" "real JSON snapshot with some null metrics - remote_call_rate"
