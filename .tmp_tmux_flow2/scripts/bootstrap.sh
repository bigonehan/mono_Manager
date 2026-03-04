#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SESSION_NAME="tmux_flow_verification"
WINDOW_NAME="bootstrap"
HELLO_LOG="${PROJECT_ROOT}/.project/hello_world.log"
STATE_LOG="${PROJECT_ROOT}/.project/bootstrap_state.log"

if ! command -v tmux >/dev/null 2>&1; then
  echo "error: tmux is required" >&2
  exit 1
fi

if tmux has-session -t "${SESSION_NAME}" 2>/dev/null; then
  :
else
  tmux new-session -d -s "${SESSION_NAME}" -n "${WINDOW_NAME}" -c "${PROJECT_ROOT}"
fi

if ! tmux list-windows -t "${SESSION_NAME}" -F '#W' | grep -x "${WINDOW_NAME}" >/dev/null 2>&1; then
  tmux new-window -d -t "${SESSION_NAME}" -n "${WINDOW_NAME}" -c "${PROJECT_ROOT}"
fi

TARGET_WINDOW="${SESSION_NAME}:${WINDOW_NAME}"
PANE_COUNT="$(tmux list-panes -t "${TARGET_WINDOW}" | wc -l | tr -d ' ')"
if [ "${PANE_COUNT}" -lt 2 ]; then
  tmux split-window -h -t "${TARGET_WINDOW}" -c "${PROJECT_ROOT}"
fi

tmux send-keys -t "${TARGET_WINDOW}.0" "cd \"${PROJECT_ROOT}\" && printf 'hello world\n' > \"${HELLO_LOG}\"" C-m
tmux send-keys -t "${TARGET_WINDOW}.1" "cd \"${PROJECT_ROOT}\" && printf 'bootstrap_verified\n' > \"${STATE_LOG}\"" C-m

for _ in 1 2 3 4 5; do
  if [ -f "${HELLO_LOG}" ] && [ -f "${STATE_LOG}" ]; then
    break
  fi
  sleep 0.1
done

if [ ! -f "${HELLO_LOG}" ] || [ ! -f "${STATE_LOG}" ]; then
  echo "error: bootstrap verification failed" >&2
  exit 1
fi

echo "hello world"
echo "session=${SESSION_NAME} window=${WINDOW_NAME} state=bootstrap_verified"
