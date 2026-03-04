#!/usr/bin/env bash
cd '/home/tree/project/rust-orc/.tmp_tmux_iso'
'codex' exec --dangerously-bypass-approvals-and-sandbox "$(cat '/home/tree/project/rust-orc/.tmp_tmux_iso/.project/runtime/tmux-llm-1772648206_150073.prompt.txt')" > '/home/tree/project/rust-orc/.tmp_tmux_iso/.project/runtime/tmux-llm-1772648206_150073.stdout.log' 2> '/home/tree/project/rust-orc/.tmp_tmux_iso/.project/runtime/tmux-llm-1772648206_150073.stderr.log'
status=$?
printf "%s" "$status" > '/home/tree/project/rust-orc/.tmp_tmux_iso/.project/runtime/tmux-llm-1772648206_150073.code'
