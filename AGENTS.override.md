# Agent Override Rules

## Screenshot Path Memory
- When the user says to check `current.png`, resolve it from:
  - `/mnt/c/Users/tende/Pictures/Screenshots/current.png`
- If `current.png` is not found at that exact path, search within:
  - `/mnt/c/Users/tende/Pictures/Screenshots/`

## Legacy Removal Rule
- When the user requests legacy/compat cleanup, do not wait for repeated instructions.
- In one pass, scan `src`, `assets`, docs, and prompts for legacy command strings and compatibility code paths.
- Remove legacy command handlers, compatibility parsers, aliases, fallback paths, and stale doc/UI text together.
- After edits, run verification and report scan results (0 remaining matches) with test output.
