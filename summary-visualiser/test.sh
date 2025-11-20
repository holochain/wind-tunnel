#!/usr/bin/env bash

# This test tries to run the script and look for an expected HTML element that
# shows that a template could be found for the scenario type given in the JSON.

script_dir=$(dirname "$0")

# Load up some JSON data and try to generate an HTML page from it.
# Arguments:
#   $1: a JSON string
#   $2: a description of the JSON string being tested
#   $3: the scenario name
test_output() {
    html_output=$(echo "$1" | "$script_dir/generate.sh")
    # Test for an expected HTML element that shows it found a template for this scenario.
    expected_html_tag="<section class=\"scenario scenario-${3//_/-}\">"
    expected_element_in_output=$(echo "$html_output" | grep "$expected_html_tag")
    if [ -n "$expected_element_in_output" ]; then
        echo "Found expected .scenario-$3 element in output for $2"
    else
        echo "Couldn't find expected .scenario-$3 element in output for $2"
        exit 1
    fi
}

# Load up both a representative sample JSON file and a real sample taken from a CI run,
# and try to turn both of them into HTML files.
# Arguments:
#   $1: the scenario name, will be used in the filename and the test messaging
#   $2: the unique identifier for the scenario name's JSON file in summariser/test_data/3_summary_outputs/
test_output_sample_real() {
    test_output "[$(cat "$script_dir/../summariser/test_data/3_summary_outputs/$1-$2.json")]" "well-populated sample JSON - $1" "$1"
    test_output "$(cat "$script_dir/test_data/$1.json")" "real JSON snapshot with some null metrics - $1" "$1"
}

test_output_sample_real "dht_sync_lag" "3a1e33ccf661bd873966c539d4d227e703e1496fb54bb999f7be30a3dd493e51"
test_output_sample_real "remote_call_rate" "f92e98962b23bfe104373a735dd9af8eb363e347a0c528902d4a2aaa8351cd74"
