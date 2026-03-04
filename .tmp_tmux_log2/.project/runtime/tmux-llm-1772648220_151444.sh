#!/usr/bin/env bash
cd '/home/tree/project/rust-orc/.tmp_tmux_log2'
'codex' exec --dangerously-bypass-approvals-and-sandbox "$(cat '/home/tree/project/rust-orc/.tmp_tmux_log2/.project/runtime/tmux-llm-1772648220_151444.prompt.txt')" > '/home/tree/project/rust-orc/.tmp_tmux_log2/.project/runtime/tmux-llm-1772648220_151444.stdout.log' 2> '/home/tree/project/rust-orc/.tmp_tmux_log2/.project/runtime/tmux-llm-1772648220_151444.stderr.log'
status=$?
printf "%s" "$status" > '/home/tree/project/rust-orc/.tmp_tmux_log2/.project/runtime/tmux-llm-1772648220_151444.code'
