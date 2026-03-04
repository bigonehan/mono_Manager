#!/usr/bin/env bash
cd '/home/tree/project/rust-orc/.tmp_tmux_flow2'
'codex' exec --dangerously-bypass-approvals-and-sandbox "$(cat '/home/tree/project/rust-orc/.tmp_tmux_flow2/.project/runtime/tmux-llm-1772648260_154137.prompt.txt')" > '/home/tree/project/rust-orc/.tmp_tmux_flow2/.project/runtime/tmux-llm-1772648260_154137.stdout.log' 2> '/home/tree/project/rust-orc/.tmp_tmux_flow2/.project/runtime/tmux-llm-1772648260_154137.stderr.log'
status=$?
printf "%s" "$status" > '/home/tree/project/rust-orc/.tmp_tmux_flow2/.project/runtime/tmux-llm-1772648260_154137.code'
