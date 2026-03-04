#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="/home/tree/project/rust-orc/.tmp_tmux_test2"
ASSET_HASH_FILE="/home/tree/project/rust-orc/.tmp_tmux_test2/logs/workspace_bootstrap/assets.sha256"

rg -q '<title>School Landing Page</title>' "$ROOT_DIR/index.html"
rg -q 'id="programs" class="programs"' "$ROOT_DIR/index.html"
rg -q -- '--primary: #0b3d91;' "$ROOT_DIR/styles.css"
rg -q 'Ready for 2026 admissions' "$ROOT_DIR/script.js"
sha256sum "$ROOT_DIR/index.html" "$ROOT_DIR/styles.css" "$ROOT_DIR/script.js" > "$ASSET_HASH_FILE"
printf 'run=ok\n'
