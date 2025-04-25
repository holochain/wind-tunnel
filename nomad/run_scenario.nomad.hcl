variable "scenario-name" {
  type        = string
  description = "The name of the scenario to run"
}

variable "connection-string" {
  type        = string
  description = "The URL to be used to connect to the service being tested"
  default     = "ws://localhost:8888"
}

variable "duration" {
  type        = number
  description = "The maximum duration of the scenario run"
  default     = null
}

variable "reporter" {
  type        = string
  description = "The method used to report the logs"
  default     = "influx-file"
}

variable "behaviours" {
  type        = list(string)
  description = "Custom behaviours defined and used by the scenarios"
  default     = [""]
}

variable "scenario-url" {
  type        = string
  description = "The URL to the binary or bundle of the scenario under test, this will be downloaded if it is not a local path"
}

variable "run-id" {
  type        = string
  description = "The ID of this run to distinguish it from other runs"
  default     = null
}

job "run_scenario" {
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
      task "start_holochain" {
        lifecycle {
          hook    = "prestart"
          sidecar = true
        }

        env {
          RUST_LOG = "info"
        }

        driver = "raw_exec"
        config {
          command = "bash"
          args    = ["-c", "hc s clean && echo 1234 | hc s --piped create --in-process-lair network --bootstrap=https://bootstrap.holo.host webrtc wss://sbd.holo.host && echo 1234 | hc s --piped -f 8888 run"]
        }

        resources {
          memory = 2048
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
          RUST_LOG       = "info"
          HOME           = "${NOMAD_TASK_DIR}"
          WT_METRICS_DIR = "${NOMAD_ALLOC_DIR}/data/telegraf/metrics"
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
            TELEGRAF_CONFIG_PATH = "${NOMAD_TASK_DIR}/runner-telegraf.conf"
            WT_METRICS_DIR       = "${NOMAD_ALLOC_DIR}/data/telegraf/metrics"
          }

          template {
            destination = "${NOMAD_SECRETS_DIR}/secrets.env"
            env         = true
            data        = <<EOT
          INFLUX_TOKEN={{ with nomadVar "nomad/jobs/run_scenario" }}{{ .INFLUX_TOKEN }}{{ end }}
          EOT
          }

          driver = "raw_exec"

          artifact {
            source = "https://raw.githubusercontent.com/holochain/wind-tunnel/refs/heads/main/telegraf/runner-telegraf.conf"
          }

          config {
            command = "telegraf"
            args    = ["--once"]
          }
        }
      }
    }
  }
}
