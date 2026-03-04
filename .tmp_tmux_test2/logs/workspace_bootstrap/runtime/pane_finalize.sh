#!/usr/bin/env bash
set -euo pipefail
ASSET_HASH_FILE="/home/tree/project/rust-orc/.tmp_tmux_test2/logs/workspace_bootstrap/assets.sha256"

test -f "$ASSET_HASH_FILE"
rg -q 'index.html' "$ASSET_HASH_FILE"
rg -q 'styles.css' "$ASSET_HASH_FILE"
rg -q 'script.js' "$ASSET_HASH_FILE"
printf 'finalize=ok\n'
