#!/usr/bin/env bash
cd '/home/tree/project/rust-orc/.tmp_tmux_verify'
'codex' exec --dangerously-bypass-approvals-and-sandbox "$(cat '/home/tree/project/rust-orc/.tmp_tmux_verify/.project/runtime/tmux-llm-1772648198_148704.prompt.txt')" > '/home/tree/project/rust-orc/.tmp_tmux_verify/.project/runtime/tmux-llm-1772648198_148704.stdout.log' 2> '/home/tree/project/rust-orc/.tmp_tmux_verify/.project/runtime/tmux-llm-1772648198_148704.stderr.log'
status=$?
printf "%s" "$status" > '/home/tree/project/rust-orc/.tmp_tmux_verify/.project/runtime/tmux-llm-1772648198_148704.code'
