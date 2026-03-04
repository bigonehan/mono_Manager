#!/usr/bin/env bash
cd '/home/tree/project/rust-orc/.tmp_tmux_test2'
'codex' exec --dangerously-bypass-approvals-and-sandbox "$(cat '/home/tree/project/rust-orc/.tmp_tmux_test2/.project/runtime/tmux-llm-1772648591_162414.prompt.txt')" > '/home/tree/project/rust-orc/.tmp_tmux_test2/.project/runtime/tmux-llm-1772648591_162414.stdout.log' 2> '/home/tree/project/rust-orc/.tmp_tmux_test2/.project/runtime/tmux-llm-1772648591_162414.stderr.log'
status=$?
printf "%s" "$status" > '/home/tree/project/rust-orc/.tmp_tmux_test2/.project/runtime/tmux-llm-1772648591_162414.code'
