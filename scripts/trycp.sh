#!/usr/bin/env bash

start_trycp() {
  trycp_server --port 9000
}

stop_trycp() {
  pgrep -af 'trycp_server.*--port 9000.*' | awk '{print $1}' | xargs kill
}
