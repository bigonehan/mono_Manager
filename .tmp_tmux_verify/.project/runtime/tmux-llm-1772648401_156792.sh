#!/usr/bin/env bash
cd '/home/tree/project/rust-orc/.tmp_tmux_verify'
'codex' exec --dangerously-bypass-approvals-and-sandbox "$(cat '/home/tree/project/rust-orc/.tmp_tmux_verify/.project/runtime/tmux-llm-1772648401_156792.prompt.txt')" > '/home/tree/project/rust-orc/.tmp_tmux_verify/.project/runtime/tmux-llm-1772648401_156792.stdout.log' 2> '/home/tree/project/rust-orc/.tmp_tmux_verify/.project/runtime/tmux-llm-1772648401_156792.stderr.log'
status=$?
printf "%s" "$status" > '/home/tree/project/rust-orc/.tmp_tmux_verify/.project/runtime/tmux-llm-1772648401_156792.code'
