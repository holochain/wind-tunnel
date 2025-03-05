variable "scenario-name" {
  type = string
  description = "The name of the scenario to run"
}

variable "connection-string" {
  type = string
  description = "The URL to be used to connect to the service being tested"
  default = "ws://localhost:8888"
}

variable "agents" {
  type = number
  description = "The number of agents to create"
  default = null
}

variable "duration" {
  type = number
  description = "The maximum duration of the scenario run"
  default = null
}

variable "reporter" {
  type = string
  description = "The method used to report the logs"
  default = "influx-file"
}

variable "behaviours" {
  type = map(string)
  description = "Custom behaviours defined and used by the scenarios"
  default = {}
}

job "run_scenario" {
  type = "batch"

  group "scenario_runnner" {
    task "start_holochain" {
      lifecycle {
        hook = "prestart"
        sidecar = true
      }

      driver = "raw_exec"
      config {
        command = "bash"
        args = ["-c", "hc s clean && echo 1234 | hc s --piped create && echo 1234 | hc s --piped -f 8888 run"]
      }
    }

    task "run_scenario" {
      driver = "raw_exec"

      artifact {
          source = "https://github.com/holochain/wind-tunnel/releases/download/bins-for-nomad/${var.scenario-name}"
      }

      env {
        RUST_LOG = "info"
        HOME = "${NOMAD_TASK_DIR}"
      }

      config {
        command = var.scenario-name
        // The `compact` function removes empty strings and `null` items from the list.
        args = concat(compact([
          "--connection-string=${var.connection-string}",
          var.agents != null ? "--agents=${var.agents}" : null,
          var.duration != null ? "--duration=${var.duration}" : null,
          var.reporter != null ? "--reporter=${var.reporter}" : null,
          "--no-progress"
        ]), [
          for k, v in var.behaviours : "--behaviour=${k}:${v}"
        ])
      }
    }
  }
}
