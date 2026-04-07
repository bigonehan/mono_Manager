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

## UI Axis Alignment Hard Gate
- detail page의 sidebar/main 수평축 정렬 규칙은 이 문서를 단일 원천으로 사용한다.
- UI 변경이 포함된 작업은 항상 `cargo run --bin rc -- check-front-ui-rules`를 실행한다.
- 위 검증 실패 시 완료 보고를 금지한다.
- `current.png` 기준 수평축 규칙:
- 왼쪽은 `search folders... + mic 버튼` 행이 먼저 나오고, 그 바로 아래 `project sidebar card`가 시작된다.
- 오른쪽 메인 영역의 첫 카드 시작 y축은 왼쪽 `project sidebar card`의 시작 y축과 같아야 한다(검색 입력 행과는 맞추지 않는다).
- detail desktop 정렬 기준은 `detail-sidebar-shell` vs `detail-main-shell`의 시작 y축 오차가 2px 이하여야 한다.
- pane 내부 정렬을 수정한 턴에서는 outer shell만 검사하면 실패로 처리한다.
- `domains`, `drafts`, `check` 같은 개별 pane의 헤더/본문 라인을 바꿨다면 해당 pane에 전용 `data-testid`를 추가하고, e2e에서 헤더와 본문 panel의 좌우 기준선 오차를 각각 2px 이하로 검증해야 한다.
- `detail-main-shell만 통과`하거나 `스크린샷 육안 확인만 수행`한 상태로는 완료 보고를 금지한다.
- fail-closed: 수평선/기준선 관련 수정 후에는 `cargo run --bin rc -- check-front-ui-rules`와 해당 pane 전용 alignment assertion이 둘 다 통과해야만 종료할 수 있다.

## UI Flow Verification Rule
- When the user requests a UI change, verify and implement the connected internal behavior flow in the same task.
- Do not finish with visual/UI text changes only; confirm trigger -> command/action -> state/file update -> UI refresh path is connected end-to-end.
- Before finalizing, run at least one real execution path (or equivalent CLI path) and report whether the functional flow is actually wired.
- Sidebar와 우측 detail pane의 수평축 정렬 검사를 필수로 수행한다.
- 최소 검사 항목: sidebar search 기준선, sidebar-right pane 경계선, 각 pane 우하단 액션 버튼의 동일 y축.

## Detail End Hook Rule
- After completing UI interaction fixes, run detail-page end hook verification before final response.
- End hook command is fixed: `npm --prefix assets/web run test:e2e:end-hook`.
- End hook must validate real button interactions (input modal submit, state/file update, modal open/close) rather than visibility-only checks.
- If end hook fails, completion reporting is blocked until fix + rerun pass.

## Performance E2E Mandatory Rule
- When the task is performance improvement/optimization, E2E execution is mandatory before completion.
- Fixed E2E command: `npm --prefix assets/web run test:e2e`.
- Do not send final completion if E2E fails; report failure cause and retry result first.
- After E2E, remove test projects and verify cleanup:
  - `rm -rf /tmp/tmp_project /tmp/pw-*`
  - confirm no residual matches for `tmp_project` or `pw-*` under `/tmp`.

## Execution-Path Verification Rule
- If behavior spans multiple steps, validate by execution path instead of isolated function success.
- Minimum path check: trigger input -> invoked command/action -> file/state transition -> follow-up action result.
- For UI-triggered behavior, include an equivalent CLI verification when direct UI automation is hard.
- Treat "status text changed" or "modal rendered" as insufficient evidence unless state/files reflect the expected transition.
- When path validation fails, report the broken stage explicitly and fix wiring before finalizing.

## Failure Retry Rule
- If a run fails or a problem is detected, append the failure cause and retry strategy to `job.md` (`# problems`, `# check`) immediately.
- After updating `job.md` verification sections, apply a concrete fix and rerun the same execution path.
- Continue this loop until the target path no longer reports the same blocking failure.

## ORC Check Hard Gate
- 점검은 `check_orc_code`와 `check-code` skill 기준으로만 진행한다.
- web runner 검사는 `check_orc_code`가 선택한 브라우저 e2e 절차로 실제 스크린샷 또는 snapshot 근거를 남겨야 한다.
- web 검증이 성공하면 검증에 사용한 임시 스크린샷 파일은 완료 전에 정리한다.
- 검증 결과 기록은 새 보조 문서로 분리하지 말고 항상 `job.md` 내부 섹션에 누적한다. 기본 위치는 `# check`, `# check evidence`, `# check feedback`만 허용한다.

## ORC Manager User-Intent Lock Gate
- `orc_manager`를 쓰는 턴은 사용자 원문을 먼저 `입력`, `출력`, `유지`, `추가`, `금지` 5줄로 분해하고 `job.md#input/#output/#keep/#add/#forbid`에 잠근 뒤에만 다음 단계로 갈 수 있다.
- plan 승인 전 `job.md# hard gate` 아래 `## requirement_lock`, `## forbidden_substitutions`, `## verification_examples`를 채워야 한다.
- `## verification_examples`에는 아래 검증 예시 항목이 반드시 그대로 있어야 한다.
  - `md 저장 != 메모리 유지`
  - `재시작 != reload`
  - `실제 e2e != fixture, mock, real-equivalent`
- 위 표가 없으면 plan 승인, preflight 통과, worker 생성, completion 검증을 진행하면 안 된다.
- 위 5줄은 구현 친화 요약으로 축소하지 말고, 사용자 요구 문장을 잃지 않는 수준으로 직접 적어야 한다.
- `job.md#check`에는 최소 `input_output_checklist`, `keep_checklist`, `add_checklist`, `forbid_checklist` 네 묶음이 있어야 한다.
- 각 checklist 줄은 사용자 원문 항목과 1:1로 대응되어야 하며, 함수명/내부구현/추상화된 개발자 용어만 있고 사용자 요구가 빠져 있으면 실패다.
- QA/check 보고에는 `input`, `expected output`, `keep`, `add`, `forbid` 대응 결과가 모두 들어 있어야 한다.
- 위 대응 없이 `유닛테스트 통과`, `브라우저 열림`, `함수 실행됨`만 보고하면 완료 처리하면 안 된다.
- `orc_manager` preflight trace 순서는 반드시 `stage_global_override_read -> stage_job_md_locked -> stage_plan_done -> stage_input_locked -> stage_output_locked -> stage_keep_locked -> stage_add_locked -> stage_forbid_locked -> stage_symptom_locked -> stage_success_locked`다.
- `stage_plan_done`를 잠금 stage들 뒤에 찍으면 실패다.
- 최신 런의 preflight trace가 틀렸으면 worker를 열지 말고 최신 `stage_global_override_read`부터 위 순서로 새 런을 다시 기록해야 한다.

## YAML/MD Format Enforcement Rule
- Any function that generates YAML/Markdown via LLM prompt must include explicit output format/schema constraints in the prompt.
- Generated YAML/Markdown must be parsed/validated before write; if validation fails, do not write files and return a format error.
- For `project.md`, enforce required section headers and domain block structure from `plan-project-code` reference format.
- For `draft/task` YAML, enforce schema-level validation (required fields + rule/contracts structure checks) before persisting.
- This rule is mandatory for all future YAML/Markdown generation tasks unless the user explicitly disables it.

## Planning Review Gate
- `job.md`를 바탕으로 `drafts.yaml`과 `draft_item`을 만드는 계획 단계에서는 먼저 경험많고 엄격한 시니어 개발자가 코드 리뷰에서 거부할 수 있는 요소를 식별한다.
- 기본 거부 사유 후보는 `불명확한 요구 해석`, `과한 추상화`, `검증 누락`, `회귀 위험`, `네이밍 불일치`, `dead path`, `불필요한 복잡도`, `범위를 벗어난 수정`이다.
- 계획 산출물의 `rule`, `step`, `tasks`, `constraints`, `check`는 위 거부 사유를 해소하는 범위까지만 구체화한다.
- `unrelated refactor`, `formatting sweep`, `speculative abstraction`은 계획 단계에서도 금지한다.

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

## Full-Scope Execution Gate Rule
- 사용자가 `전부`, `모두`, `전체`, `다`, `다 지워`, `다 바꿔`를 포함해 지시하면 `Full-Scope Mode`를 강제한다.
- `Full-Scope Mode`에서는 단일 파일 추정 수정을 금지하고, 먼저 현재 사용자 원문이 요구한 범위와 검증 강도를 최상위 조건으로 고정한다.
- 기존 규칙이 사용자 원문의 범위/검증/예외 조건을 더 좁게 만들면 그 규칙은 해당 턴에서 무효다.
- 삭제/제거 작업의 기본 검사 표면은 저장소 전체 `rg`다.
- 1차 검사: 사용자 원문 기준 `primary pattern`을 전수 검색한다.
- 2차 검사: 경로 변형/출력문구/escape 표현을 포함한 `alias pattern`을 전수 검색한다.
- `git ls-files`, `git grep`는 보조 확인용으로만 허용하고, 삭제 작업의 완료 판단에는 사용할 수 없다.
- 사용자가 보존하라고 명시한 경로만 `rg` 예외로 제외할 수 있고, 그 외 임의 범위 축소는 금지한다.
- 수정 완료 판단은 `primary 0건` + `alias 0건` 동시 충족일 때만 허용한다.
- 두 조건 중 하나라도 실패하면 완료 보고를 금지하고 같은 루프(검색 -> 수정 -> 재검색)를 즉시 반복한다.
- 최종 보고에는 사용자 원문 기준 삭제 대상, 실행한 `rg` 명령, 예외 경로, 검사 패턴(`primary/alias`), 0건 확인 결과를 반드시 포함한다.

## Screenshot Path Memory Rule
- When the user says `current.png`, resolve it to this fixed directory by default:
  - `/mnt/c/Users/tende/Pictures/Screenshots/current.png`
- If only folder context is needed, use:
- Treat this mapping as persistent unless the user explicitly changes it.

## Retry Loop Rule (Permanent)
- Required execution loop:
  1) 문제 제시 + 해결책 + 검증 기준 설정
  2) 해결책 시도
  3) 검증 실행
  4) 실패 시 `job.md`의 `# problems`, `# check`를 갱신 후 이를 바탕으로 문제를 재설계
  5) 재정비된 검증 상태를 바탕으로 처음부터 전체 재시작
- On failure, write/update `job.md` (`# problems`, `# check`) before restarting.
- Do not stop at intermediate logs only; continue until pass or max retry reached.

## ORC Tmux Worker Loop Hard Gate
- 사용자가 구현 실행을 요구하고 ORC/tmux 흐름이 열려 있으면, `/plan` 종료 후 구현은 현재 pane에서 직접 수행하지 않고 반드시 worker tmux session으로 위임한다.
- worker session 생성/전달/대기/종료는 `orc worker-create`, `orc worker-send`, `orc worker-wait`, `orc worker-close`만 사용한다.
- worker는 구현 종료 시 동적으로 만든 sentinel을 포함한 완료 줄만 source of truth로 사용한다.
- 권장 형식:
  - 성공: `__ORC_DONE__ worker:<session_name>:done:dev=${url};report=${report}`
  - 실패: `__ORC_FAIL__ worker:<session_name>:fail:${reason}`
- manager는 `worker:` 일반 문자열이 아니라 위 sentinel 값으로 `orc worker-wait`를 수행해야 한다.
- worker 명령 본문에 `dev=http://...` literal을 직접 박지 말고 shell 변수에서 최종 `echo`에만 출력해야 한다.
- manager pane은 worker의 `done` 메시지를 받아도 구현 완료로 간주하면 안 되고, 즉시 `job.md`를 다시 읽은 뒤 검증 단계로 들어가야 한다.
- 이 하드게이트를 건너뛰고 현재 pane에서 직접 구현/완료 보고를 하면 규칙 위반이다.

## Manager Recheck Hard Gate
- worker가 완료를 보고한 직후 manager는 기존 `job.md`를 그대로 신뢰하지 않고 반드시 다시 읽는다.
- manager 재검토 순서는 고정한다: `worker done 수신 -> job.md 재확인 -> e2e 실행 -> 실제 스크린샷 저장/확인 -> 완료/실패 판정`.
- 위 순서 중 하나라도 빠지면 완료 보고를 금지한다.
- UI가 포함된 작업은 스크린샷 산출물을 `./.project/captures/`에 저장하고, 그 이미지를 기준으로 실제 구현 상태를 확인해야 한다.

## Failed Manager Review Hard Gate
- worker가 완료를 보고했더라도 manager의 `job.md` 재검토, e2e, 스크린샷 확인 중 하나라도 실패하면 그 결과는 실패로 처리한다.
- 이 경우 manager는 기존 `job.md`를 현재 실패 상태 기준으로 갱신한 뒤에만 새 worker session을 열 수 있다.
- 실패 시 새 `job.md`는 최소 다음 정보를 포함해야 한다:
  - worker가 완료했다고 보고한 작업 요약
  - manager 재검증에서 실패한 단계
  - e2e 결과와 남은 문제
  - 스크린샷 기준으로 아직 해결되지 않은 항목
  - 다음 worker가 즉시 수행해야 할 수정 작업
  - 재검증 기준
- 위 정보 없이 재시도 pane을 다시 여는 것을 금지한다.

## Improve Loop Hard Gate
- improve는 `check` 성공 뒤 1회만 실행한다.
- improve 결과는 반드시 `blocking` 또는 `non_blocking`으로만 분류한다.
- `non_blocking` 결과는 backlog 또는 메모로만 남기고 구현/QA/check 루프를 다시 열면 안 된다.
- `blocking` 결과가 있으면 manager는 `job.md`의 `# problems`, `# check`, `## verify`를 갱신한 뒤 `impl -> qa -> check`로 1회만 재진입할 수 있다.
- improve 재진입은 최대 1회다. 재진입 뒤에도 다시 `blocking`이 나오면 자동 반복을 금지하고 실패로 종료해야 한다.
- 종료 조건은 `개선점을 더 못 찾음`이 아니라 `blocking issue 없음 + improve 재진입 한도 내 처리 완료`다.

## Rule-First Enforcement (Highest Priority)
- On any new user behavioral instruction, update `/home/tree/ai/codex/AGENTS.override.md` first before running commands or editing source.
- If execution already started, stop running process first, write rule, then resume work.
- This rule has higher priority than implementation speed.

## Temp Auto Loop Rule (Permanent)
- When user requests `orc cli` validation in `/home/tree/temp`, run iterative loop with this order:
  1) current issue와 해결 조건 정리
  2) remove and recreate `/home/tree/temp`
  3) run `orc auto` for requested app
  4) if failed, write `/home/tree/temp/job.md`의 `# problems`, `# check`에 문제/미해결점
  5) reflect feedback into next retry notes and restart from step 1
- Keep looping until verification passes or hard technical blocker is confirmed.

## Feedback->Plan Merge Rule (Highest Priority)
- After any failure, write/update `job.md` first with `문제` and `미해결점`.
- Then update retry notes by merging prior execution context + new feedback deltas.
- The updated retry notes must include:
  - new/changed problem statements
  - concrete solution steps
  - forced execution item (must-apply action)
- Do not run the next attempt unless merged retry notes have been written.

## Forced Resolution Rule
- Retry is not a blind rerun.
- Every retry must apply at least one concrete change from updated retry notes before execution.
- If no new change is applied, stop and mark as process violation.

## Failure-Solution Mandatory Rule (Highest Priority)
- If any failure cause is detected, retry notes must be updated with a concrete fix for that exact cause before next run.
- Retry note update is invalid if it only repeats the problem without actionable solution steps.
- Retry execution is blocked until the failure->solution mapping is explicitly written.

## Regret Skill Trigger Rule (Highest Priority)
- If the assistant output includes the token `잘못` in any channel, run the `regret` skill immediately in the same turn.
- Required action order:
  1) Append one item to `/home/tree/ai/skills/regret/references/regret-notes.md` under `# 잘못한점`.
  2) Append one item to `/home/tree/ai/skills/regret/references/regret-notes.md` under `# 개선할점`.
  3) State that the regret skill execution record was written.
- This rule is mandatory for `commentary`, `final`, and `summary` channels.

## AGENTS Override

- 2026-03-05: `draft_item` 생성은 주석이 포함된 템플릿(`assets/presets/code/templates/draft_item.yaml`)을 LLM 입력으로 사용해 의미를 추론한 뒤 값 채우기로 수행한다.
- 최종 산출물(`.project/drafts.yaml` item)에는 템플릿 주석/예시/placeholder를 포함하지 않는다.
- `draft_item` 관련 프롬프트는 "주석 읽기 -> 값 채우기 -> 주석 제거" 순서를 명시해야 한다.
- 2026-03-05: `if)` 가상 시나리오 출력은 줄 단위 `a -> b` 포맷만 사용한다. 각 단계는 반드시 다음 줄에 분리해서 작성한다.
- 2026-03-05: 사용자가 `~~~을 만들어줘` 형태로 요청하면 매니저 pane이 `orc worker-create -> worker-send -> worker-wait -> worker-close` 표준으로 독립 worker session에 `auto -> plan -> drafts -> impl -> check_orc_code`를 순차 위임/완료 회수/재시도 판단하는 흐름을 우선 적용한다.
- 2026-03-05: 트리거 문구는 `~~~을 만들어줘`, `~~~을 추가해줘`, `~을 읽고 처리해줘` 3가지를 동일 계열로 인식한다. 단, `읽고 처리해줘`는 기존 `job.md`를 읽는 명령 경로(`create_job_md`, `add_orc_drafts`)를 사용한다.
- 2026-03-08: draft 타입 스키마 변경 시 web UI `edit_{type}_drafts` 모달(`edit_code_drafts`, `edit_mono_drafts`, `edit_video_drafts`, `edit_write_drafts`)을 동일 변경에서 함께 갱신한다.
- 2026-03-06: profile 레이어 리팩토링 작업 시 별도 브랜치에서 진행한다.
- 2026-03-06: profile 종속 범위는 prompt/template 로딩(project.md, drafts.yaml, parallel run) 및 해당 호출 프롬프트로 한정한다.
- 2026-03-06: 공통 인터페이스는 project/plan/draft/feedback 흐름을 우선 제공하고, 기본 구현체는 code profile로 유지한다.
- 2026-03-07: 모든 작업 완료 시 `nf -m "<task-name> complete"` 실행을 강제한다. 별도 요청이 없어도 필수로 실행하며 `notify.fish` 직접 호출은 금지한다.
- 2026-03-07: 사용자가 `/temp` 검증 루프를 요청하면 `/home/tree/temp`를 삭제/재생성 후 `orc auto`를 실행하고, 실패 시 `/home/tree/temp/job.md`의 `# problems`, `# check`를 갱신한 뒤 다음 재시도 조건을 갱신한다.
- 2026-03-07: 사용자가 web UI 확장(`open-ui -w`, assets 기반 frontend, playwright 검증 루프)을 요청한 경우, TUI 기능 목록을 먼저 추출해 `job.md`에 반영한 뒤 구현/검증을 반복한다.
- 2026-03-07: web UI는 `project`/`detail` 탭 분리 구조를 유지하고, detail 편집은 pane 선택 시 우상단 gear 아이콘으로 진입하는 읽기전용 기본 화면으로 제공한다.
- 2026-03-07: web UI 상태는 로컬 useState보다 `zustand` 스토어를 우선 사용해 탭/선택/편집/로그를 중앙 관리한다.
- 2026-03-07: project pane의 선택 item 우상단에 수정/삭제 아이콘 버튼(SVG)을 노출하고, 해당 item 단위 edit/delete 동작을 제공한다.
- 2026-03-07: UI 스타일 변경 요청 시 `current.png`를 기준으로 project info 시각톤을 맞추고, 모든 pane 컨테이너에 rounded border를 강제 적용한다.
- 2026-03-07: project 탭은 grid 카드 + 생성 모달 구조를 기본으로 하며, 카드에 type 라벨/상태 태그/폴더 아이콘/큰 제목을 표시하고 `project_type(story|movie|code|mono)` 필드를 기본 `code`로 유지한다.
- 2026-03-07: web navbar는 좌측에 현재 선택 프로젝트 표시, 우측에 project/detail 탭 버튼을 두고, 카드형 border 대신 하단 underline(border-b)만 사용한다.
- 2026-03-07: `current.png` 요청은 검색 없이 `/mnt/c/Users/tende/Pictures/Screenshots/current.png`를 먼저 연다. 해당 경로 미존재 시 그 경로 부재만 즉시 보고하고 대체 경로를 요청한다.
- 2026-03-07: 사용자가 개선사항에 `전부`, `모두`, `전체`를 명시하면 부분 개선 보고를 금지하고, 경고/실패가 남지 않을 때까지 연속으로 수정-검증을 반복한 뒤 최종 결과만 보고한다.
- 2026-03-08: templates asset 모달은 좌측 `PROMPTS/TEMPLATES` 폴더 섹션(접기/펼치기) + 우측 파일 내용 패널 구조를 유지하고, 파일 저장 시 `{수정 파일 경로} 수정 반영 후 관련 항목 전체 갱신` LLM 요청을 자동 실행한다.
- 2026-03-08: detail pane의 add/build 흐름은 `form_add_input` 모달 기반으로 유지한다. add 확인 시 `orc create_job_md` 후 `orc add_orc_drafts`를 실행해 `drafts.yaml`를 갱신하며, build는 `orc impl_orc_code -> orc check_orc_code` 결과와 `current_job`을 project 카드에 반영한다.
- 2026-03-09: tmux pane 명령 전송은 기본 셸을 `fish -ic`로 고정하고 `bash -lc`/`bash -ic` 래퍼 생성을 금지한다(사용자가 bash를 명시한 경우만 예외).
- 2026-03-10: repo 루트의 규칙 파일은 `AGENTS.md` 하나로 유지한다. `/home/tree/project/rust-orc` 아래에 `AGENTS.override` 또는 `AGENTS.override.md`를 새로 만들거나 심볼릭 링크로 생성하지 않는다.
- 2026-03-10: repo 전용 규칙 추가는 `AGENTS.md`에 직접 병합하고, 전역 사용자 동작 규칙만 `/home/tree/ai/codex/AGENTS.override.md`에 기록한다.
