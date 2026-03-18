#!/usr/bin/env bash

# this script is executed on Nomad clients, after a scenario has been run, to upload metrics with telegraf.
# It also takes care of setting the RUN_ID if unset from env

set -euo pipefail

# Chunk writes to reduce per-request load on InfluxDB.
INFLUX_WRITE_CHUNK_LINES=10000
INFLUX_WRITE_RETRY_MAX_ATTEMPTS=3
INFLUX_WRITE_RETRY_DELAY_S=2

function check_envset() {
  local var_name="$1"
  if [[ -z "${!var_name:-}" ]]; then
    echo "Environment variable $var_name is not set or is an empty string." >&2
    exit 1
  fi
}

function upload_metric_file() {
  local metric_file="$1"
  local out_file="$2"

  # Split the line protocol file and upload chunks sequentially.
  local chunk_dir
  chunk_dir="$(mktemp -d)"

  # Always clean up chunk files even if a write fails.
  trap 'rm -rf "$chunk_dir"' RETURN
  split -l "$INFLUX_WRITE_CHUNK_LINES" -d -a 6 --additional-suffix=".influx" "$out_file" "$chunk_dir/chunk-"

  # Avoid iterating a literal glob when split produced no chunks.
  local old_nullglob; old_nullglob=$(shopt -p nullglob)
  trap 'eval "$old_nullglob"; rm -rf "$chunk_dir"' RETURN
  shopt -s nullglob
  local -a chunk_files=("$chunk_dir"/chunk-*.influx)
  if (( ${#chunk_files[@]} == 0 )); then
    return 0
  fi

  for chunk_file in "${chunk_files[@]}"; do
    local attempt=1

    while (( attempt <= INFLUX_WRITE_RETRY_MAX_ATTEMPTS )); do
      echo "Uploading chunk ${chunk_file} for ${metric_file} (attempt ${attempt}/${INFLUX_WRITE_RETRY_MAX_ATTEMPTS})"
      if influx write \
          --host "$INFLUX_HOST" \
          --bucket "$influx_bucket" \
          --org "holo" \
          --file "$chunk_file"; then
        break
      fi

      if (( attempt == INFLUX_WRITE_RETRY_MAX_ATTEMPTS )); then
        echo "Failed uploading chunk ${chunk_file} for ${metric_file} after ${INFLUX_WRITE_RETRY_MAX_ATTEMPTS} attempts." >&2
        return 1
      fi

      echo "Chunk upload failed for ${chunk_file}; retrying in ${INFLUX_WRITE_RETRY_DELAY_S}s..." >&2
      sleep "$INFLUX_WRITE_RETRY_DELAY_S"
      ((attempt++))
    done
  done
}

check_envset "WT_METRICS_DIR"
check_envset "INFLUX_TOKEN"
check_envset "INFLUX_HOST"
check_envset "NOMAD_ALLOC_DIR"

influx_bucket="${INFLUX_BUCKET:-windtunnel}"

# if RUN_ID is NOT set, try to get it from run_summary.jsonl
echo "Current RUN_ID: '${RUN_ID:-unset}'"
if [ "${RUN_ID:+x}" != "x" ]; then
    # if is set RUN_SUMMARY_PATH
    summary_path=${RUN_SUMMARY_PATH:-"run_summary.jsonl"}

    if [ -f "$summary_path" ]; then
        RUN_ID=$(jq --slurp --raw-output 'sort_by(.started_at|tonumber) | last | .run_id' < "$summary_path")
    else
        echo "Run summary file not found: $summary_path" >&2
        exit 1
    fi
    export RUN_ID
    echo "RUN_ID: '$RUN_ID'"
else
    echo "RUN_ID is already set to '$RUN_ID'"
fi

# for each metric file, import to influx
# Avoid iterating a literal glob when there are no metric files.
shopt -s nullglob
metric_files=("$WT_METRICS_DIR"/*.influx)
if (( ${#metric_files[@]} == 0 )); then
    echo "No metrics files found in ${WT_METRICS_DIR}, skipping upload."
    exit 0
fi

for metric_file in "${metric_files[@]}"; do
    echo "Importing $metric_file"
    out_file="$NOMAD_ALLOC_DIR/$(basename "$metric_file")"
    # Tag metrics with RUN_ID, if set
    if [[ "${RUN_ID:+x}" == "x" ]]; then
        lp-tool -input "$metric_file" -output "$out_file" -tag run_id="$RUN_ID"
    else
        cp "$metric_file" "$out_file"
    fi
    # import metrics to influx
    upload_metric_file "$metric_file" "$out_file"
    echo "Finished importing $metric_file"
done
