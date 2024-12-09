variable "scenario-name" {
  type = string
}

variable "connection-string" {
  type = string
  default = "ws://localhost:8888"
}

variable "agents" {
  type = number
  default = null
}

variable "duration" {
  type = number
  default = null
}

variable "behaviours" {
  type = map(string)
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
      env {
        RUST_LOG = "info"
      }
      config {
        command = abspath("result/bin/${var.scenario-name}")
        // The `compact` function removes empty strings and `null` items from the list.
        args = concat(compact([
          "--connection-string=${var.connection-string}",
          var.agents != null ? "--agents=${var.agents}" : null,
          var.duration != null ? "--duration=${var.duration}" : null,
          "--no-progress"
        ]), [
          for k, v in var.behaviours : "--behaviour=${k}:${v}"
        ])
      }
    }
  }
}
