#!/usr/bin/env bash
cd '/home/tree/project/rust-orc/.tmp_tmux_test'
'codex' exec --dangerously-bypass-approvals-and-sandbox "$(cat '/home/tree/project/rust-orc/.tmp_tmux_test/.project/runtime/tmux-llm-1772648151_146028.prompt.txt')" > '/home/tree/project/rust-orc/.tmp_tmux_test/.project/runtime/tmux-llm-1772648151_146028.stdout.log' 2> '/home/tree/project/rust-orc/.tmp_tmux_test/.project/runtime/tmux-llm-1772648151_146028.stderr.log'
status=$?
printf "%s" "$status" > '/home/tree/project/rust-orc/.tmp_tmux_test/.project/runtime/tmux-llm-1772648151_146028.code'
