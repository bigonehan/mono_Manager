#!/usr/bin/env bash
set -euo pipefail
status=0
{
  echo "pane=0 start"
  [[ -f "/home/tree/project/rust-orc/.tmp_tmux_test2/index.html" ]]
  [[ -f "/home/tree/project/rust-orc/.tmp_tmux_test2/styles.css" ]]
  [[ -f "/home/tree/project/rust-orc/.tmp_tmux_test2/script.js" ]]
  echo "pane=0 done"
} >"/home/tree/project/rust-orc/.tmp_tmux_test2/logs/workspace_bootstrap/pane0.log" 2>&1 || status=$?
echo "$status" >"/home/tree/project/rust-orc/.tmp_tmux_test2/logs/workspace_bootstrap/pane0.exit"
exit "$status"
