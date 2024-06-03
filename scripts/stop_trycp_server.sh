#!/usr/bin/env bash

pgrep -af 'trycp_server.*--port 9000.*' | awk '{print $1}' | xargs kill
