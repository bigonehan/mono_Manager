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
- `orc create_job_md`
- `orc add_orc_drafts`
- `orc impl_orc_code`
- `orc check_orc_code`
- `orc cli_rust_orchestra`
- `orc chat -n <name>`
- `orc chat -n <name> --background`
- `orc chat -n <name> -m <message> [-i <receiver_id>] [--data <data>]`
- `orc chat-wait -n <name> -a <true|false> [-c <count>]`
- `orc open-ui [-w|--web|-b|--build]`
- `orc serve-web-api [--addr <host:port>]`
- `orc worker-create [name]`
- `orc worker-send <worker_ref|pane_id> <msg...>|--stdin [enter|enter-exit|raw|display]`
- `orc worker-wait <worker_ref|pane_id> <pattern> [timeout_ms] [lines]`
- `orc worker-close <worker_ref|pane_id>`
- `orc worker-dev-url <worker_ref|pane_id> [lines]`
- `worker-create`가 반환한 전체 `worker_ref`를 그대로 재사용하는 것이 기준이다. ref가 잘려 `worker_id`만 남아도 최근 registry에서 같은 impl worker pane을 다시 찾는다.
- `orc manager-trace <stage> [detail...]`
- `orc check-manager-trace [preflight|impl|check|final]`
- `orc check-manager-completion [job.md]`
- `orc capture-pane <pane_id> [lines]`
- `orc wait-ready <pane_id> <pattern> [timeout_ms] [lines]`
- `orc http-healthcheck <url> [timeout_ms]`
- `orc auto <message>`
- `orc auto -f` (auto-generate `job.md` from project metadata, then continue to implementation)

## Standard ORC Chain
- Default implementation chain:
  - `orc create_job_md`
  - `orc add_orc_drafts`
  - `orc impl_orc_code`
  - `orc check_orc_code`
- Add helper verification only when needed:
- `orc capture-pane`
- `orc wait-ready`
- `orc http-healthcheck`
- browser e2e / screenshot validation
- `check_orc_code`는 `job.md`의 `# check evidence` 실행 증거가 없으면 성공으로 끝나면 안 된다.
- 유닛 테스트 통과는 보조 신호다. 상태 변화 기능은 재진입/reload 검증, UI 기능은 실제 렌더 근거까지 포함해야 한다.
- Removed legacy commands are not supported and must not appear in docs or workflow guidance.

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
- Create a worker tmux session and get a reusable worker ref:
  - `orc worker-create [name]`
- Send text to a worker session pane:
  - `orc worker-send <worker_ref|pane_id> <msg...>|--stdin [enter|enter-exit|raw|display]`
- Wait until worker output contains a pattern:
  - `orc worker-wait <worker_ref|pane_id> <pattern> [timeout_ms] [lines]`
- Close a worker tmux session:
  - `orc worker-close <worker_ref|pane_id>`
- Resolve the latest actual dev URL reported by a worker session pane:
  - `orc worker-dev-url <worker_ref|pane_id> [lines]`
- Append an `orc_manager` trace stage:
  - `orc manager-trace <stage> [detail...]`
- Verify `orc_manager` trace order:
  - `orc check-manager-trace [preflight|impl|check|final]`
- Verify `orc_manager` completion gate from `job.md` without shell wrappers:
  - `orc check-manager-completion [job.md]`

## tmux Worker Helpers
- Capture recent pane output:
  - `orc capture-pane <pane_id> [lines]`
- Wait until pane output contains a readiness pattern:
  - `orc wait-ready <pane_id> <pattern> [timeout_ms] [lines]`
- Check whether a dev server URL is responding:
  - `orc http-healthcheck <url> [timeout_ms]`
- `orc send-tmux`는 일반 tmux 브리지로 유지되지만 worker orchestration 표준으로는 사용하지 않는다.
- worker 완료는 `worker:` 문자열 자체를 기다리지 말고 동적으로 만든 sentinel을 사용한다.
  - 권장: `marker=$(printf '__ORC_%s__' DONE)` 후 `echo "$marker worker:<session_name>:done:dev=${url};report=${report}"`
  - manager 대기는 `orc worker-wait <worker_ref> "$marker"` 형식으로 수행한다.
- `worker-dev-url`는 실제 완료 줄에서만 URL을 회수한다. 입력 명령줄에 `dev=http://...` literal을 직접 넣지 말고 shell 변수에서 최종 `echo`에만 출력한다.
- worker 시작 전에는 `orc cli_help`가 `worker-create [name]`와 `worker-send ...|--stdin`를 보여 주는지 확인하고, 다르면 `cargo install --path /home/tree/project/mono_Manager --bin orc --force`로 설치 바이너리를 먼저 갱신한다.

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
