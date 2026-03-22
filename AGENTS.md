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

## Auto Install Rule
- When any task is completed, run `cargo install --path /home/tree/project/mono_Manager --bin orc --force` automatically before finalizing the task.
- Completion preflight order is fixed: `cargo install` -> `nf -m "<task-name> complete"` -> final response.

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
- If a run fails or a problem is detected, append the failure cause and retry strategy to `.project/feedback.md` immediately.
- After updating `.project/feedback.md`, apply a concrete fix and rerun the same execution path.
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
- Prefer prompt-driven LLM inference using files under `assets/presets/code/prompts` for generation/decision paths.
- If temporary fallback is unavoidable, keep it minimal and generic (non-domain-specific), and treat it as a last resort.

## Legacy Compatibility Removal Rule
- Remove legacy compatibility paths/modes instead of keeping dual-path support.
- When standard path/name changes, keep only the current canonical path and update callers in the same change.


## Request Summary Output Rule
- For every user request, before starting work, output with label and description split across separate lines.
- Line 1: `요구사항 요약 >`
- Line 2: `[${행동 설명:생성, 추가, 삭제, 변경}]`
- Line 3: `${대상}은 기능 한줄 요약`
- Line 4: `[결과]`
- Line 5: `일어날 결과`
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
  4) 실패 시  `.project/feedback.md` 생성후 이를 바탕으로 `plan.md`문제를 재설계 
  5) 재 정비된 plan.md 문서를 바탕으로 처음부터 전체 재시작
- On failure, write/update `.project/feedback.md` and append retry reason to `plan.md` before restarting.
- Do not stop at intermediate logs only; continue until pass or max retry reached.

## Rule-First Enforcement (Highest Priority)
- On any new user behavioral instruction, update `/home/tree/ai/codex/AGENTS.override.md` first before running commands or editing source.
- If execution already started, stop running process first, write rule, then resume work.
- This rule has higher priority than implementation speed.

## Temp Auto Loop Rule (Permanent)
- When user requests `orc cli` validation in `/home/tree/temp`, run iterative loop with this order:
  1) write/update `plan.md`
  2) remove and recreate `/home/tree/temp`
  3) run `orc auto` for requested app
  4) if failed, write `/home/tree/temp/.project/feedback.md` with 문제/미해결점
  5) reflect feedback into next plan and restart from step 1
- Keep looping until verification passes or hard technical blocker is confirmed.

## Feedback->Plan Merge Rule (Highest Priority)
- After any failure, write/update `.project/feedback.md` first with `문제` and `미해결점`.
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

## AGENTS Override

- 2026-03-05: `draft_item` 생성은 주석이 포함된 템플릿(`assets/presets/code/templates/draft_item.yaml`)을 LLM 입력으로 사용해 의미를 추론한 뒤 값 채우기로 수행한다.
- 최종 산출물(`.project/drafts.yaml` item)에는 템플릿 주석/예시/placeholder를 포함하지 않는다.
- `draft_item` 관련 프롬프트는 "주석 읽기 -> 값 채우기 -> 주석 제거" 순서를 명시해야 한다.
- 2026-03-05: `if)` 가상 시나리오 출력은 줄 단위 `a -> b` 포맷만 사용한다. 각 단계는 반드시 다음 줄에 분리해서 작성한다.
- 2026-03-05: 사용자가 `~~~을 만들어줘` 형태로 요청하면 매니저 pane이 워커 pane을 단계별로 열고(`tmux split-window`), `orc send-tmux`로 `auto -> plan -> drafts -> impl -> check_code_draft -a`를 순차 위임/완료 회수/재시도 판단하는 흐름을 우선 적용한다.
- 2026-03-05: 트리거 문구는 `~~~을 만들어줘`, `~~~을 추가해줘`, `~을 읽고 처리해줘` 3가지를 동일 계열로 인식한다. 단, `읽고 처리해줘`는 기존 `input.md`를 읽는 명령 경로(`add_code_plan -f`, `add_code_draft -f`)를 사용하고 `create_input_md`를 호출하지 않는다.
- 2026-03-08: draft 타입 스키마 변경 시 web UI `edit_{type}_drafts` 모달(`edit_code_drafts`, `edit_mono_drafts`, `edit_video_drafts`, `edit_write_drafts`)을 동일 변경에서 함께 갱신한다.
- 2026-03-06: profile 레이어 리팩토링 작업 시 별도 브랜치에서 진행한다.
- 2026-03-06: profile 종속 범위는 prompt/template 로딩(project.md, plan.yaml, drafts.yaml, parallel run) 및 해당 호출 프롬프트로 한정한다.
- 2026-03-06: 공통 인터페이스는 project/plan/draft/feedback 흐름을 우선 제공하고, 기본 구현체는 code profile로 유지한다.
- 2026-03-07: 모든 작업 완료 시 `nf -m "<task-name> complete"` 실행을 강제한다. 별도 요청이 없어도 필수로 실행하며 `notify.fish` 직접 호출은 금지한다.
- 2026-03-07: 사용자가 `/temp` 검증 루프를 요청하면 `/home/tree/temp`를 삭제/재생성 후 `orc auto`를 실행하고, 실패 시 `/home/tree/temp/todo.md`와 `/home/tree/temp/.project/feedback.md`를 작성한 뒤 plan 갱신 후 재시도한다.
- 2026-03-07: 사용자가 web UI 확장(`open-ui -w`, assets 기반 frontend, playwright 검증 루프)을 요청한 경우, TUI 기능 목록을 먼저 추출해 `input.md`에 반영한 뒤 구현/검증을 반복한다.
- 2026-03-07: web UI는 `project`/`detail` 탭 분리 구조를 유지하고, detail 편집은 pane 선택 시 우상단 gear 아이콘으로 진입하는 읽기전용 기본 화면으로 제공한다.
- 2026-03-07: web UI 상태는 로컬 useState보다 `zustand` 스토어를 우선 사용해 탭/선택/편집/로그를 중앙 관리한다.
- 2026-03-07: project pane의 선택 item 우상단에 수정/삭제 아이콘 버튼(SVG)을 노출하고, 해당 item 단위 edit/delete 동작을 제공한다.
- 2026-03-07: UI 스타일 변경 요청 시 `current.png`를 기준으로 project info 시각톤을 맞추고, 모든 pane 컨테이너에 rounded border를 강제 적용한다.
- 2026-03-07: project 탭은 grid 카드 + 생성 모달 구조를 기본으로 하며, 카드에 type 라벨/상태 태그/폴더 아이콘/큰 제목을 표시하고 `project_type(story|movie|code|mono)` 필드를 기본 `code`로 유지한다.
- 2026-03-07: web navbar는 좌측에 현재 선택 프로젝트 표시, 우측에 project/detail 탭 버튼을 두고, 카드형 border 대신 하단 underline(border-b)만 사용한다.
- 2026-03-07: `current.png` 요청은 검색 없이 `/mnt/c/Users/tende/Pictures/Screenshots/current.png`를 먼저 연다. 해당 경로 미존재 시 그 경로 부재만 즉시 보고하고 대체 경로를 요청한다.
- 2026-03-07: 사용자가 개선사항에 `전부`, `모두`, `전체`를 명시하면 부분 개선 보고를 금지하고, 경고/실패가 남지 않을 때까지 연속으로 수정-검증을 반복한 뒤 최종 결과만 보고한다.
- 2026-03-08: templates asset 모달은 좌측 `PROMPTS/TEMPLATES` 폴더 섹션(접기/펼치기) + 우측 파일 내용 패널 구조를 유지하고, 파일 저장 시 `{수정 파일 경로} 수정 반영 후 관련 항목 전체 갱신` LLM 요청을 자동 실행한다.
- 2026-03-08: detail pane의 add/build 흐름은 `form_add_input` 모달 기반으로 유지한다. add 확인 시 `input.md`를 생성하고 `orc add_code_plan -f` + `orc add_code_draft -f`를 실행해 `plan.yaml`/`drafts.yaml`를 갱신하며, build는 병렬 실행 상태(`build`)와 `current_job`을 project 카드에 반영한다.
- 2026-03-09: tmux pane 명령 전송은 기본 셸을 `fish -ic`로 고정하고 `bash -lc`/`bash -ic` 래퍼 생성을 금지한다(사용자가 bash를 명시한 경우만 예외).
- 2026-03-10: repo 루트의 규칙 파일은 `AGENTS.md` 하나로 유지한다. `/home/tree/project/rust-orc` 아래에 `AGENTS.override` 또는 `AGENTS.override.md`를 새로 만들거나 심볼릭 링크로 생성하지 않는다.
- 2026-03-10: repo 전용 규칙 추가는 `AGENTS.md`에 직접 병합하고, 전역 사용자 동작 규칙만 `/home/tree/ai/codex/AGENTS.override.md`에 기록한다.
