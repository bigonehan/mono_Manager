#!/usr/bin/env bash
cd '.'
'codex' exec --dangerously-bypass-approvals-and-sandbox "$(cat './.project/runtime/tmux-llm-1772648098_143286.prompt.txt')" > './.project/runtime/tmux-llm-1772648098_143286.stdout.log' 2> './.project/runtime/tmux-llm-1772648098_143286.stderr.log'
status=$?
printf "%s" "$status" > './.project/runtime/tmux-llm-1772648098_143286.code'
