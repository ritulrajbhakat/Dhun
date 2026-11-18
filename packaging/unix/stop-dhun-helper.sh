#!/usr/bin/env bash
set -euo pipefail

DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/dhun/helper"
PID_FILE="$DATA_DIR/helper.pid"

if [ -f "$PID_FILE" ]; then
  PID=$(cat "$PID_FILE")
  if kill -0 "$PID" 2>/dev/null; then
    kill -TERM "$PID"
    echo "Stopped DHUN Local Helper (PID: $PID)."
  fi
  rm -f "$PID_FILE"
else
  pkill -TERM -f dhun-local-helper 2>/dev/null && echo "Stopped dhun-local-helper." || echo "DHUN Local Helper is not running."
fi
