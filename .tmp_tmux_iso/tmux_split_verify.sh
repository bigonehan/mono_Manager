#!/usr/bin/env bash
set -euo pipefail

session="tmux_split_verify_$$"

cleanup() {
  tmux has-session -t "$session" 2>/dev/null && tmux kill-session -t "$session" >/dev/null 2>&1 || true
}

trap cleanup EXIT

if ! command -v tmux >/dev/null 2>&1; then
  echo "tmux is not installed"
  exit 1
fi

tmux new-session -d -s "$session"
tmux split-window -t "$session":0

pane_count="$(tmux list-panes -t "$session":0 | wc -l | tr -d ' ')"

if [ "$pane_count" -ne 2 ]; then
  echo "tmux split verification failed: expected 2 panes, got $pane_count"
  exit 1
fi

echo "tmux split verification passed"
