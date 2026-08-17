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

variable "nodejs_url" {
  type        = string
  description = "URL of the Node.js dist tarball used to run the Peerkit CLI"
  {{- /* Default: read `nodejs_url` from the JSON data source `vars`, or use the official dist. */}}
  default     = {{ index (ds "vars") "nodejs_url" | default "https://nodejs.org/dist/v22.14.0/node-v22.14.0-linux-x64.tar.gz" | quote }}
}

variable "nodejs_sha256" {
  type        = string
  description = "SHA-256 checksum of the Node.js dist tarball"
  {{- /* Default: read `nodejs_sha256` from the JSON data source `vars`, or use the checksum for the official dist default above. */}}
  default     = {{ index (ds "vars") "nodejs_sha256" | default "9d942932535988091034dc94cc5f42b6dc8784d6366df3a36c4c9ccb3996f0c2" | quote }}
}

variable "peerkit_cli_version" {
  type        = string
  description = "The @peerkit/cli npm version to install"
  default     = {{ index (ds "vars") "peerkit_cli_version" | default "0.1.0-alpha.15" | quote }}
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

      task "install_peerkit" {
        restart {
          attempts = 5
          interval = "5m"
          delay    = "10s"
          mode     = "fail"
        }

        lifecycle {
          hook    = "prestart"
          sidecar = false
        }

        driver = "raw_exec"

        env {
          PEERKIT_CLI_VERSION = var.peerkit_cli_version
        }

        artifact {
          source      = var.nodejs_url
          destination = "${NOMAD_ALLOC_DIR}/nodejs_dist"

          options {
            // Download the tarball without unpacking it. Nomad's own
            // decompression refuses archives with more files than the client's
            // `decompression_file_count_limit` (4096 by default) and the
            // Node.js dist tarball exceeds that, so it is unpacked with `tar`
            // in the script below instead.
            archive  = "false"
            checksum = "sha256:${var.nodejs_sha256}"
          }
        }

        template {
          // Unpack Node.js and install the Peerkit CLI.
          // Only bash-level variables are used here; $${} escaping keeps HCL from
          // interpolating them.
          data        = <<-EOF
          #!/usr/bin/env bash
          set -euo pipefail
          tarball="$(find "$NOMAD_ALLOC_DIR/nodejs_dist" -maxdepth 1 -type f -name 'node-*' -print -quit)"
          if [ -z "$tarball" ]; then
            echo "No Node.js tarball found in $NOMAD_ALLOC_DIR/nodejs_dist" >&2
            exit 1
          fi
          rm -rf "$NOMAD_ALLOC_DIR/nodejs"
          mkdir -p "$NOMAD_ALLOC_DIR/nodejs"
          tar -xf "$tarball" -C "$NOMAD_ALLOC_DIR/nodejs"
          node_dir="$(find "$NOMAD_ALLOC_DIR/nodejs" -maxdepth 1 -mindepth 1 -type d -print -quit)"
          if [ -z "$node_dir" ]; then
            echo "Node.js tarball did not contain a top level directory" >&2
            exit 1
          fi
          ln -sfn "$node_dir" "$NOMAD_ALLOC_DIR/node"
          export PATH="$NOMAD_ALLOC_DIR/node/bin:$PATH"
          npm install --global --prefix "$NOMAD_ALLOC_DIR/peerkit" "@peerkit/cli@$PEERKIT_CLI_VERSION"
          EOF
          destination = "${NOMAD_TASK_DIR}/install_peerkit.sh"
          perms       = "755"
        }

        config {
          command = "bash"
          args    = ["${NOMAD_TASK_DIR}/install_peerkit.sh"]
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
          RUST_LOG                = "info"
          HOME                    = "${NOMAD_TASK_DIR}"
          WT_METRICS_DIR          = "${NOMAD_ALLOC_DIR}/data/telegraf/metrics"
          RUN_SUMMARY_PATH        = "${NOMAD_ALLOC_DIR}/run_summary.jsonl"
          WT_PEERKIT_PATH         = "${NOMAD_ALLOC_DIR}/peerkit/bin/peerkit"
          PATH                    = "${NOMAD_ALLOC_DIR}/node/bin:/usr/local/bin:/usr/bin:/bin"
          PEERKIT_RELAY_DIAL_ADDR = secret.job_secrets.PEERKIT_RELAY_DIAL_ADDR
          PEERKIT_NETWORK_ACCESS  = secret.job_secrets.PEERKIT_NETWORK_ACCESS
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
