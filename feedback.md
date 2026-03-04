# feedback

# 문제
- `~/temp` 1차 시도에서 출력 파일(`project.md`, `src/*`, `package.json`)은 생성됐지만 `.project/feature/*` draft는 생성되지 않았다.
- 즉, 정지 지점은 UI 갱신 문제가 아니라 `create_code_draft` 이전 단계다.
- 추가로 `project.md`에서 `func_xxxxxxxx`/`TODO`가 추출되며 `drafts_list.yaml` 키가 비정상(`func_*`, `t_o_d_o`)으로 오염된다.

# 해결책
- LLM 실행 경로는 유지하고 재시도 정책 강화(공통 retry, dangerous 플래그 누락 방지).
- `project.md` 동기화 추출 로직 보강:
  - `func_xxxxxxxx: 설명` 형태는 설명 본문을 키 후보로 사용.
  - `TODO` placeholder는 planned/features 동기화에서 제외.

# 개선할 수 있는 점
- `project.md` 템플릿 출력 직후 schema 검사에서 `func_*`/`TODO` 금지 규칙을 추가해, 동기화 전에 차단할 수 있다.
- 다음 반복에서 필요 시 `action_validate_project_md_format`에 해당 검사 규칙을 추가한다.
## 2026-03-03 failure log
- 문제: `build-function-auto` 실행 시 `generated draft yaml invalid: task[0]: rule[..] is not auto-verifiable` 오류로 중단됨.
- 원인: LLM이 생성한 `draft.rule`에 자동검증 연산자/구조가 없는 자연어/표현식이 포함됨.
- 해결: draft YAML 정규화 단계에서 `rule` 항목을 문자열로 평탄화하고, auto-verifiable 규칙이 아니면 `check: <rule> should hold` 형태로 강제 변환.
- 재시도: `cargo run --bin orc -- build-function-auto` 실행으로 동일 경로 재검증.
## 2026-03-03 failure log
- 문제: `build-function-auto`가 `parallel::run_parallel_todo` 단계에서 `.project/project.md` 누락으로 실패.
- 원인: 현재 워크스페이스에 `.project`는 있으나 `project.md`가 없어 project_info 로딩이 중단됨.
- 해결: `build-function-auto` 경로에서 실행 전 `.project/project.md` 존재를 보장하고, 없으면 최소 기본 문서를 자동 생성.
- 재시도: `cargo run --bin orc -- build-function-auto` 동일 경로 재실행.
## 2026-03-03 failure fix
- 적용: `build-function-auto` 시작 시 `.project/project.md`가 없으면 `assets/code/templates/project.md`로 자동 생성하도록 보강.
- 기대: `parallel::run_parallel_todo`의 project info 로딩 실패 방지.

## 2026-03-03 chat command validation failure
- cause: `cargo run --bin orc -- chat -n does-not-exist-room` failed because `.temp/does-not-exist-room.yaml` does not exist (expected by spec: no auto-create).
- retry: create a valid room yaml fixture under `.temp/` and rerun `orc chat` path with existing room to verify success.

## 2026-03-03 background-chat compile failure
- cause: `chat --watch` 분기 추가 후 `last_read_message_id`를 watch 루프로 move한 뒤 interactive 분기에서 재사용해 소유권 오류(E0382) 발생.
- retry: watch 분기 호출 시 `last_read_message_id.clone()` 전달로 move를 방지하고 `cargo test` 동일 경로를 재실행.

## 2026-03-03 run_parallel_test write-race failure
- cause: 10 background workers wrote `.temp/test.yaml` concurrently via `orc chat`, causing lost updates (read-modify-write race) and `chat-wait -c 10` hang.
- retry: add room-level lock in chat send path to serialize yaml writes, then rerun `cargo run --bin orc -- run_parallel_test`.

## 2026-03-03 run_parallel_test failure fix
- apply: room-level lock (`.temp/<room>.lock`) added in chat send path to serialize YAML updates.
- result: `cargo run --bin orc -- run_parallel_test` completed with 10/10 reactions.

## entry-1772646602
- status: failed
- summary: auto_code_message failed (attempt 1/3)
- detail: error: init_code_project failed: timeout after 420s | current_message: astro zustand gsap을 이용한 scroll event가 들어간 학교 랜딩페이지를 만들어줘. 반응형 디자인을 지원해야하고 shadcn을 사용해야해, 그리고 Nav 메뉴에는 main, ask 메뉴가 있어야하고 ask, main페이지는 각각 별개로 들어가야해. | next_attempt_message: 타임아웃 방지를 위해 최소 범위로 먼저 완성해줘: Astro + shadcn + Zustand + GSAP 기반 반응형 학교 랜딩페이지를 구현하되 Nav는 main/ask 2개만 두고 `/main`, `/ask`를 별도 라우트로 분리, GSAP 스크롤 이벤트는 main에 핵심 섹션 1개만 적용, 불필요한 에셋/애니메이션/설정 확장은 제외하고 실행 가능한 최소 코드부터 만들어줘.

## 2026-03-05 tmux pane behavior failure log
- cause: tmux worker pane이 상하 분할로 열리고 pane 출력이 보이지 않는 현상이 재현됨.
- analysis: 실행 바이너리 재빌드 전 구버전 경로가 호출되어 `split-window -P -F ...` 형태(분할 방향/타깃 미고정)로 동작했고, pane 시작 로그가 없어 무출력처럼 보임.
- retry: `cargo build`로 바이너리 재빌드 후 동일 경로(`orc init_code_project -a ...` in tmux)를 재실행해 `split-window -h -t <pane>` 호출 및 pane 출력 표시를 확인.
