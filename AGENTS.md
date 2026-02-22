# Project Agent Rules

## CLI Rename Sync Rule
- If any CLI command name, subcommand alias, or file name shown by CLI help is changed, update all related help/documentation in the same change.
- Minimum sync targets:
  - `src/cli/mod.rs` usage/help output
  - `README.md` command list/examples
  - Any command examples in repo docs
- Do not finish the task until `cargo run --bin orc -- --help` matches the documented command names.
