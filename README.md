# rust-orc

## Introduction
`rust-orc` is a Rust-based CLI for running the ORC workflow, including job scaffolding, implementation stages, chat utilities, worker orchestration, and web or UI entrypoints.

## CLI
- Run the main CLI:
  - `cargo run --bin orc -- <command>`
- Run the helper CLI:
  - `cargo run --bin rc -- <command>`
- Common commands:
  - `orc help`
  - `orc cli_help`
  - `orc create_job_md`
  - `orc add_orc_drafts`
  - `orc impl_orc_code`
  - `orc check_orc_code`
  - `orc open-ui [-w|--web|-b|--build]`
  - `orc serve-web-api [--addr <host:port>]`
  - `orc worker-create [name]`
  - `orc worker-send <worker_ref|pane_id> <msg...>|--stdin [enter|enter-exit|raw|display]`
  - `orc worker-wait <worker_ref|pane_id> <pattern> [timeout_ms] [lines]`
  - `orc worker-close <worker_ref|pane_id>`
