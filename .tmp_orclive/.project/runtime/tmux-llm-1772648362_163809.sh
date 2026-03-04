#!/usr/bin/env bash
cd '/home/tree/project/rust-orc/.tmp_orclive'
echo "[orc-llm] start: 'codex' exec"
echo "[orc-llm] cwd: /home/tree/project/rust-orc/.tmp_orclive"
'codex' exec --dangerously-bypass-approvals-and-sandbox "$(cat '/home/tree/project/rust-orc/.tmp_orclive/.project/runtime/tmux-llm-1772648362_163809.prompt.txt')" > >(tee '/home/tree/project/rust-orc/.tmp_orclive/.project/runtime/tmux-llm-1772648362_163809.stdout.log') 2> >(tee '/home/tree/project/rust-orc/.tmp_orclive/.project/runtime/tmux-llm-1772648362_163809.stderr.log' >&2)
status=$?
printf "%s" "$status" > '/home/tree/project/rust-orc/.tmp_orclive/.project/runtime/tmux-llm-1772648362_163809.code'
