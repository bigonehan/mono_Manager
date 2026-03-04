#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="/home/tree/project/rust-orc/.tmp_tmux_test2"

test -d "$ROOT_DIR"
test -f "$ROOT_DIR/index.html"
test -f "$ROOT_DIR/styles.css"
test -f "$ROOT_DIR/script.js"
printf 'prepare=ok\n'
