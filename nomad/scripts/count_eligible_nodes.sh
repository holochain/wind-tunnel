#!/usr/bin/env bash
# Counts Nomad nodes that are eligible to run wind-tunnel jobs.
#
# Modes:
#   (default)          Count eligible nodes that have no running allocations (free).
#   --eligible-only    Count eligible nodes regardless of current allocation status.
#
# Eligibility criteria:
#   - Nomad version >= 1.11.0 (matches the constraint in run_scenario.tpl.hcl)
#   - Status: ready
#   - Scheduling eligibility: eligible
#
# Requires env vars: NOMAD_ADDR, NOMAD_TOKEN, NOMAD_CACERT
# Optional env var:  NOMAD_BIN — path to the nomad binary (defaults to "nomad")

set -euo pipefail

NOMAD="${NOMAD_BIN:-nomad}"
ELIGIBLE_ONLY=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --eligible-only)
            ELIGIBLE_ONLY=true
            shift
            ;;
        *)
            echo "Unknown argument: $1" >&2
            exit 1
            ;;
    esac
done

nodes_json=$("$NOMAD" node status -json 2>&1) || {
  echo "ERROR: 'nomad node status -json' failed: $nodes_json" >&2
  exit 1
}
if ! echo "$nodes_json" | jq -e . >/dev/null 2>&1; then
  echo "ERROR: 'nomad node status -json' did not return valid JSON:" >&2
  echo "$nodes_json" >&2
  exit 1
fi

echo "Found total nodes: $(<<< "$nodes_json" jq length)" >&2

# Filter for eligible nodes with version >= 1.11.0 (matching the constraint in run_scenario.tpl.hcl).
eligible_nodes_json=$(<<< "$nodes_json" jq '[.[] | select(
    .Status == "ready" and
    .SchedulingEligibility == "eligible" and
    (.Version | split(".") | map(split("-")[0] | tonumber) as $v |
        ($v[0] > 1) or
        ($v[0] == 1 and $v[1] > 11) or
        ($v[0] == 1 and $v[1] == 11)
    )
)]')

echo "Found eligible nodes: $(<<< "$eligible_nodes_json" jq length)" >&2

if [[ "$ELIGIBLE_ONLY" == "true" ]]; then
    <<< "$eligible_nodes_json" jq 'length'
    exit 0
fi

# 'nomad job status -namespace * -json' returns the full allocation list across all namespaces.
# The response is buffered so we can handle the "No running jobs" plain-text fallback before
# passing valid JSON to jq for filtering.
nomad_jobs_stderr=$(mktemp)
jobs_json=$("$NOMAD" job status -namespace '*' -json 2>"$nomad_jobs_stderr") || {
  echo "ERROR: 'nomad job status -namespace * -json' failed: $jobs_json" >&2
  if [[ -s "$nomad_jobs_stderr" ]]; then
    echo "Nomad stderr: $(cat "$nomad_jobs_stderr")" >&2
  fi
  rm -f "$nomad_jobs_stderr"
  exit 1
}
if [[ -s "$nomad_jobs_stderr" ]]; then
  echo "WARNING (nomad job status stderr): $(cat "$nomad_jobs_stderr")" >&2
fi
rm -f "$nomad_jobs_stderr"

# When there are no running jobs, Nomad outputs "No running jobs" instead of JSON.
if [[ "$jobs_json" == "No running jobs" ]]; then
  busy_json='[]'
elif echo "$jobs_json" | jq -e . >/dev/null 2>&1; then
  busy_json=$(<<< "$jobs_json" jq '[.[].Allocations // [] | .[]] | map(select(.ClientStatus == "running") | .NodeID) | unique')
else
  echo "ERROR: 'nomad job status -namespace * -json' did not return valid JSON." >&2
  echo "First 500 chars of output: ${jobs_json:0:500}" >&2
  exit 1
fi

echo "There are currently $(<<< "$busy_json" jq length) nodes with running allocations" >&2

# Filter out nodes with running allocations and count the remainder.
<<< "$eligible_nodes_json" jq \
  --argjson busy "$busy_json" \
  '[.[] | select(.ID | IN($busy[]) | not)] | length'
