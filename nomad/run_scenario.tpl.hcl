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

job "{{ (ds "vars").scenario_name }}" {
  type        = "batch"
  all_at_once = true // Try to run all groups at once
  node_pool = "all" // Run on all node pools

  constraint {
    distinct_hosts = true // Don't run multiple instances on the same client at once
  }

  constraint {
    distinct_property = "${attr.unique.hostname}"
  }

  dynamic "group" {
    for_each = {{ index (ds "vars") "behaviours" | default (coll.Slice "") | toJSON }}
    labels   = ["{{ (ds "vars").scenario_name }}-${group.key}-${group.value}"]

    content {
      restart {
        interval         = "30m"
        attempts         = 5
        delay            = "120s"
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
            source = "https://raw.githubusercontent.com/holochain/wind-tunnel/refs/heads/main/telegraf/telegraf.host.conf"
          }

          config {
            command = "telegraf"
            args    = []
          }
        }
      }

      dynamic "task" {
        // Only download the holochain binary if `var.holochain_bin_url` is set.
        for_each = var.holochain_bin_url != null ? [var.holochain_bin_url] : []
        labels   = ["download_holochain_bin"]

        content {
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

      task "run_scenario" {
        driver = "raw_exec"

        dynamic "artifact" {
          // Download the scenario from the URL if it is not a valid local path.
          for_each = fileexists(abspath(var.scenario_url)) ? [] : [var.scenario_url]

          content {
            source = var.scenario_url
          }
        }

        env {
          RUST_LOG          = "info"
          HOME              = "${NOMAD_TASK_DIR}"
          WT_METRICS_DIR    = "${NOMAD_ALLOC_DIR}/data/telegraf/metrics"
          MIN_AGENTS        = "{{ mul (index (ds "vars") "agents_per_node" | default 1) (len (index (ds "vars") "behaviours" | default (coll.Slice "") )) }}"
          RUN_SUMMARY_PATH  = "${NOMAD_ALLOC_DIR}/run_summary.jsonl"
          WT_HOLOCHAIN_PATH = var.holochain_bin_url == null ? "holochain" : "${NOMAD_ALLOC_DIR}/holochain"
        }

        config {
          // If `var.scenario_url` is a valid local path then run that. Otherwise run the scenario downloaded by the `artifact` block.
          command = fileexists(abspath(var.scenario_url)) ? abspath(var.scenario_url) : "${NOMAD_TASK_DIR}/bin/{{ (ds "vars").scenario_name }}"
          // The `compact` function removes empty strings and `null` items from the list.
          args = compact([
            "--duration=${var.duration}",
            "--reporter=${var.reporter}",
            group.value != "" ? "--behaviour=${group.value}:1" : null,
            var.run_id != null ? "--run-id=${var.run_id}" : null,
            "--agents={{ index (ds "vars") "agents_per_node" | default 1 }}",
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
          lifecycle {
            hook = "poststop"
          }

          env {
            WT_METRICS_DIR       = "${NOMAD_ALLOC_DIR}/data/telegraf/metrics"
            RUN_ID               = "${var.run_id != null ? var.run_id : ""}"
            RUN_SUMMARY_PATH     = "${NOMAD_ALLOC_DIR}/run_summary.jsonl"
            INFLUX_HOST          = "https://ifdb.holochain.org"
            INFLUX_BUCKET        = "windtunnel"
          }

          template {
            destination = "${NOMAD_SECRETS_DIR}/secrets.env"
            env         = true
            data        = <<EOT
            INFLUX_TOKEN={{ "{{ with nomadVar \"nomad/jobs\" }}{{ .INFLUX_TOKEN }}{{ end }}" }}
            EOT
          }

          driver = "raw_exec"

          artifact {
            source = "https://raw.githubusercontent.com/holochain/wind-tunnel/refs/heads/main/nomad/scripts/upload_metrics.sh"
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
