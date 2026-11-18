#!/usr/bin/env bash
set -euo pipefail

PORT="${DHUN_LOCAL_PORT:-47821}"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/dhun/helper"
mkdir -p "$DATA_DIR"
PID_FILE="$DATA_DIR/helper.pid"
LOG_FILE="$DATA_DIR/helper.log"

BIN_PATH="${DHUN_HELPER_BIN:-$(command -v dhun-local-helper || echo './dhun-local-helper')}"
if [ ! -x "$BIN_PATH" ]; then
  echo "Error: dhun-local-helper binary not found or not executable at $BIN_PATH" >&2
  exit 1
fi

export DHUN_LOCAL_PORT="$PORT"
nohup "$BIN_PATH" >> "$LOG_FILE" 2>&1 &
HELPER_PID=$!
echo "$HELPER_PID" > "$PID_FILE"

sleep 2
if curl -fsS "http://127.0.0.1:$PORT/api/local/health" > /dev/null 2>&1; then
  echo "DHUN Local Helper running (PID: $HELPER_PID) on http://127.0.0.1:$PORT"
else
  echo "DHUN Local Helper started (PID: $HELPER_PID); inspect $LOG_FILE if health check failed."
fi
