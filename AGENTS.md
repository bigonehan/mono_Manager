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

## UI Flow Verification Rule
- When the user requests a UI change, verify and implement the connected internal behavior flow in the same task.
- Do not finish with visual/UI text changes only; confirm trigger -> command/action -> state/file update -> UI refresh path is connected end-to-end.
- Before finalizing, run at least one real execution path (or equivalent CLI path) and report whether the functional flow is actually wired.

## Execution-Path Verification Rule
- If behavior spans multiple steps, validate by execution path instead of isolated function success.
- Minimum path check: trigger input -> invoked command/action -> file/state transition -> follow-up action result.
- For UI-triggered behavior, include an equivalent CLI verification when direct UI automation is hard.
- Treat "status text changed" or "modal rendered" as insufficient evidence unless state/files reflect the expected transition.
- When path validation fails, report the broken stage explicitly and fix wiring before finalizing.

## YAML/MD Format Enforcement Rule
- Any function that generates YAML/Markdown via LLM prompt must include explicit output format/schema constraints in the prompt.
- Generated YAML/Markdown must be parsed/validated before write; if validation fails, do not write files and return a format error.
- For `project.md`, enforce required section headers and domain block structure from `plan-project-code` reference format.
- For `draft/task` YAML, enforce schema-level validation (required fields + rule/contracts structure checks) before persisting.
- This rule is mandatory for all future YAML/Markdown generation tasks unless the user explicitly disables it.

## Planning Framework Rule
- Task minimum unit must include: feature, domain, flow.
- Planning order is fixed: feature -> domain -> flow.
- Implementation preparation order is fixed:
  - collect features -> identify domains -> define flows -> assign domain/flow per feature -> implement.
- Domain and flow each have independent rules/constraints.
- Final task constraints are composed from domain constraints + flow constraints + task-local constraints.
- All task records must be written under `./.project/`.
- For YAML/Markdown output, copy template first, then remove comments/examples, then fill values.

## Init-Plan Sequence Rule
- `init-plan` input minimum set: `name`, `description`, `path`, `spec`.
- Collect features in object-list format: `#기능 이름 - 기능 규칙 > 기능 순서`.
- Add collected features into `project.md` `## plan` list.
- Generate domains from plan using `build-domain` skill.
- Generate stages from plan + domains.
- Create `.project/drafts_list.yaml`.
- Append feature list into `drafts_list.yaml.planned`.
- Then wait for: `add-domain`, `add-rule`, `add-step`, `enter-draft`.

## Draft Stage Rule
- `enter-draft` enters `stage_draft`.
- `stage_draft` must show current `drafts_list.yaml.planned`.
- `create-draft` loops planned items and creates:
  - `./.project/feature/<feature>/task.yaml`.
- After create, wait for: `set-draft`, `add-draft`, `enter-parallel`.
- `set-draft` updates selected draft's `rule`, `step`, `domain`, `flow`.
- `add-draft` receives object-list input and confirms `domain`, `step`, `rule`, `stage` per object via LLM.

## Check/Build Rule
- `check-draft` must:
  - inspect dependency by `stage`/`domain` across task files.
  - validate virtual scenario path when user uses `if)` pattern.
  - use `check-code` skill.
- `build-draft` starts only after `check-draft` passes.
- `build-draft` implements drafts in parallel and then enters `enter-check-job`.
- `enter-check-job` must:
  - verify generated files against rules in `project.md`, `stage.md`, `task.md`.
  - use `check-code` skill.
  - move completed feature dirs from `./.project/features/<name>` to `./project/clear/<name>`.
  - move `tasks_list.planned` items to `tasks_list.features`.
  - move `project.md ## plan` items to `## features`.

## Response Phrase Rule
- Do not use agreement-preface phrases in responses.
- Forbidden examples: `맞습니다`, `맞아요`, `인식했습니다`.
- Start directly with result/action without those prefaces.
