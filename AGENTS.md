# Project Agent Rules

## CLI Rename Sync Rule
- If any CLI command name, subcommand alias, or file name shown by CLI help is changed, update all related help/documentation in the same change.
- Minimum sync targets:
  - `src/cli/mod.rs` usage/help output
  - `README.md` command list/examples
  - Any command examples in repo docs
- Do not finish the task until `cargo run --bin orc -- --help` matches the documented command names.

## Completion Log Auto-Write Rule
- Any task that adds or changes behavior must update `./.agents/log.md` in the same turn.
- Writing `./.agents/log.md` is pre-authorized in this repository workflow; do not ask the user for confirmation before applying that file update.
- Apply this as the default for future tasks unless the user explicitly disables it.

## Notification Output Rule
- Do not include completion-notification execution details in the final user-facing summary.
- If a notification command is run by policy, keep it silent in the response unless the user explicitly asks for notification logs.

## Auto Install Rule
- When a feature addition or improvement task is completed, run `cargo install --path /home/tree/project/rust-orc` automatically before finalizing the task.
