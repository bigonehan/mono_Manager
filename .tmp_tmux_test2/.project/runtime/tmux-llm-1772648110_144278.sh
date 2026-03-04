#!/usr/bin/env bash
cd '/home/tree/project/rust-orc/.tmp_tmux_test2'
'codex' exec --dangerously-bypass-approvals-and-sandbox "$(cat '/home/tree/project/rust-orc/.tmp_tmux_test2/.project/runtime/tmux-llm-1772648110_144278.prompt.txt')" > '/home/tree/project/rust-orc/.tmp_tmux_test2/.project/runtime/tmux-llm-1772648110_144278.stdout.log' 2> '/home/tree/project/rust-orc/.tmp_tmux_test2/.project/runtime/tmux-llm-1772648110_144278.stderr.log'
status=$?
printf "%s" "$status" > '/home/tree/project/rust-orc/.tmp_tmux_test2/.project/runtime/tmux-llm-1772648110_144278.code'
