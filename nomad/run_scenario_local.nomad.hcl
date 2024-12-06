variable "scenario_name" {
  type = string
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
        command = abspath("result/bin/${var.scenario_name}")
        args = [
          "--connection-string", "ws://localhost:8888",
          "--agents", "2",
          "--behaviour", "minimal:1",
          "--behaviour", "large:1",
          "--duration", "5",
          "--no-progress"
        ]
      }
    }
  }
}
