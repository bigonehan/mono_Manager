# Project Agent Rules

## CLI Rename Sync Rule
- If any CLI command name, subcommand alias, or file name shown by CLI help is changed, update all related help/documentation in the same change.
- Minimum sync targets:
  - `src/cli.rs` usage/help output
  - `README.md` command list/examples
  - Any command examples in repo docs
- Do not finish the task until `cargo run --bin orc -- --help` matches the documented command names.

## CLI Help Update Rule
- Whenever a feature addition introduces or changes CLI behavior/commands, update the `orc` help command list in the same change.

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

## Failure Retry Rule
- If a run fails or a problem is detected, append the failure cause and retry strategy to `feedback.md` immediately.
- After updating `feedback.md`, apply a concrete fix and rerun the same execution path.
- Continue this loop until the target path no longer reports the same blocking failure.

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
- Hard ban: never output `맞습니다` in any response, including short acknowledgements, summaries, or status updates.
- Additional banned starters: `네, 맞습니다`, `맞습니다.`, `네 맞습니다`, `그렇습니다`.
- Pre-send guard: before every response, scan the final text and if any banned phrase appears, rewrite the sentence and re-check before sending.

## CLI Execute-First Interpretation Rule
- If the user says phrases like `호출해서 실행`, `실행해봐`, `돌려봐`, interpret the request as **run existing CLI command first**, not implementation.
- In this case, do not edit code/docs unless the user explicitly asks to implement/change.
- Output must prioritize executed command + result summary.
- If command execution hangs, first report hang reason and ask whether to stop/retry with timeout; do not switch to implementation.

## No-Hardcoding Default Rule
- Unless the user explicitly requests hardcoding, do not implement behavior with hardcoded domain/output-specific branches.
- Prefer prompt-driven LLM inference using files under `assets/code/prompts` for generation/decision paths.
- If temporary fallback is unavoidable, keep it minimal and generic (non-domain-specific), and treat it as a last resort.
