#!/usr/bin/env bash
cd '/home/tree/project/rust-orc/.tmp_tmux_test'
'codex' exec --dangerously-bypass-approvals-and-sandbox "$(cat '/home/tree/project/rust-orc/.tmp_tmux_test/.project/runtime/tmux-llm-1772648377_159055.prompt.txt')" > '/home/tree/project/rust-orc/.tmp_tmux_test/.project/runtime/tmux-llm-1772648377_159055.stdout.log' 2> '/home/tree/project/rust-orc/.tmp_tmux_test/.project/runtime/tmux-llm-1772648377_159055.stderr.log'
status=$?
printf "%s" "$status" > '/home/tree/project/rust-orc/.tmp_tmux_test/.project/runtime/tmux-llm-1772648377_159055.code'
