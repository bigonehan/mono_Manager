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
- `orc create-draft`
- `orc add-plan [hint]`
- `orc add-draft <feature_name> [request]`
- `orc delete-draft <feature_name>`
- `orc validate-tasks <feature_name>`
- `orc add-function`
- `orc build-parallel-code`
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
- UI has two tabs: `Projects` and `Selected Project`.
- In `Selected Project`, three panes are shown:
  - Project info pane
  - Draft feature list pane
  - Parallel runtime pane
- Initial active pane is the Project pane.
- Pane border colors come from `configs/style.yaml` (`active` / `inactive`).
- `q` closes current focused menu (to inactive). If already inactive, `q` exits UI.
- In `Project Select` tab, press `m` to run auto mode for the selected project.
