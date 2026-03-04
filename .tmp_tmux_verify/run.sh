#!/usr/bin/env bash
set -euo pipefail

SESSION_NAME="${1:-verify_worker_pane_split_and_output}"

if ! command -v tmux >/dev/null 2>&1; then
  echo "tmux is required"
  exit 1
fi

if tmux has-session -t "$SESSION_NAME" 2>/dev/null; then
  tmux kill-session -t "$SESSION_NAME"
fi

tmux new-session -d -s "$SESSION_NAME" "bash -lc 'echo hello world from worker pane; sleep 1'"
tmux split-window -h -t "$SESSION_NAME":0 "bash -lc 'echo hello world from main pane; sleep 1'"
tmux select-layout -t "$SESSION_NAME":0 even-horizontal

WORKER_OUTPUT=""
MAIN_OUTPUT=""

for _ in {1..20}; do
  WORKER_OUTPUT="$(tmux capture-pane -p -t "$SESSION_NAME":0.0)"
  MAIN_OUTPUT="$(tmux capture-pane -p -t "$SESSION_NAME":0.1)"
  if [[ "$WORKER_OUTPUT" == *"hello world"* ]] && [[ "$MAIN_OUTPUT" == *"hello world"* ]]; then
    break
  fi
  sleep 0.1
done

if [[ "$WORKER_OUTPUT" == *"hello world"* ]] && [[ "$MAIN_OUTPUT" == *"hello world"* ]]; then
  echo "hello world"
  echo "pane split and output verified"
else
  echo "verification failed"
  tmux kill-session -t "$SESSION_NAME"
  exit 1
fi

tmux kill-session -t "$SESSION_NAME"
