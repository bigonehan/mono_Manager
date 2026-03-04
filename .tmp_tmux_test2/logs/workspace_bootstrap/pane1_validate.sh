#!/usr/bin/env bash
set -euo pipefail
status=0
{
  echo "pane=1 start"
  rg -q '<title>School Landing Page</title>' "/home/tree/project/rust-orc/.tmp_tmux_test2/index.html"
  rg -q 'id="school-title"' "/home/tree/project/rust-orc/.tmp_tmux_test2/index.html"
  rg -q 'id="cta-button"' "/home/tree/project/rust-orc/.tmp_tmux_test2/index.html"
  rg -q 'script src="./script.js"' "/home/tree/project/rust-orc/.tmp_tmux_test2/index.html"
  rg -q 'grid-template-columns' "/home/tree/project/rust-orc/.tmp_tmux_test2/styles.css"
  rg -q 'addEventListener\("click"' "/home/tree/project/rust-orc/.tmp_tmux_test2/script.js"
  echo "pane=1 done"
} >"/home/tree/project/rust-orc/.tmp_tmux_test2/logs/workspace_bootstrap/pane1.log" 2>&1 || status=$?
echo "$status" >"/home/tree/project/rust-orc/.tmp_tmux_test2/logs/workspace_bootstrap/pane1.exit"
exit "$status"
