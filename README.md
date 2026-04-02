# rust-orc

## Run
- Build and run with the `orc` binary target:
  - `cargo run --bin orc -- <command>`
- Build and run with the `rc` helper binary target:
  - `cargo run --bin rc -- <command>`

## Main Commands (orc)
- `orc help`
- `orc cli_help`
- `orc init_orc_project [-n <name>] [-p <path>] [-s <spec>] [-d <description>] [-m <message>] [-a]`
- `orc init_code_project [-n <name>] [-p <path>] [-s <spec>] [-d <description>] [-m <message>] [-a]` (alias)
- `orc create_job_md`
- `orc create_code_draft`
- `orc add_code_draft_item [-f] [-m <message>]`
- `orc impl_code_draft`
- `orc check_orc_code`
- `orc check_task`
- `orc check_draft`
- `orc cli_rust_orchestra`
- `orc chat -n <name>`
- `orc chat -n <name> --background`
- `orc chat -n <name> -m <message> [-i <receiver_id>] [--data <data>]`
- `orc chat-wait -n <name> -a <true|false> [-c <count>]`
- `orc open-ui [-w|--web|-b|--build]`
- `orc serve-web-api [--addr <host:port>]`
- `orc worker-create`
- `orc worker-send <worker_ref|pane_id> <msg...> [enter|enter-exit|raw|display]`
- `orc worker-wait <worker_ref|pane_id> <pattern> [timeout_ms] [lines]`
- `orc worker-close <worker_ref|pane_id>`
- `orc worker-dev-url <worker_ref|pane_id> [lines]`
- `orc manager-trace <stage> [detail...]`
- `orc check-manager-trace [preflight|impl|check|final]`
- `orc capture-pane <pane_id> [lines]`
- `orc wait-ready <pane_id> <pattern> [timeout_ms] [lines]`
- `orc http-healthcheck <url> [timeout_ms]`
- `orc auto <message>`
- `orc auto -f` (auto-generate `job.md` from project metadata, then continue to implementation)

## UI Mode
- Enter UI mode:
- `orc open-ui` (TUI)
- `orc open-ui -w` (Web UI dev server at `assets/web`)
- `orc open-ui -b` (build then preview serve at `assets/web`)

## Rust Web API Mode
- Start Rust API server:
  - `cargo run --bin orc -- serve-web-api --addr 127.0.0.1:7788`
- Run Astro Web UI against Rust API:
  - `PUBLIC_ORC_API_BASE=http://127.0.0.1:7788 npm --prefix assets/web run dev`

## tmux Worker
- Create a worker pane and get a reusable worker ref:
  - `orc worker-create`
- Send text to a worker pane:
  - `orc worker-send <worker_ref|pane_id> <msg...> [enter|enter-exit|raw|display]`
- Wait until worker output contains a pattern:
  - `orc worker-wait <worker_ref|pane_id> <pattern> [timeout_ms] [lines]`
- Close a worker pane:
  - `orc worker-close <worker_ref|pane_id>`
- Resolve the latest actual dev URL reported by a worker pane:
  - `orc worker-dev-url <worker_ref|pane_id> [lines]`
- Append an `orc_manager` trace stage:
  - `orc manager-trace <stage> [detail...]`
- Verify `orc_manager` trace order:
  - `orc check-manager-trace [preflight|impl|check|final]`

## tmux Worker Helpers
- Capture recent pane output:
  - `orc capture-pane <pane_id> [lines]`
- Wait until pane output contains a readiness pattern:
  - `orc wait-ready <pane_id> <pattern> [timeout_ms] [lines]`
- Check whether a dev server URL is responding:
  - `orc http-healthcheck <url> [timeout_ms]`
- `orc send-tmux`는 일반 tmux 브리지로 유지되지만 worker orchestration 표준으로는 사용하지 않는다.

## Notes
- `cargo run --bin rc -- run-playwright-qa --web-root assets/web -- <command...>` runs Playwright/Node QA commands from the installed web workspace with `NODE_PATH`, `.bin`, and helper env wired by `rc`.
- `cargo run --bin rc -- check-front-ui-rules` is the canonical UI alignment check entrypoint used by the repo wrappers/docs.
- `orc chat -n <name>` 실행 시 `.temp/<name>.yaml`이 없거나 비어 있으면 기본 chat room YAML이 자동 생성됩니다.
- `orc chat -n <name> --background`는 watcher를 백그라운드로 실행하고, 출력은 `.temp/<name>.watch.log`에 기록됩니다.
- 같은 tmux pane(기준: `TMUX_PANE`)에서 `orc chat`을 여러 번 호출하면 동일 `sender_id`를 재사용합니다. 즉 같은 window라도 pane이 다르면 `sender_id`는 독립적으로 관리됩니다. tmux 외 환경은 fallback(`PPID + TTY`), 강제 지정은 `ORC_CHAT_SESSION_KEY`를 사용합니다 (`.temp/<name>.sessions.yaml`).
- `orc chat-wait -n <name> -a true`는 모든 새 메시지에 반응하고, `-a false`는 자신의 `sender_id`를 receiver로 가진 메시지에만 반응합니다.
- `orc chat-wait -n <name> -a <true|false> -c <count>`를 사용하면 지정 개수 반응 후 자동 종료됩니다.
- UI has two tabs: `Projects` and `Selected Project`.
- In `Selected Project`, three panes are shown:
  - Project info pane
  - Draft feature list pane
  - Parallel runtime pane
- Initial active pane is the Project pane.
- Pane border colors come from `configs/style.yaml` (`active` / `inactive`).
- `q` closes current focused menu (to inactive). If already inactive, `q` exits UI.
- In `Project Select` tab, press `m` to run auto mode for the selected project.
