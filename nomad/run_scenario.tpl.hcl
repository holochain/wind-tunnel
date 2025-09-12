variable "scenario-name" {
  type        = string
  description = "The name of the scenario to run"
  default = {{ (ds "vars").scenario_name | quote }}
}

variable "connection-string" {
  type        = string
  description = "The URL to be used to connect to the service being tested"
  {{/* Default: read `connection_string` from the JSON data source `vars`, or set to `"ws://localhost:8888"` if not provided.*/}}
  default     = {{ index (ds "vars") "connection_string" | default "ws://localhost:8888" | quote }}
}

variable "duration" {
  type        = number
  description = "The maximum duration of the scenario run"
  {{/* Default: read `duration` from the JSON data source `vars`, or set to `null` if not provided.*/}}
  default     = {{ with index (ds "vars") "duration" }}{{ . | quote }}{{ else }}null{{ end }}
}

variable "reporter" {
  type        = string
  description = "The method used to report the logs"
  {{/* Default: read `reporter` from the JSON data source `vars`, or set to `"influx-file"` if not provided.*/}}
  default     = {{ index (ds "vars") "reporter" | default "influx-file" | quote }}
}

variable "behaviours" {
  type        = list(string)
  description = "Custom behaviours defined and used by the scenarios"
  {{/* Default: read `behaviours` from the JSON data source `vars`, or set to `[]` if not provided.*/}}
  default = {{ index (ds "vars") "behaviours" | default (coll.Slice "") | toJSON }}
}

variable "scenario-url" {
  type        = string
  description = "The URL to the binary or bundle of the scenario under test, this will be downloaded if it is not a local path" 
  {{/* Default: read `scenario_url` from the JSON data source `vars`, or set to `""` if not provided.*/}}
  default = {{ index (ds "vars") "scenario_url" | default "" | quote }}
}

variable "run-id" {
  type        = string
  description = "The ID of this run to distinguish it from other runs"
  {{/* Default: read `run_id` from the JSON data source `vars`, or set to `null` if not provided. */}}
  default     = {{ with index (ds "vars") "run_id" }}{{ . | quote }}{{ else }}null{{ end }}
}

variable "agents-per-node" {
  type        = number
  description = "The number of agents to run per client node that is running the scenario"
  {{/* Default: read `agents_per_node` from the JSON data source `vars`, or set to `1` if not provided. */}}
  default     = {{ index (ds "vars") "agents_per_node" | default 1 }}
}

variable "min-agents" {
  type        = number
  description = "The minimum number of agents to wait for in the scenario"
  {{/* Default: read `min_agents` from the JSON data source `vars`, or set to `2` if not provided. */}}
  default     = {{ index (ds "vars") "min_agents" | default 2 }}
}

job "{{ (ds "vars").scenario_name }}" {
  type        = "batch"
  all_at_once = true // Try to run all groups at once

  constraint {
    distinct_hosts = true // Don't run multiple instances on the same client at once
  }

  constraint {
    distinct_property = "${attr.unique.hostname}"
  }

  dynamic "group" {
    for_each = var.behaviours
    labels   = ["${var.scenario-name}-${group.key}-${group.value}"]

    content {
      restart {
        interval         = "30m"
        attempts         = 5
        delay            = "120s"
      }

      task "start_holochain" {
        lifecycle {
          hook    = "prestart"
          sidecar = true
        }

        env {
          RUST_LOG = "info"
          HOLOCHAIN_INFLUXIVE_FILE = "${var.reporter == "influx-file" ? "${NOMAD_ALLOC_DIR}/data/telegraf/metrics/holochain_${group.value}.influx" : ""}"
        }

        driver = "raw_exec"
        config {
          command = "bash"
          args    = ["-c", "mkdir -p ${NOMAD_ALLOC_DIR}/data/telegraf/metrics/ && hc s clean && echo 1234 | hc s --piped create --in-process-lair network --bootstrap=https://bootstrap.holo.host webrtc wss://sbd.holo.host && echo 1234 | hc s --piped -f 8888 run"]
        }

        resources {
          cores = 2
          memory = 2048
        }
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

      task "wait_for_holochain" {
        lifecycle {
          hook = "prestart"
        }

        driver = "raw_exec"
        config {
          command = "bash"
          args    = ["-c", "echo -n 'Waiting for Holochain to start'; until hc s call --running=8888 dump-conductor-state 2>/dev/null >/dev/null; do echo '.'; sleep 0.5; done; echo 'done'; sleep 1"]
        }
      }

      task "run_scenario" {
        driver = "raw_exec"

        dynamic "artifact" {
          // Download the scenario from the URL if it is not a valid local path.
          for_each = fileexists(abspath(var.scenario-url)) ? [] : [var.scenario-url]

          content {
            source = var.scenario-url
          }
        }

        env {
          RUST_LOG          = "info"
          HOME              = "${NOMAD_TASK_DIR}"
          WT_METRICS_DIR    = "${NOMAD_ALLOC_DIR}/data/telegraf/metrics"
          MIN_AGENTS        = "${var.min-agents}"
          RUN_SUMMARY_PATH  = "${NOMAD_ALLOC_DIR}/run_summary.jsonl"
        }

        config {
          // If `var.scenario-url` is a valid local path then run that. Otherwise run the scenario downloaded by the `artifact` block.
          command = fileexists(abspath(var.scenario-url)) ? abspath(var.scenario-url) : var.scenario-name
          // The `compact` function removes empty strings and `null` items from the list.
          args = compact([
            "--connection-string=${var.connection-string}",
            var.duration != null ? "--duration=${var.duration}" : null,
            var.reporter != null ? "--reporter=${var.reporter}" : null,
            group.value != "" ? "--behaviour=${group.value}:1" : null,
            var.run-id != null ? "--run-id=${var.run-id}" : null,
            "--agents=${var.agents-per-node}",
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
            RUN_ID               = "${var.run-id != null ? var.run-id : ""}"
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
            source = "https://raw.githubusercontent.com/holochain/wind-tunnel/refs/heads/main/nomad/upload_metrics.sh"
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
