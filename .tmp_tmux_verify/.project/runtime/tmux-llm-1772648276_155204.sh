#!/usr/bin/env bash
cd '/home/tree/project/rust-orc/.tmp_tmux_verify'
'codex' exec --dangerously-bypass-approvals-and-sandbox "$(cat '/home/tree/project/rust-orc/.tmp_tmux_verify/.project/runtime/tmux-llm-1772648276_155204.prompt.txt')" > '/home/tree/project/rust-orc/.tmp_tmux_verify/.project/runtime/tmux-llm-1772648276_155204.stdout.log' 2> '/home/tree/project/rust-orc/.tmp_tmux_verify/.project/runtime/tmux-llm-1772648276_155204.stderr.log'
status=$?
printf "%s" "$status" > '/home/tree/project/rust-orc/.tmp_tmux_verify/.project/runtime/tmux-llm-1772648276_155204.code'
