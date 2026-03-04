#!/usr/bin/env bash
cd '/home/tree/project/rust-orc/.tmp_orclive'
'codex' exec --dangerously-bypass-approvals-and-sandbox "$(cat '/home/tree/project/rust-orc/.tmp_orclive/.project/runtime/tmux-llm-1772648331_160017.prompt.txt')" > '/home/tree/project/rust-orc/.tmp_orclive/.project/runtime/tmux-llm-1772648331_160017.stdout.log' 2> '/home/tree/project/rust-orc/.tmp_orclive/.project/runtime/tmux-llm-1772648331_160017.stderr.log'
status=$?
printf "%s" "$status" > '/home/tree/project/rust-orc/.tmp_orclive/.project/runtime/tmux-llm-1772648331_160017.code'
