# rust-orc

## Run
- Build and run with the `orc` binary target:
  - `cargo run --bin orc -- <command>`

## Main Commands (orc)
- `orc help`
- `orc list-projects`
- `orc create-project <name> [path] [description]`
- `orc select-project <name>`
- `orc delete-project <name>`
- `orc plan-project [llm]`
- `orc detail-project [llm]`
- `orc chat -n <name>`
- `orc chat -n <name> --background`
- `orc chat -n <name> -m <message> [-i <receiver_id>] [--data <data>]`
- `orc chat-wait -n <name> -a <true|false> [-c <count>]`
- `orc create-draft`
- `orc add-plan [hint]`
- `orc add-draft <feature_name> [request]`
- `orc delete-draft <feature_name>`
- `orc validate-tasks <feature_name>`
- `orc add-function`
- `orc build-parallel-code`
- `orc build-parallel-todo`
- `orc run_parallel_test`
- `orc feedback`
- `orc build-function-auto` (alias: `build-todo-auto`, `build-functon-auto`)
- `orc press-key <key>`

## UI Mode
- Enter UI mode:
- `orc open-ui`
- `orc run-auto [project_name]`
  - or `cargo run --bin orc -- ui`

## tmux Send
- Send text to a tmux pane:
  - `orc send-tmux <pane_id> <msg...> [enter|raw]`
- Options:
  - `enter` (default): send message and press Enter
  - `raw`: send message only

## Notes
- `orc chat -n <name>` 실행 시 `.temp/<name>.yaml`이 없거나 비어 있으면 기본 chat room YAML이 자동 생성됩니다.
- `orc chat -n <name> --background`는 watcher를 백그라운드로 실행하고, 출력은 `.temp/<name>.watch.log`에 기록됩니다.
- 같은 tmux pane(기준: `TMUX_PANE`)에서 `orc chat`을 여러 번 호출하면 동일 `sender_id`를 재사용합니다. 즉 같은 window라도 pane이 다르면 `sender_id`는 독립적으로 관리됩니다. tmux 외 환경은 fallback(`PPID + TTY`), 강제 지정은 `ORC_CHAT_SESSION_KEY`를 사용합니다 (`.temp/<name>.sessions.yaml`).
- `orc chat-wait -n <name> -a true`는 모든 새 메시지에 반응하고, `-a false`는 자신의 `sender_id`를 receiver로 가진 메시지에만 반응합니다.
- `orc chat-wait -n <name> -a <true|false> -c <count>`를 사용하면 지정 개수 반응 후 자동 종료됩니다.
- `orc run_parallel_test`는 `test` room을 준비하고 10개 백그라운드 프로세스를 실행한 뒤, `chat-wait`로 완료 메시지 10개를 기다립니다.
- UI has two tabs: `Projects` and `Selected Project`.
- In `Selected Project`, three panes are shown:
  - Project info pane
  - Draft feature list pane
  - Parallel runtime pane
- Initial active pane is the Project pane.
- Pane border colors come from `configs/style.yaml` (`active` / `inactive`).
- `q` closes current focused menu (to inactive). If already inactive, `q` exits UI.
- In `Project Select` tab, press `m` to run auto mode for the selected project.
