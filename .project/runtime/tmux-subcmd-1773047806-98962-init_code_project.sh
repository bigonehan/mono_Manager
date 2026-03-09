#!/usr/bin/env bash
set +e
cd '/home/tree/project/rust-orc'
echo "[orc-worker] start: 'init_code_project'"
echo "[orc-worker] cwd: /home/tree/project/rust-orc"
'/home/tree/project/rust-orc/target/debug/orc' 'init_code_project' > >(tee '.project/runtime/tmux-subcmd-1773047806-98962-init_code_project.stdout.log') 2> >(tee '.project/runtime/tmux-subcmd-1773047806-98962-init_code_project.stderr.log' >&2)
status=$?
printf "%s" "$status" > '.project/runtime/tmux-subcmd-1773047806-98962-init_code_project.code'
