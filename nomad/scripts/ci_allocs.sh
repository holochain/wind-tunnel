#!/usr/bin/env bash

function make_allocs_csv() {
    local output_file=$1
    local job_name=$2
    local scenario_name=$3
    local run_id=$4
    local started_at=$5
    local alloc_ids=$6

    touch $output_file

    local duration
    local behaviours
    local csv_behaviours

    duration=$(jq -r '.duration // 300' "nomad/vars/${job_name}.json")
    behaviours=$(jq -r '(.behaviours // [""])[]' "nomad/vars/${job_name}.json")
    csv_behaviours=""
    for behaviour in $behaviours; do
        if [ -z "$csv_behaviours" ]; then
            csv_behaviours="${behaviour}"
        else
            csv_behaviours="${csv_behaviours} ${behaviour}"
        fi
    done
    for alloc_id in $alloc_ids; do
        # job_name,scenario_name,alloc_id,run_id,started_at,duration,behaviours
        echo "${job_name},${scenario_name},${alloc_id},${run_id},${started_at},${duration},${csv_behaviours}" >> ${output_file}
    done
}

function generate_run_summary() {
    local allocs_csv_file=$1
    local run_summary_file=$2

    if [ ! -f "$allocs_csv_file" ]; then
        echo "Alloc CSV file not found: $allocs_csv_file"
        return 1
    fi

    touch $run_summary_file

    local wind_tunnel_version
    wind_tunnel_version=$(cargo metadata --no-deps --format-version 1 | jq -r ".packages[0].version")

    local scenario_name
    local alloc_id
    local run_id
    local started_at
    local duration
    local behaviours
    local json_behaviours
    local peer_count

    while IFS=',' read -r _job_name scenario_name alloc_id run_id started_at duration behaviours; do
        echo "Processing line: $run_id / $scenario_name / $alloc_id / $started_at / $duration / $behaviours"
        jq -cn \
          --arg run_id "$run_id" \
          --arg scenario_name "$scenario_name" \
          --argjson started_at "$started_at" \
          --arg behaviours "${behaviours:-}" \
          --arg wind_tunnel_version "$wind_tunnel_version" \
          --argjson duration "${duration:-0}" '
            # Split behaviours field; default to [""]
            ($behaviours | if . == "" then [""] else split(" ") end) as $bs
            | {
                run_id: $run_id,
                scenario_name: $scenario_name,
                started_at: $started_at,
                duration: $duration,
                assigned_behaviours: (reduce $bs[] as $b ({}; .[$b] += 1)),
                peer_count: ($bs | length),
                peer_end_count: ($bs | length),
                env: {},
                wind_tunnel_version: $wind_tunnel_version
              }
        ' >> "$run_summary_file"
    done < "$allocs_csv_file"
}

function wait_for_jobs() {
    local allocs_csv_file=$1

    local nomad_script_dir
    nomad_script_dir=$(dirname "$0")

    local job_name
    local alloc_id

    while IFS= read -r line; do
        job_name=$(echo "$line" | cut -d',' -f1)
        alloc_id=$(echo "$line" | cut -d',' -f3)
        echo "Waiting for $job_name with allocation ID $alloc_id"
        $nomad_script_dir/wait_for_jobs.sh $job_name $alloc_id
    done < "$allocs_csv_file"
}

set -euo pipefail

cmd=$1
shift

case $cmd in
    "make_allocs_csv")
        make_allocs_csv "$@"
        ;;
    "generate_run_summary")
        generate_run_summary "$@"
        ;;
    "wait_for_jobs")
        wait_for_jobs "$@"
        ;;
    *)
        echo "Unknown command: $cmd"
        exit 1
        ;;
esac

