## 2026-02-21 - 작업한일
- `.project/project.md`의 `features`에 UI 상호작용 기능(3-패널 포커스/활성 상태 전환, esc 2회 정책)을 추가하고 개수를 6개로 갱신함.

## 2026-02-21 - 작업한일
- `plan-drafts-code` 기반으로 `.project/drafts_list.yaml` 생성 및 feature별 draft 6종(`projectCliLifecycle`, `draftLifecycle`, `taskPlanning`, `parallelRun`, `workspaceUi`, `failureLogging`)을 `.project/feature/*/draft.yaml`에 추가함.

## 2026-02-21 - 작업한일
- `project.md`에 언어/스택 기반 초기화 feature를 추가하고(`7번`), 이에 대응하는 draft `projectBootstrap`을 `.project/feature/projectBootstrap/draft.yaml`로 추가함.

## 2026-02-21 - 작업한일
- `projectBootstrap` draft를 구현하여 `Cargo.toml`, `src/main.rs`, `src/config/mod.rs`를 생성했고, `cargo check` 통과 후 draft 폴더를 `.project/clear/projectBootstrap`으로 이동 및 `drafts_list.yaml`의 planned에서 제거함.

## 2026-02-21 - 작업한일
- `projectCliLifecycle` draft를 구현하여 `configs/project.yaml`, `src/ui/mod.rs`, `src/main.rs`에 CLI(list/create/add/select/delete) 및 프로젝트 registry 로직을 추가하고, `cargo check` 통과 후 `.project/clear/projectCliLifecycle`로 이동 및 planned에서 제거함.

## 2026-02-21 - 작업한일
- `draftLifecycle` draft를 구현하여 `src/ui/mod.rs`에 draft 생성/추가/삭제 유틸을 추가하고 `src/assets/templates/draft.yaml`, `.project/features/work/.keep`를 생성한 뒤 `cargo check` 통과 후 `.project/clear/draftLifecycle`로 이동 및 planned에서 제거함.

## 2026-02-21 - 작업한일
- `taskPlanning` draft를 구현하여 `src/main.rs`에 draft task 파싱/의존성 검증(`validate-tasks`) 로직을 추가하고 `src/ui/mod.rs`에 runnable/blocked 출력 함수를 추가한 뒤 `cargo check` 통과 후 `.project/clear/taskPlanning`으로 이동 및 planned에서 제거함.

## 2026-02-21 - 작업한일
- `parallelRun` draft를 구현하여 `src/main.rs`에 `run-parallel`(tokio semaphore 기반 병렬 실행, auto-yes/dry-run 설정 반영) 로직을 추가하고 `src/assets/templates/prompts/tasks.txt`를 생성한 뒤 `cargo check` 통과 후 `.project/clear/parallelRun`으로 이동 및 planned에서 제거함.

## 2026-02-21 - 작업한일
- `workspaceUi` draft를 구현하여 `src/ui/mod.rs`에 pane 포커스 이동/enter-esc 상태전이/esc 2회 정책/task runtime status 렌더 함수를 추가하고 `src/assets/style/pane_style.yaml`을 생성한 뒤 `cargo check` 통과 후 `.project/clear/workspaceUi`로 이동 및 planned에서 제거함.

## 2026-02-21 - 작업한일
- `failureLogging` draft를 구현하여 `src/main.rs`에 실패/timeout 판정 및 `.project/log.md` append 로직을 추가하고 `src/config/mod.rs`에 `performance.timeout_sec` 설정을 확장했으며, `cargo check` 통과 후 `.project/clear/failureLogging`로 이동 및 planned를 비움.

## 2026-02-21 - 작업한일
- CLI 실행 바이너리를 `rust-orchestra`와 `orc` 두 이름으로 모두 제공하도록 `Cargo.toml`에 `[[bin]]` 항목을 추가하고, `src/main.rs`의 usage 출력이 실행한 바이너리명(`argv[0]`)을 반영하도록 수정한 뒤 `cargo test`로 동작 검증함.

## 2026-02-21 - 작업한일
- `plan-drafts-code` 기반으로 `draftCodexFlow`, `parallelBuildCode` draft를 생성하고, `buil-code-parallel` 방식으로 순차 구현하여 `src/main.rs`에 draft-create/add/delete(tmux codex 전송, y/n 삭제 확인), `run-parallel-build-code`(depends_on 게이팅, tokio max_parallel, 상태 모달 렌더, 실패 로그)를 추가했으며 `src/config/mod.rs`, `src/ui/mod.rs`, `config.yaml`, `src/assets/templates/prompts/tasks.txt`를 갱신한 뒤 `cargo test`를 통과시킴.

## 2026-02-21 - 작업한일
- `orc add-func` 대화형 명령을 추가해 LLM이 생성한 질문을 순차 질의하고, 답변 + `.project/project.md`의 `# info`/`## rule`을 기반으로 새 draft yaml을 `.project/feature/<feature>/draft.yaml`에 생성하며 `.project/drafts_list.yaml`의 `planned`를 갱신하도록 `src/main.rs`를 확장하고 `cargo test`를 통과시킴.

## 2026-02-21 - 작업한일
- `src/main.rs`에 `help`/`-h`/`--help` 인자 감지(`calc_is_help_command`)를 추가해 사용 가능한 CLI 명령 목록을 에러 없이 출력하도록 개선하고, `cargo test` 및 `cargo run --bin orc -- --help`, `cargo run --bin orc -- help`로 동작 검증함.

## 2026-02-21 - 작업한일
- CLI 관련 함수들을 `src/cli/mod.rs`로 분리해(`calc_program_name`, `calc_is_help_command`, `flow_print_usage`, `flow_execute_cli`) `src/main.rs`는 모듈 호출만 담당하도록 정리하고, `cargo test` 및 `cargo run --bin orc -- help`로 검증함.

## 2026-02-21 - 작업한일
- `orc init [llm]`, `orc plan-project [llm]`, `orc detail-project [llm]` 명령을 추가해 현재 디렉터리명 기반 프로젝트 초기화와 대화형 입력(name/description/spec/goal/rule, feature 보강 힌트)을 받아 LLM(기본 `codex`)이 `.project/project.md` 초안/상세화를 생성하도록 `src/main.rs`와 `src/cli/mod.rs`를 확장하고, `cargo test` 및 `cargo run --bin orc -- help`로 검증함.

## 2026-02-21 - 작업한일
- 병렬 codex 실행 시 dangerous 인수를 토글할 수 있도록 `config.yaml`/`src/config/mod.rs`/`src/main.rs`를 수정해 `dangerous_bypass` 설정(기본 true)으로 `--dangerously-bypass-approvals-and-sandbox` 전달 여부를 제어하고, 기존 `auto_yes`도 병렬 실행 인수에 실제 반영되도록 개선했으며 `cargo test` 통과를 확인함.

## 2026-02-21 - 작업한일
- `detail-project`가 텍스트 하드코딩 프롬프트 대신 `assets/templates/project.md`와 `assets/prompt/detail-project.txt`를 읽어 LLM에게 전달하도록 `src/main.rs`를 수정하고, 보강 대상 중심 입력으로 동작을 단순화했으며 `cargo test`/`orc help`로 검증함.

## 2026-02-21 - 작업한일
- `README.md`를 추가해 `orc` 실행 명령과 주요 커맨드(영문), `orc ui`, `orc tsend` 사용법을 문서화함.
- `orc ui` 명령을 추가하고 `src/ui/mod.rs`에 2탭 TUI(상단 현재 탭, 하단 단축키/상태바, 상세탭 3개 pane: project/draft feature/parallel runtime)를 구현했으며, 초기 활성 pane은 project로 설정함.
- pane 활성/비활성 border 색을 `configs/style.yaml`의 `active`/`inactive` 값으로 읽어 적용하도록 추가함(미존재 시 기존 스타일 파일 fallback).
- `src/tmux/mod.rs`를 신설하고 `orc tsend <pane_id> <msg...> [enter|raw]` 명령을 추가해 tmux pane 메시지 전송을 지원함.

## 2026-02-22 - 작업한일
- `src/ui/mod.rs`를 개선해 Project 탭을 `ratatui::widgets::Table` 기반으로 렌더링하고, Detail 탭에서 선택 프로젝트 정보가 즉시 보이도록 유지함.
- 하단 상태바를 명령어 바 형태로 확장해 현재 UI에서 사용 가능한 단축키/동작을 항상 표시하도록 수정함.
- `a` 키로 열리는 프로젝트 생성 모달을 추가하고, 이름(기본: 상위 폴더), 설명, 프로젝트 경로(기본: 현재 경로) 3개 입력을 지원함.
- Confirm Pane 래퍼(`확인/취소`)를 도입해 생성 모달의 최종 승인/취소를 공통 pane 동작으로 처리하고, 확인 시 실제 프로젝트 디렉터리/`.project` 생성 및 registry 반영(선택 상태 포함)되도록 연결함.
- `src/main.rs`의 `flow_ui`를 갱신해 UI에서 변경된 프로젝트/선택 상태를 종료 시 `configs/project.yaml`에 저장하도록 반영함.

## 2026-02-22 - 작업한일
- UI의 `q` 동작을 공통화해 현재 focus를 먼저 inactive로 전환하고, 모든 메뉴가 inactive 상태에서 다시 `q` 입력 시 UI가 종료되도록 수정함(`enter`로 재활성).
- Project 탭에서 `m` 키로 auto mode를 트리거할 수 있게 하고, 선택된 프로젝트 기준으로 auto mode 요청을 `main`으로 전달하도록 `UiRunResult`를 확장함.
- `src/main.rs`에 `flow_auto_mode`를 추가해 tmux 활성 여부 검사(미활성 시 경고/종료), 현재 pane 이름을 `plan`으로 변경, codex 자동 실행(유사 앱 웹 검색+기능 선정+draft/구현), `cargo test` 통과 후 `jj commit` 순서로 처리하도록 구현함.
- CLI에 `orc auto [project_name]` 명령을 추가해 UI 외부에서도 동일 auto mode를 직접 실행할 수 있도록 함.
- `src/tmux/mod.rs`에 현재 pane 조회/rename 함수를 추가해 auto mode의 tmux 제어를 모듈 함수로 분리함.

## 2026-02-22 - 작업한일
- UI 상단 탭 표시를 `Tabs` 위젯에서 단일 pane 헤더(`Current Pane`) 방식으로 변경하고, `Project | Detail` 텍스트만 노출되도록 수정함.
- 활성 pane 이름은 active 색/굵게, 비활성 pane 이름은 inactive 색으로 렌더링되도록 스타일 적용함.

## 2026-02-22 - 작업한일
- `codex exec` 호출 경로에 `--dangerously-bypass-approvals-and-sandbox`를 기본 추가해 승인/샌드박스 프롬프트를 우회하도록 반영함.
- 대상: tmux 신규 pane 전송 명령, 동기 codex 실행, 특정 디렉터리 codex 실행(`src/main.rs`).
- `-y`는 유지해 대화형 확인도 자동 승인되도록 구성함.

## 2026-02-22 - 작업한일
- `plan-project` 입력을 멀티라인 붙여넣기 친화적으로 개선: description/spec/goal/rule 입력을 빈 줄 종료 방식으로 변경해 줄바꿈 포함 텍스트가 다음 프롬프트로 흘러가지 않게 수정함.
- rule 파싱은 `;`와 줄바꿈 둘 다 분리자로 처리하도록 확장함.

## 2026-02-22 - 작업한일
- CLI 서브커맨드 네이밍을 동사-명사 형태로 개편함: list/create/add/select/delete project, create/add/delete draft, add-function, open-ui, run-auto, send-tmux, run-build-parallel.
- `--help`와 README를 새 명령 체계로 업데이트하고, 기존 명령어는 하위호환 alias로 유지해 즉시 사용 중단 없이 전환 가능하게 함.

## 2026-02-22 - 작업한일
- 병렬 빌드 명령의 기본 네이밍을 `build-parallel-code`(동사-형용사-명사)로 변경함.
- 기존 `run-build-parallel`, `run-parallel-build-code`, `run-parallel`은 하위호환 alias로 유지함.
- `--help`와 README 명령 목록을 새 기본 명령명으로 동기화함.

## 2026-02-22 - 작업한일
- 설치된 전역 `orc` 바이너리를 최신 소스 기준으로 교체(`cargo install --path . --bin orc --force`)해 PATH의 `orc --help`가 최신 명령 체계를 출력하도록 맞춤.
- 리포지토리 루트 `AGENTS.md`를 추가하고, CLI 명령/파일명 변경 시 `src/cli/mod.rs`, `README.md`, 기타 문서 예시를 같은 변경에서 동기화해야 한다는 규칙을 명시함.

## 2026-02-22 - 작업한일
- `orc plan-init` 명령을 추가하고 인자 전달(`-n` name, `-d` description, `-s` spec, `--llm`)을 지원하도록 CLI 파서를 확장함.
- `plan-project`는 내부적으로 `plan-init` 로직을 재사용하도록 정리했으며, 인자 미지정 항목은 기존처럼 대화형 입력으로 보완되게 유지함.
- `--help`와 `README.md`에 `plan-init` 사용법을 동기화함.

## 2026-02-22 - 작업한일
- `create-draft`를 단건 feature 인자 방식에서 전체 feature 기반 방식으로 전환함: `.project/project.md`의 `## features`를 기준으로 `plan-drafts-code`를 호출하도록 `flow_draft_create`를 변경함.
- 특정 feature 추가/수정은 `add-draft <feature_name> [request]`로만 처리되도록 역할을 분리함.
- CLI/문서 동기화: `src/cli/mod.rs` help를 `create-draft`(무인자)로 변경하고, 인자 전달 시 에러를 반환하게 했으며 `README.md`도 동일하게 갱신함.

## 2026-02-22 - 작업한일
- 병렬 실행 관련 legacy alias(`run-build-parallel`, `run-parallel-build-code`, `run-parallel`)를 CLI 파서와 help에서 제거하고 `build-parallel-code`만 공식 명령으로 유지함.
- README 명령 목록에서 legacy alias 표기를 제거해 help/문서 동기화를 맞춤.
- 전역 설치본 `orc`를 최신으로 재설치해(PATH) alias 제거 결과가 즉시 반영되도록 정합화함.

## 2026-02-22 - 작업한일
- `build-parallel-code` 실행 직전에 현재 작업 폴더가 비어있는지 검사하도록 추가함.
- 폴더가 비어있으면 `.project/feature`, `.project/clear`, `.project/project.md`, `.project/drafts_list.yaml`을 자동 초기화한 뒤 병렬 처리 흐름으로 진행하도록 구현함(`src/main.rs`).
- 빈 폴더 검증에서 초기화 메시지 출력 후 `no feature draft to run`까지 정상 동작함을 확인함.

## 2026-02-22 - 작업한일
- `src/ui/mod.rs`의 Detail 탭 3개 pane에 활성 전환 tween 효과를 추가해, focus가 활성화되거나 좌우 이동될 때 pane margin이 짧게 `2→1→0`으로 줄어들며 커지는 애니메이션처럼 보이도록 구현함.
