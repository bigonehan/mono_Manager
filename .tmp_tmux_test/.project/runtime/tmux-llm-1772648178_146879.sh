#!/usr/bin/env bash
cd '/home/tree/project/rust-orc/.tmp_tmux_test'
'codex' exec --dangerously-bypass-approvals-and-sandbox "$(cat '/home/tree/project/rust-orc/.tmp_tmux_test/.project/runtime/tmux-llm-1772648178_146879.prompt.txt')" > '/home/tree/project/rust-orc/.tmp_tmux_test/.project/runtime/tmux-llm-1772648178_146879.stdout.log' 2> '/home/tree/project/rust-orc/.tmp_tmux_test/.project/runtime/tmux-llm-1772648178_146879.stderr.log'
status=$?
printf "%s" "$status" > '/home/tree/project/rust-orc/.tmp_tmux_test/.project/runtime/tmux-llm-1772648178_146879.code'
