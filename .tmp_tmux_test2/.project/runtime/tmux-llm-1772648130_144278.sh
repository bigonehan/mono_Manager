#!/usr/bin/env bash
cd '.'
'codex' exec --dangerously-bypass-approvals-and-sandbox "$(cat './.project/runtime/tmux-llm-1772648130_144278.prompt.txt')" > './.project/runtime/tmux-llm-1772648130_144278.stdout.log' 2> './.project/runtime/tmux-llm-1772648130_144278.stderr.log'
status=$?
printf "%s" "$status" > './.project/runtime/tmux-llm-1772648130_144278.code'
