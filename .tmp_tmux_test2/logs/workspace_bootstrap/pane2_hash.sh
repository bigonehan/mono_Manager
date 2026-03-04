#!/usr/bin/env bash
set -euo pipefail
status=0
{
  echo "pane=2 start"
  sha256sum "/home/tree/project/rust-orc/.tmp_tmux_test2/index.html" "/home/tree/project/rust-orc/.tmp_tmux_test2/styles.css" "/home/tree/project/rust-orc/.tmp_tmux_test2/script.js" >"/home/tree/project/rust-orc/.tmp_tmux_test2/logs/workspace_bootstrap/assets.sha256"
  cat "/home/tree/project/rust-orc/.tmp_tmux_test2/logs/workspace_bootstrap/assets.sha256"
  echo "pane=2 done"
} >"/home/tree/project/rust-orc/.tmp_tmux_test2/logs/workspace_bootstrap/pane2.log" 2>&1 || status=$?
echo "$status" >"/home/tree/project/rust-orc/.tmp_tmux_test2/logs/workspace_bootstrap/pane2.exit"
exit "$status"
