#!/usr/bin/env bash
cd '/home/tree/project/rust-orc/.tmp_tmux_test'
'codex' exec --dangerously-bypass-approvals-and-sandbox "$(cat '/home/tree/project/rust-orc/.tmp_tmux_test/.project/runtime/tmux-llm-1772648077_143286.prompt.txt')" > '/home/tree/project/rust-orc/.tmp_tmux_test/.project/runtime/tmux-llm-1772648077_143286.stdout.log' 2> '/home/tree/project/rust-orc/.tmp_tmux_test/.project/runtime/tmux-llm-1772648077_143286.stderr.log'
status=$?
printf "%s" "$status" > '/home/tree/project/rust-orc/.tmp_tmux_test/.project/runtime/tmux-llm-1772648077_143286.code'
