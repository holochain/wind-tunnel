variable "duration" {
  type        = number
  description = "The maximum duration of the scenario run"
  {{- /* Default: read `duration` from the JSON data source `vars`.*/}}
  default     = {{ index (ds "vars") "duration" }}
}

variable "reporter" {
  type        = string
  description = "The method used to report the logs"
  {{- /* Default: read `reporter` from the JSON data source `vars`, or set to `"influx-file"` if not provided.*/}}
  default     = {{ index (ds "vars") "reporter" | default "influx-file" | quote }}
}

variable "holochain_bin_url" {
  type        = string
  description = "URL from which to download the `holochain` binary from to start conductors with"
  default     = null
}

variable "scenario_url" {
  type        = string
  description = "The URL to the local binary or download link to the zip file of the scenario under test"
}

variable "run_id" {
  type        = string
  description = "The ID of this run to distinguish it from other runs"
  {{- /* Default: read `run_id` from the JSON data source `vars`, or set to `null` if not provided. */}}
  default     = {{ with index (ds "vars") "run_id" }}{{ . | quote }}{{ else }}null{{ end }}
}

variable "include_threefold_node_pool" {
  type        = bool
  description = "Allow scheduling on the threefold node pool"
  {{- /* Default: exclude threefold unless explicitly enabled. */}}
  default     = {{ index (ds "vars") "include_threefold_node_pool" | default false }}
}

variable "wind_tunnel_git_ref" {
  type        = string
  description = "Git ref from which github hosted artifacts are fetched"
  default     = "main"
}

job "{{ (ds "vars").job_name }}" {
  type        = "batch"
  all_at_once = true // Try to run all groups at once
  // Use the all-pools node pool so cross-pool scheduling is possible.
  // Additional pool restrictions are applied with the `${node.pool}` constraint below.
  node_pool   = "all"

  // Exclude threefold nodes unless explicitly enabled by `include_threefold_node_pool`
  constraint {
    attribute = "${node.pool}"
    operator  = var.include_threefold_node_pool ? "regexp" : "!="
    value     = var.include_threefold_node_pool ? ".*" : "threefold"
  }

  constraint {
    distinct_hosts = true // Don't run multiple instances on the same client at once
  }

  constraint {
    distinct_property = "${attr.unique.hostname}"
  }

  constraint {
    attribute = "${attr.nomad.version}"
    operator  = "version"
    value     = ">= 1.11.0"
  }

  # Soft preference: weight toward nodes where the meta is set
  affinity {
    attribute = "${meta.unyt_agent_id}"
    operator  = "regexp"
    value     = ".+"
    weight    = 100
  }

  secret "job_secrets" {
    provider = "nomad"
    path     = "nomad/jobs"
  }

  dynamic "group" {
    for_each = [{{- $assignments := (index (ds "vars") "assignments" | default (coll.Slice)) -}}{{- range $aIdx, $assignment := $assignments -}}{{- $nodes := (index $assignment "nodes" | default 1) -}}{{- range $nodeIdx := math.Seq 0 (sub $nodes 1) -}}{{- if or (gt $aIdx 0) (gt $nodeIdx 0) -}},{{- end -}}{{ merge $assignment (dict "nodeIndex" $nodeIdx) | toJSON }}{{- end -}}{{- end -}}{{- if eq (len $assignments) 0 -}}{{ dict "behaviour" "default" | toJSON }}{{- end -}}]
    labels   = ["{{ (ds "vars").scenario_name }}-${group.key}-${group.value.behaviour}-${lookup(group.value, "nodeIndex", 0)}"]

    content {
      restart {
        attempts = 0
        mode = "fail"
      }

      reschedule {
        attempts  = 0
        unlimited = false
      }

      dynamic "task" {
        // Only run host metrics collector if `var.reporter` is set to `influx-file`.
        for_each = var.reporter == "influx-file" ? [var.reporter] : []
        labels   = ["report_host_metrics"]

        content {
          lifecycle {
            hook = "prestart"
            sidecar = true
          }

          env {
            TELEGRAF_CONFIG_PATH = "${NOMAD_TASK_DIR}/telegraf.host.conf"
            WT_METRICS_DIR       = "${NOMAD_ALLOC_DIR}/data/telegraf/metrics"
          }

          driver = "raw_exec"

          artifact {
            source = "https://raw.githubusercontent.com/holochain/wind-tunnel/${var.wind_tunnel_git_ref}/telegraf/telegraf.host.conf"
          }

          config {
            command = "sh"
            args    = ["-c", "mkdir -p ${WT_METRICS_DIR} && telegraf --config ${NOMAD_TASK_DIR}/telegraf.host.conf"]
          }
        }
      }

      dynamic "task" {
        // Only download the holochain binary if `var.holochain_bin_url` is set.
        for_each = var.holochain_bin_url != null ? [var.holochain_bin_url] : []
        labels   = ["download_holochain_bin"]

        content {
          restart {
            attempts = 5
            interval = "1m"
            delay    = "8s"
            mode     = "fail"
          }

          lifecycle {
            hook = "prestart"
            sidecar = false
          }

          driver = "raw_exec"

          artifact {
            source      = var.holochain_bin_url
            destination = "${NOMAD_ALLOC_DIR}/holochain"
            mode        = "file"
            chown       = true
          }

          config {
            command = "chmod"
            args    = ["+x", "${NOMAD_ALLOC_DIR}/holochain"]
          }
        }
      }

      dynamic "task" {
        // Only download the scenario binary if `var.scenario_url` is not a valid local path.
        for_each = fileexists(abspath(var.scenario_url)) ? [] : [var.scenario_url]
        labels   = ["download_scenario_bin"]

        content {
          restart {
            attempts = 5
            interval = "1m"
            delay    = "8s"
            mode     = "fail"
          }

          lifecycle {
            hook = "prestart"
            sidecar = false
          }

          driver = "raw_exec"

          artifact {
            source      = var.scenario_url
            destination = "${NOMAD_ALLOC_DIR}/scenario"
          }

          config {
            command = "chmod"
            args    = ["+x", "${NOMAD_ALLOC_DIR}/scenario/bin/{{ (ds "vars").scenario_name }}"]
          }
        }
      }

      task "run_scenario" {
        driver = "raw_exec"

        env {
          RUST_LOG                    = "info"
          HOME                        = "${NOMAD_TASK_DIR}"
          WT_METRICS_DIR              = "${NOMAD_ALLOC_DIR}/data/telegraf/metrics"
          RUN_SUMMARY_PATH            = "${NOMAD_ALLOC_DIR}/run_summary.jsonl"
          WT_HOLOCHAIN_PATH           = var.holochain_bin_url == null ? "holochain" : "${NOMAD_ALLOC_DIR}/holochain"
          UNYT_DURABLE_OBJECTS_URL    = secret.job_secrets.UNYT_DURABLE_OBJECTS_URL
          UNYT_DURABLE_OBJECTS_SECRET = secret.job_secrets.UNYT_DURABLE_OBJECTS_SECRET
          {{- range $key, $value := (index (ds "vars") "env" | default (coll.Dict)) }}
          {{ $key }} = "{{ $value }}"
          {{- end }}
        }

        template {
          // Wrapper used to run dynamically linked binaries via a
          // compatibility shim on runners that provide at /bin/wt-nix-ld-run.
          // Other runners fall back to direct execution.
          data = <<-EOF
          #!/usr/bin/env bash
          set -euo pipefail
          binary="$1"
          shift

          if [ -x /bin/wt-nix-ld-run ]; then
            exec /bin/wt-nix-ld-run "$binary" "$@"
          fi

          exec "$binary" "$@"
          EOF
          destination = "${NOMAD_TASK_DIR}/exec_with_optional_nixld.sh"
          perms       = "755"
        }

        config {
          command = "${NOMAD_TASK_DIR}/exec_with_optional_nixld.sh"
          // The `compact` function removes empty strings and `null` items from the list.
          args = compact([
            // If `var.scenario_url` is a valid local path then run that. Otherwise run the scenario downloaded by the `download_scenario_bin` task.
            fileexists(abspath(var.scenario_url)) ? abspath(var.scenario_url) : "${NOMAD_ALLOC_DIR}/scenario/bin/{{ (ds "vars").scenario_name }}",
            "--duration=${var.duration}",
            "--reporter=${var.reporter}",
            "--behaviour=${group.value.behaviour}:${lookup(group.value, "agents", 1)}",
            var.run_id != null ? "--run-id=${var.run_id}" : null,
            "--agents=${lookup(group.value, "agents", 1)}",
            "--no-progress"
          ])
        }

        resources {
          memory = 2048
        }
      }

      dynamic "task" {
        // Only upload the metrics if `var.reporter` is set to `influx-file`.
        for_each = var.reporter == "influx-file" ? [var.reporter] : []
        labels   = ["upload_metrics"]

        content {
          // Retry to workaround inconsistent InfluxDB upload failures.
          // Conservative settings to avoid thundering-herd when the InfluxDB is under load.
          restart {
            attempts = 3
            interval = "30m"
            delay    = "45s"
            mode     = "fail"
          }

          lifecycle {
            hook = "poststop"
          }

          env {
            WT_METRICS_DIR       = "${NOMAD_ALLOC_DIR}/data/telegraf/metrics"
            RUN_ID               = "${var.run_id != null ? var.run_id : ""}"
            RUN_SUMMARY_PATH     = "${NOMAD_ALLOC_DIR}/run_summary.jsonl"
            INFLUX_HOST          = "https://ifdb.holochain.org"
            INFLUX_BUCKET        = "windtunnel"
            INFLUX_TOKEN         = secret.job_secrets.INFLUX_TOKEN
          }

          driver = "raw_exec"

          artifact {
            source = "https://raw.githubusercontent.com/holochain/wind-tunnel/${var.wind_tunnel_git_ref}/nomad/scripts/upload_metrics.sh"
          }

          config {
            command = "bash"
            args    = ["${NOMAD_TASK_DIR}/upload_metrics.sh"]
          }
        }
      }
    }
  }
}
