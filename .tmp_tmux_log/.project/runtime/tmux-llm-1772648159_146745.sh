#!/usr/bin/env bash
cd '/home/tree/project/rust-orc/.tmp_tmux_log'
'codex' exec --dangerously-bypass-approvals-and-sandbox "$(cat '/home/tree/project/rust-orc/.tmp_tmux_log/.project/runtime/tmux-llm-1772648159_146745.prompt.txt')" > '/home/tree/project/rust-orc/.tmp_tmux_log/.project/runtime/tmux-llm-1772648159_146745.stdout.log' 2> '/home/tree/project/rust-orc/.tmp_tmux_log/.project/runtime/tmux-llm-1772648159_146745.stderr.log'
status=$?
printf "%s" "$status" > '/home/tree/project/rust-orc/.tmp_tmux_log/.project/runtime/tmux-llm-1772648159_146745.code'
