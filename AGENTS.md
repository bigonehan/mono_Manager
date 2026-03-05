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
- Forbidden examples: `맞습니다`, `맞아요`, `인식했습니다`, `확인했습니다`.
- Start directly with result/action without those prefaces.
- Hard ban: never output `맞습니다` in any response, including short acknowledgements, summaries, or status updates.
- Additional banned starters: `네, 맞습니다`, `맞습니다.`, `네 맞습니다`, `그렇습니다`, `확인했습니다`.
- Pre-send guard: before every response, scan the final text and if any banned phrase appears, rewrite the sentence and re-check before sending.
- Enforcement order:
  1. Draft response
  2. Run banned phrase scan
  3. Rewrite with neutral action/result wording (no acknowledgement phrasing)
  4. Re-scan and send only if zero banned matches

## CLI Execute-First Interpretation Rule
- If the user says phrases like `호출해서 실행`, `실행해봐`, `돌려봐`, interpret the request as **run existing CLI command first**, not implementation.
- In this case, do not edit code/docs unless the user explicitly asks to implement/change.
- Output must prioritize executed command + result summary.
- If command execution hangs, first report hang reason and ask whether to stop/retry with timeout; do not switch to implementation.

## No-Hardcoding Default Rule
- Unless the user explicitly requests hardcoding, do not implement behavior with hardcoded domain/output-specific branches.
- Prefer prompt-driven LLM inference using files under `assets/code/prompts` for generation/decision paths.
- If temporary fallback is unavoidable, keep it minimal and generic (non-domain-specific), and treat it as a last resort.

## Legacy Compatibility Removal Rule
- Remove legacy compatibility paths/modes instead of keeping dual-path support.
- When standard path/name changes, keep only the current canonical path and update callers in the same change.


## Request Summary Output Rule
- For every user request, before starting work, output using this exact 2-line format:
- Line 1: `요구사항 요약 > [${행동 설명:생성, 추가, 삭제, 변경}] ${대상}은 기능 한줄 요약`
- Line 2: `[결과] : 일어날 결과`
- Keep this output concise and always place it immediately before implementation.

## Screenshot Path Memory Rule
- When the user says `current.png`, resolve it to this fixed directory by default:
  - `/mnt/c/Users/tende/Pictures/Screenshots/current.png`
- If only folder context is needed, use:
- Treat this mapping as persistent unless the user explicitly changes it.

## Plan First Rule (Permanent)
- Before any source code edit, create or update `plan.md` first.
- Minimum `plan.md` structure is mandatory: `문제`, `해결책`, `검증`.
- If `plan.md` is missing, stop editing source and write `plan.md` first.

## Retry Loop Rule (Permanent)
- Required execution loop:
  1) 문제 제시 + 해결책 + 검증 기준 설정후 `plan.md` 생성 
  2) 해결책 시도
  3) 검증 실행
  4) 실패 시  `feedback.md` 생성후 이를 바탕으로 `plan.md`문제를 재설계 
  5) 재 정비된 plan.md 문서를 바탕으로 처음부터 전체 재시작
- On failure, write/update `feedback.md` and append retry reason to `plan.md` before restarting.
- Do not stop at intermediate logs only; continue until pass or max retry reached.

## Rule-First Enforcement (Highest Priority)
- On any new user behavioral instruction, update `AGENTS.override.md` first before running commands or editing source.
- If execution already started, stop running process first, write rule, then resume work.
- This rule has higher priority than implementation speed.

## Temp Auto Loop Rule (Permanent)
- When user requests `orc cli` validation in `/home/tree/temp`, run iterative loop with this order:
  1) write/update `plan.md`
  2) remove and recreate `/home/tree/temp`
  3) run `orc auto` for requested app
  4) if failed, write `/home/tree/temp/feedback.md` with 문제/미해결점
  5) reflect feedback into next plan and restart from step 1
- Keep looping until verification passes or hard technical blocker is confirmed.

## Feedback->Plan Merge Rule (Highest Priority)
- After any failure, write/update `feedback.md` first with `문제` and `미해결점`.
- Then update `plan.md` by merging prior plan + new feedback deltas.
- The updated `plan.md` must include:
  - new/changed problem statements
  - concrete solution steps
  - forced execution item (must-apply action)
- Do not run the next attempt unless merged `plan.md` has been written.

## Forced Resolution Rule
- Retry is not a blind rerun.
- Every retry must apply at least one concrete change from updated `plan.md` before execution.
- If no new change is applied, stop and mark as process violation.

## Failure-Solution Mandatory Rule (Highest Priority)
- If any failure cause is detected, `plan.md` must be updated with a concrete fix for that exact cause before next run.
- `plan.md` update is invalid if it only repeats the problem without actionable solution steps.
- Retry execution is blocked until the failure->solution mapping is explicitly written in `plan.md`.

## Regret Skill Trigger Rule (Highest Priority)
- If the assistant output includes the token `잘못` in any channel, run the `regret` skill immediately in the same turn.
- Required action order:
  1) Append one item to `/home/tree/ai/skills/regret/references/report.md` under `# 잘못한점`.
  2) Append one item to `/home/tree/ai/skills/regret/references/report.md` under `# 개선할점`.
  3) State that the regret skill execution record was written.
- This rule is mandatory for `commentary`, `final`, and `summary` channels.
