#!/bin/bash

set -euo pipefail

# Timeout in seconds (30 minutes). Can be overridden via environment.
TIMEOUT="${TIMEOUT:-1800}"

function check_envset() {
  local var_name="$1"
  if [[ -z "${!var_name:-}" ]]; then
    echo "Environment variable $var_name is not set or is an empty string." >&2
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

function get_nomad_status() {
  local alloc_id="$1"
  nomad alloc status -json "${alloc_id}"
}

function get_status() {
    local nomad_status="$1"
    # Fetch JSON and extract ClientStatus; fail fast if the command errors.
    local client_status
    if ! client_status="$(echo "$nomad_status" | jq -r '.ClientStatus')"; then
        echo "Failed to retrieve status for allocation ID: $alloc_id" >&2
        exit 1
    fi
    if [[ -z "$client_status" || "$client_status" == "null" ]]; then
        echo "No job found with allocation ID: $alloc_id" >&2
        exit 1
    fi
    echo "$client_status"
}

# See https://developer.hashicorp.com/nomad/commands/job/status#running
function is_running() {
    local status="$1"
    case "$status" in
      running)
        return 0 ;;  # still running
      *)
        return 1 ;;  # not running
    esac
}

function is_run_success() {
    local status="$1"
    [[ "$status" == "complete" ]]
}

function print_failed_tasks_and_logs() {
  local alloc_id="$1"
  local status="$2"

  # get failed tasks and create an object for each status with the task name and the error message
  failed_tasks=$(jq -r '
    .TaskStates
    | to_entries[]
    | . as $entry
    | $entry.value.Events[]?
    | {
      task: $entry.key,
      message: .DisplayMessage,
      details: .Details
    }
  ' <<< "$status")

  # get logs for each (unique) task
  local tasks
  tasks="$(echo "$failed_tasks" | jq -r '.task' | sort -u)"

  for task in $tasks; do
    echo "Fetching logs for task: $task"
    nomad alloc logs -stderr "$alloc_id" "$task" || echo "Failed to fetch stderr logs for task: $task"
    nomad alloc logs -stdout "$alloc_id" "$task" || echo "Failed to fetch stdout logs for task: $task"
  done

  echo "Failed task: $failed_tasks"
}

function wait_for_job() {
    local scenario_name="$1"
    local alloc_id="$2"

    echo "Waiting for job: $scenario_name ($alloc_id)"

    ELAPSED_SECS=0
    # Run until timeout or no scenario is still running
    while true; do
        local nomad_status
        if ! nomad_status="$(get_nomad_status "$alloc_id")"; then
            echo "Failed to fetch Nomad status for $scenario_name ($alloc_id)" >&2
            exit 1
        fi
        local status
        status="$(get_status "$nomad_status")"

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

        # Job has completed at this point, either successfully or failed.
        if is_run_success "$status"; then
            echo "Scenario $scenario_name ($alloc_id) completed successfully in $ELAPSED_SECS seconds."
            return 0
        else
            echo "Scenario $scenario_name ($alloc_id) finished with status=$status after $ELAPSED_SECS seconds."
            print_failed_tasks_and_logs "$alloc_id" "$nomad_status"
            return 1
        fi
    done
}


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

# Count total number of allocations and track failures
TOTAL_ALLOCATIONS=$#
FAILED_ALLOCATIONS=0

# Process each allocation ID passed as arguments
while [[ $# -gt 0 ]]; do
    # Get next allocation ID from arguments
    alloc_id="$1"

    if ! wait_for_job "$SCENARIO_NAME" "$alloc_id"; then
        FAILED_ALLOCATIONS=$((FAILED_ALLOCATIONS + 1))
    fi

    shift # Remove the processed allocation ID
done

# If all allocations failed, exit with error
if [[ $FAILED_ALLOCATIONS -eq $TOTAL_ALLOCATIONS ]]; then
    echo "Error: All $TOTAL_ALLOCATIONS allocation(s) failed." >&2
    exit 1
fi

exit 0
