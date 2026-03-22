#!/usr/bin/env bash
# Counts Nomad nodes that are eligible to run wind-tunnel jobs.
#
# Modes:
#   (default)                     Count free eligible nodes.
#   --eligible-only               Count eligible nodes regardless of current allocation status.
#   --include-threefold-node-pool Include threefold pool nodes; default excludes them.
#
# Eligibility criteria:
#   - Nomad version >= 1.11.0 (matches the constraint in run_scenario.tpl.hcl)
#   - Status: ready
#   - Scheduling eligibility: eligible
#   - Pool filter: exclude `threefold` unless explicitly included
#   - Capacity key: unique `attr.unique.hostname` values (matches distinct_property)
#
# Requires env vars: NOMAD_ADDR, NOMAD_TOKEN, NOMAD_CACERT
# Optional env var:  NOMAD_BIN — path to the nomad binary (defaults to "nomad")

set -euo pipefail

NOMAD="${NOMAD_BIN:-nomad}"
ELIGIBLE_ONLY=false
INCLUDE_THREEFOLD_NODE_POOL=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --eligible-only)
            ELIGIBLE_ONLY=true
            shift
            ;;
        --include-threefold-node-pool)
            INCLUDE_THREEFOLD_NODE_POOL=true
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

include_threefold_json="$INCLUDE_THREEFOLD_NODE_POOL"

# Filter for eligible nodes with version >= 1.11.0 (matching run_scenario.tpl.hcl)
# and pool policy (exclude threefold by default).
eligible_nodes_json=$(<<< "$nodes_json" jq --argjson include_threefold "$include_threefold_json" '[.[] | select(
    .Status == "ready" and
    .SchedulingEligibility == "eligible" and
    ($include_threefold or ((.NodePool // "") != "threefold")) and
    (.Version | split(".") | map(split("-")[0] | tonumber) as $v |
        ($v[0] > 1) or
        ($v[0] == 1 and $v[1] > 11) or
        ($v[0] == 1 and $v[1] == 11)
    )
)]')

echo "Found eligible nodes: $(<<< "$eligible_nodes_json" jq length)" >&2
echo "Found eligible unique hostnames: $(<<< "$eligible_nodes_json" jq '[ .[] | (.Attributes["unique.hostname"] // .Name) ] | unique | length')" >&2

if [[ "$ELIGIBLE_ONLY" == "true" ]]; then
    <<< "$eligible_nodes_json" jq '[ .[] | (.Attributes["unique.hostname"] // .Name) ] | unique | length'
    exit 0
fi

# Query the Nomad allocations API directly for running allocations only.
# The API is used because it supports server-side filtering which the nomad cli tool lacks,
# and the full list was large enough to crash jq.
allocs_response=$(curl -sS --fail \
  --cacert "$NOMAD_CACERT" \
  -H "X-Nomad-Token: $NOMAD_TOKEN" \
  "$NOMAD_ADDR/v1/allocations?namespace=*&filter=ClientStatus+%3D%3D+%22running%22") || {
  echo "ERROR: Nomad allocations API request failed" >&2
  exit 1
}
busy_json=$(<<< "$allocs_response" jq '[.[] | .NodeID] | unique')

echo "There are currently $(<<< "$busy_json" jq length) nodes with running allocations" >&2

# Filter out nodes with running allocations and count effective capacity
# by unique hostname (matches distinct_property behavior).
<<< "$eligible_nodes_json" jq \
  --argjson busy "$busy_json" \
  '[.[] | select(.ID | IN($busy[]) | not) | (.Attributes["unique.hostname"] // .Name) ] | unique | length'
