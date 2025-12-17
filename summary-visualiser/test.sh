#!/usr/bin/env bash

# This test tries to run the script and look for an expected HTML element that
# shows that a template could be found for the scenario type given in the JSON.

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
if [[ ! -d "$script_dir" ]]; then
    echo "Could not find script directory: $script_dir" >&2
    exit 1
fi

quiet=false
if [[ "${1:-}" == "--quiet" ]] || [[ "${1:-}" == "-q" ]]; then
    quiet=true
fi

# Summaries from a recent run of all scenarios can be found in test_data/all.json.
# If any template has an error in it, this command will fail.
# Note: unsupported scenarios (ones without templates) won't cause a failure;
# instead, it'll output a `<section class="scenario-not-found scenario-not-found-<foo>">` element
# with some notice text about the missing template.
html_output="$("$script_dir/generate.sh" "$script_dir/test_data/all.json")"

# Check that the scenario has an HTML element in the output.
# Arguments:
#   $1: the name of the scenario in snake_case or kebab-case
smoke_test_scenario() {
    scenario_class_name_str="${1//_/-}"
    # Test for an expected HTML element that shows it found a template for this scenario.
    expected_html_tag="<section class=\"scenario scenario-$scenario_class_name_str\">"
    expected_element_in_output=$(echo "$html_output" | grep "$expected_html_tag")
    if [ -n "$expected_element_in_output" ]; then
        if [ "$quiet" = false ]; then
            echo "Found expected .scenario-$scenario_class_name_str element in output"
        fi
    else
        echo "Couldn't find expected .scenario-$scenario_class_name_str element in output" >&2
        exit 1
    fi
}

smoke_test_scenario "app_install"
smoke_test_scenario "dht_sync_lag"
smoke_test_scenario "first_call"
smoke_test_scenario "local_signals"
smoke_test_scenario "remote_call_rate"
smoke_test_scenario "remote_signals"
smoke_test_scenario "single_write_many_read"
smoke_test_scenario "two_party_countersigning"
smoke_test_scenario "validation_receipts"
smoke_test_scenario "write_get_agent_activity"
smoke_test_scenario "write_query"
smoke_test_scenario "write_read"
smoke_test_scenario "write_validated_must_get_agent_activity"
smoke_test_scenario "write_validated"
smoke_test_scenario "zero_arc_create_and_read"
smoke_test_scenario "zero_arc_create_data_validated"
smoke_test_scenario "zero_arc_create_data"
smoke_test_scenario "zome_call_single_value"
smoke_test_scenario "zero_arc_create_data_validated"

# Finally, lint the generated HTML to ensure it's valid.
echo "$html_output" | tidy -eq
