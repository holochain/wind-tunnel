#!/bin/bash

set -euo pipefail

# Timeout in seconds (30 minutes). Can be overridden via environment.
TIMEOUT="${TIMEOUT:-1800}"

function check_envset() {
  local var_name="$1"
  if [[ -z "${!var_name}" ]]; then
    echo "Environment variable $var_name is not set." >&2
    exit 1
  fi
}

function check_command() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "Command '$cmd' not found in PATH." >&2
    exit 1
  fi
}


function get_status() {
    local alloc_id="$1"
    # Fetch JSON and extract ClientStatus; fail fast if the command errors.
    local client_status
    if ! client_status="$(nomad alloc status -json "${alloc_id}" | jq -r '.ClientStatus')"; then
        echo "Failed to retrieve status for allocation ID: $alloc_id" >&2
        exit 1
    fi
    if [[ -z "$client_status" || "$client_status" == "null" ]]; then
        echo "No job found with allocation ID: $alloc_id" >&2
        exit 1
    fi
    echo "$client_status"
}

function is_running() {
    local status="$1"
    case "$status" in
      complete|failed|lost|stopped|dead|unknown)
        return 1 ;;  # not running
      *)
        return 0 ;;  # still running (e.g., pending, running)
    esac
}

function is_run_success() {
    local status="$1"
    [[ "$status" == "complete" ]]
}

function wait_for_job() {
    local scenario_name="$1"
    local alloc_id="$2"

    echo "Waiting for job: $scenario_name ($alloc_id)"

    ELAPSED_SECS=0
    # Run until timeout or no scenario is still running
    while true; do
        status=$(get_status "$alloc_id")

        if is_running "$status"; then
            echo "Scenario $scenario_name ($alloc_id) is still running (status=$status) (elapsed: $ELAPSED_SECS seconds)."
            sleep 1
            ELAPSED_SECS=$((ELAPSED_SECS + 1))
            if [[ $ELAPSED_SECS -gt $TIMEOUT ]]; then
                echo "Timeout reached after $TIMEOUT seconds."
                exit 255
            fi
            continue
        fi

        # Job has completed
        if is_run_success "$status"; then
            break
        fi

        echo "Scenario $scenario_name ($alloc_id) failed (status=$status) after $ELAPSED_SECS seconds."
        exit 1
    done
    echo "Scenario $scenario_name ($alloc_id) completed successfully in $ELAPSED_SECS seconds."

    return 0
}


# verify NOMAD variables are set
# verify NOMAD variables are set
check_envset "NOMAD_CACERT"
check_envset "NOMAD_ADDR"
check_envset "NOMAD_TOKEN"

# verify required tools are available
check_command "nomad"
check_command "jq"

# Scenario name must be passed as an argument; at least one allocation ID must be provided
if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <scenario_name> <allocation_id> [more_alloc_ids...]"
  exit 1
fi

SCENARIO_NAME="$1"

shift # Remove the first argument (scenario name)

# Process each allocation ID passed as arguments
while [[ $# -gt 0 ]]; do
    # Get next allocation ID from arguments
    alloc_id="$1"
    wait_for_job "$SCENARIO_NAME" "$alloc_id"

    shift # Remove the processed allocation ID
done

exit 0
