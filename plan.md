## 문제
- 글로벌 AGENTS 규칙에 `nf` 강제 문구가 중복되어 작업 종료 동작 혼선이 발생한다.
- 일부 skill 문서에 레거시 `.project/feature/*/draft.yaml` 경로가 남아 있다.
- fish 함수 가이드에 `vim` 실행 안내가 남아 있어 종료 후 편집기 실행 오해를 유발한다.

## 해결책
- `/home/tree/.codex/AGENTS.override.md`에서 `nf` 강제 문구를 정리하고 현재 정책과 충돌 없는 형태로 수정한다.
- `/home/tree/ai/skills/*/SKILL.md`에서 레거시 `draft.yaml` 경로 문구를 `drafts.yaml` 기준으로 교체한다.
- `/home/tree/.config/fish/functions/level_init.fish`의 `vim` 안내 문구를 제거/대체한다.
- 변경 사항을 `./.agents/log.md`에 기록한다.

## 검증
- 관련 문자열 검색으로 변경 반영 확인.
- `cargo test -q` 통과.

## 문제 (2026-03-05 virtual-scenario)
- `if)` 가상 시나리오 응답 포맷이 사용자 요구(`a -> b` 줄바꿈 고정)와 다를 수 있다.

## 해결책 (2026-03-05 virtual-scenario)
- `AGENTS.override.md`에 가상 시나리오 출력 포맷 강제 규칙을 추가한다.
- `/home/tree/ai/skills/virtual-scenario/SKILL.md`의 Mandatory Output/예시를 `a -> b` 줄 형식으로 수정한다.
- 동일 포맷으로 `if) auto react todo` 실행 흐름을 다시 작성한다.

## 검증 (2026-03-05 virtual-scenario)
- `rg`로 `a -> b` 규칙 문구와 예시 반영 여부를 확인한다.
- `.project/scenario.md`를 생성/갱신해 동일 포맷이 기록되었는지 확인한다.

## 문제 (2026-03-05 tmux manager workflow)
- `orc-cli-workflow` 스킬에 매니저 pane이 단계별 워커 pane을 열고 `send-tmux`로 위임/회수하는 상세 절차가 부족하다.
- 사용자가 `"~~~을 만들어줘"`를 입력했을 때 `auto -> plan -> drafts -> impl -> check_draft` 순차 위임 규칙이 문서화되어 있지 않다.

## 해결책 (2026-03-05 tmux manager workflow)
- `/home/tree/ai/skills/orc-cli-workflow/SKILL.md`에 매니저-워커 오케스트레이션 규칙을 추가한다.
- 트리거 문구(`"~~~을 만들어줘"`) 처리 규칙, 단계별 워커 pane 생성, `orc send-tmux` 전송, 완료 회수, 실패 시 재시도 판단 루프를 명시한다.
- 단계별 예시 명령(`auto`, `init/add_code_plan`, `add_code_draft`, `impl_code_draft`, `check_draft`)을 같은 방식으로 통일한다.

## 검증 (2026-03-05 tmux manager workflow)
- `rg`로 스킬 문서에 `send-tmux`, `tmux split-window`, `manager pane`, `check_draft` 단계가 모두 반영됐는지 확인한다.
- `cargo test -q`를 실행해 저장소 상태 검증을 통과한다.

## 문제 (2026-03-05 tmux trigger branching)
- `orc-cli-workflow` 스킬은 `~~~을 만들어줘`만 명시되어 있고 `~~~을 추가해줘` 트리거가 빠져 있다.
- `~을 읽고 처리해줘` 요청에서 `input.md`를 재생성하지 않고 기존 파일을 읽는 경로가 문서화되어 있지 않다.

## 해결책 (2026-03-05 tmux trigger branching)
- 스킬 문서 트리거를 3종(`만들어줘`, `추가해줘`, `읽고 처리해줘`)으로 확장한다.
- `읽고 처리해줘` 분기는 `create_input_md`를 금지하고 `add_code_plan -f -> add_code_draft -f -> impl_code_draft -> check_draft` 경로를 명시한다.
- manager pane 단계별 워커 위임 표에 트리거별 실행 명령을 분기 형태로 추가한다.

## 검증 (2026-03-05 tmux trigger branching)
- `rg`로 스킬 문서에 `추가해줘`, `읽고 처리해줘`, `create_input_md` 금지, `-f` 경로 반영을 확인한다.
- `cargo test -q` 실행 결과를 기록한다.
