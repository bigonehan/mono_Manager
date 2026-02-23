## 2026-02-21 - 작업한일
- `.project/project.md`의 `features`에 UI 상호작용 기능(3-패널 포커스/활성 상태 전환, esc 2회 정책)을 추가하고 개수를 6개로 갱신함.

- `plan-drafts-code` 기반으로 `.project/drafts_list.yaml` 생성 및 feature별 draft 6종(`projectCliLifecycle`, `draftLifecycle`, `taskPlanning`, `parallelRun`, `workspaceUi`, `failureLogging`)을 `.project/feature/*/draft.yaml`에 추가함.

- `project.md`에 언어/스택 기반 초기화 feature를 추가하고(`7번`), 이에 대응하는 draft `projectBootstrap`을 `.project/feature/projectBootstrap/draft.yaml`로 추가함.

- `projectBootstrap` draft를 구현하여 `Cargo.toml`, `src/main.rs`, `src/config/mod.rs`를 생성했고, `cargo check` 통과 후 draft 폴더를 `.project/clear/projectBootstrap`으로 이동 및 `drafts_list.yaml`의 planned에서 제거함.

- `projectCliLifecycle` draft를 구현하여 `configs/project.yaml`, `src/ui/mod.rs`, `src/main.rs`에 CLI(list/create/add/select/delete) 및 프로젝트 registry 로직을 추가하고, `cargo check` 통과 후 `.project/clear/projectCliLifecycle`로 이동 및 planned에서 제거함.

- `draftLifecycle` draft를 구현하여 `src/ui/mod.rs`에 draft 생성/추가/삭제 유틸을 추가하고 `src/assets/templates/draft.yaml`, `.project/features/work/.keep`를 생성한 뒤 `cargo check` 통과 후 `.project/clear/draftLifecycle`로 이동 및 planned에서 제거함.

- `taskPlanning` draft를 구현하여 `src/main.rs`에 draft task 파싱/의존성 검증(`validate-tasks`) 로직을 추가하고 `src/ui/mod.rs`에 runnable/blocked 출력 함수를 추가한 뒤 `cargo check` 통과 후 `.project/clear/taskPlanning`으로 이동 및 planned에서 제거함.

- `parallelRun` draft를 구현하여 `src/main.rs`에 `run-parallel`(tokio semaphore 기반 병렬 실행, auto-yes/dry-run 설정 반영) 로직을 추가하고 `src/assets/templates/prompts/tasks.txt`를 생성한 뒤 `cargo check` 통과 후 `.project/clear/parallelRun`으로 이동 및 planned에서 제거함.

- `workspaceUi` draft를 구현하여 `src/ui/mod.rs`에 pane 포커스 이동/enter-esc 상태전이/esc 2회 정책/task runtime status 렌더 함수를 추가하고 `src/assets/style/pane_style.yaml`을 생성한 뒤 `cargo check` 통과 후 `.project/clear/workspaceUi`로 이동 및 planned에서 제거함.

- `failureLogging` draft를 구현하여 `src/main.rs`에 실패/timeout 판정 및 `.project/log.md` append 로직을 추가하고 `src/config/mod.rs`에 `performance.timeout_sec` 설정을 확장했으며, `cargo check` 통과 후 `.project/clear/failureLogging`로 이동 및 planned를 비움.

- CLI 실행 바이너리를 `rust-orchestra`와 `orc` 두 이름으로 모두 제공하도록 `Cargo.toml`에 `[[bin]]` 항목을 추가하고, `src/main.rs`의 usage 출력이 실행한 바이너리명(`argv[0]`)을 반영하도록 수정한 뒤 `cargo test`로 동작 검증함.

- `plan-drafts-code` 기반으로 `draftCodexFlow`, `parallelBuildCode` draft를 생성하고, `buil-code-parallel` 방식으로 순차 구현하여 `src/main.rs`에 draft-create/add/delete(tmux codex 전송, y/n 삭제 확인), `run-parallel-build-code`(depends_on 게이팅, tokio max_parallel, 상태 모달 렌더, 실패 로그)를 추가했으며 `src/config/mod.rs`, `src/ui/mod.rs`, `config.yaml`, `src/assets/templates/prompts/tasks.txt`를 갱신한 뒤 `cargo test`를 통과시킴.

- `orc add-func` 대화형 명령을 추가해 LLM이 생성한 질문을 순차 질의하고, 답변 + `.project/project.md`의 `# info`/`## rule`을 기반으로 새 draft yaml을 `.project/feature/<feature>/draft.yaml`에 생성하며 `.project/drafts_list.yaml`의 `planned`를 갱신하도록 `src/main.rs`를 확장하고 `cargo test`를 통과시킴.

- `src/main.rs`에 `help`/`-h`/`--help` 인자 감지(`calc_is_help_command`)를 추가해 사용 가능한 CLI 명령 목록을 에러 없이 출력하도록 개선하고, `cargo test` 및 `cargo run --bin orc -- --help`, `cargo run --bin orc -- help`로 동작 검증함.

- CLI 관련 함수들을 `src/cli/mod.rs`로 분리해(`calc_program_name`, `calc_is_help_command`, `flow_print_usage`, `flow_execute_cli`) `src/main.rs`는 모듈 호출만 담당하도록 정리하고, `cargo test` 및 `cargo run --bin orc -- help`로 검증함.

- `orc init [llm]`, `orc plan-project [llm]`, `orc detail-project [llm]` 명령을 추가해 현재 디렉터리명 기반 프로젝트 초기화와 대화형 입력(name/description/spec/goal/rule, feature 보강 힌트)을 받아 LLM(기본 `codex`)이 `.project/project.md` 초안/상세화를 생성하도록 `src/main.rs`와 `src/cli/mod.rs`를 확장하고, `cargo test` 및 `cargo run --bin orc -- help`로 검증함.

- 병렬 codex 실행 시 dangerous 인수를 토글할 수 있도록 `config.yaml`/`src/config/mod.rs`/`src/main.rs`를 수정해 `dangerous_bypass` 설정(기본 true)으로 `--dangerously-bypass-approvals-and-sandbox` 전달 여부를 제어하고, 기존 `auto_yes`도 병렬 실행 인수에 실제 반영되도록 개선했으며 `cargo test` 통과를 확인함.

- `detail-project`가 텍스트 하드코딩 프롬프트 대신 `assets/templates/project.md`와 `assets/prompt/detail-project.txt`를 읽어 LLM에게 전달하도록 `src/main.rs`를 수정하고, 보강 대상 중심 입력으로 동작을 단순화했으며 `cargo test`/`orc help`로 검증함.

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

- UI의 `q` 동작을 공통화해 현재 focus를 먼저 inactive로 전환하고, 모든 메뉴가 inactive 상태에서 다시 `q` 입력 시 UI가 종료되도록 수정함(`enter`로 재활성).
- Project 탭에서 `m` 키로 auto mode를 트리거할 수 있게 하고, 선택된 프로젝트 기준으로 auto mode 요청을 `main`으로 전달하도록 `UiRunResult`를 확장함.
- `src/main.rs`에 `flow_auto_mode`를 추가해 tmux 활성 여부 검사(미활성 시 경고/종료), 현재 pane 이름을 `plan`으로 변경, codex 자동 실행(유사 앱 웹 검색+기능 선정+draft/구현), `cargo test` 통과 후 `jj commit` 순서로 처리하도록 구현함.
- CLI에 `orc auto [project_name]` 명령을 추가해 UI 외부에서도 동일 auto mode를 직접 실행할 수 있도록 함.
- `src/tmux/mod.rs`에 현재 pane 조회/rename 함수를 추가해 auto mode의 tmux 제어를 모듈 함수로 분리함.

- UI 상단 탭 표시를 `Tabs` 위젯에서 단일 pane 헤더(`Current Pane`) 방식으로 변경하고, `Project | Detail` 텍스트만 노출되도록 수정함.
- 활성 pane 이름은 active 색/굵게, 비활성 pane 이름은 inactive 색으로 렌더링되도록 스타일 적용함.

- `codex exec` 호출 경로에 `--dangerously-bypass-approvals-and-sandbox`를 기본 추가해 승인/샌드박스 프롬프트를 우회하도록 반영함.
- 대상: tmux 신규 pane 전송 명령, 동기 codex 실행, 특정 디렉터리 codex 실행(`src/main.rs`).
- `-y`는 유지해 대화형 확인도 자동 승인되도록 구성함.

- `plan-project` 입력을 멀티라인 붙여넣기 친화적으로 개선: description/spec/goal/rule 입력을 빈 줄 종료 방식으로 변경해 줄바꿈 포함 텍스트가 다음 프롬프트로 흘러가지 않게 수정함.
- rule 파싱은 `;`와 줄바꿈 둘 다 분리자로 처리하도록 확장함.

- CLI 서브커맨드 네이밍을 동사-명사 형태로 개편함: list/create/add/select/delete project, create/add/delete draft, add-function, open-ui, run-auto, send-tmux, run-build-parallel.
- `--help`와 README를 새 명령 체계로 업데이트하고, 기존 명령어는 하위호환 alias로 유지해 즉시 사용 중단 없이 전환 가능하게 함.

- 병렬 빌드 명령의 기본 네이밍을 `build-parallel-code`(동사-형용사-명사)로 변경함.
- 기존 `run-build-parallel`, `run-parallel-build-code`, `run-parallel`은 하위호환 alias로 유지함.
- `--help`와 README 명령 목록을 새 기본 명령명으로 동기화함.

- 설치된 전역 `orc` 바이너리를 최신 소스 기준으로 교체(`cargo install --path . --bin orc --force`)해 PATH의 `orc --help`가 최신 명령 체계를 출력하도록 맞춤.
- 리포지토리 루트 `AGENTS.md`를 추가하고, CLI 명령/파일명 변경 시 `src/cli/mod.rs`, `README.md`, 기타 문서 예시를 같은 변경에서 동기화해야 한다는 규칙을 명시함.

- `orc plan-init` 명령을 추가하고 인자 전달(`-n` name, `-d` description, `-s` spec, `--llm`)을 지원하도록 CLI 파서를 확장함.
- `plan-project`는 내부적으로 `plan-init` 로직을 재사용하도록 정리했으며, 인자 미지정 항목은 기존처럼 대화형 입력으로 보완되게 유지함.
- `--help`와 `README.md`에 `plan-init` 사용법을 동기화함.

- `create-draft`를 단건 feature 인자 방식에서 전체 feature 기반 방식으로 전환함: `.project/project.md`의 `## features`를 기준으로 `plan-drafts-code`를 호출하도록 `flow_draft_create`를 변경함.
- 특정 feature 추가/수정은 `add-draft <feature_name> [request]`로만 처리되도록 역할을 분리함.
- CLI/문서 동기화: `src/cli/mod.rs` help를 `create-draft`(무인자)로 변경하고, 인자 전달 시 에러를 반환하게 했으며 `README.md`도 동일하게 갱신함.

- 병렬 실행 관련 legacy alias(`run-build-parallel`, `run-parallel-build-code`, `run-parallel`)를 CLI 파서와 help에서 제거하고 `build-parallel-code`만 공식 명령으로 유지함.
- README 명령 목록에서 legacy alias 표기를 제거해 help/문서 동기화를 맞춤.
- 전역 설치본 `orc`를 최신으로 재설치해(PATH) alias 제거 결과가 즉시 반영되도록 정합화함.

- `build-parallel-code` 실행 직전에 현재 작업 폴더가 비어있는지 검사하도록 추가함.
- 폴더가 비어있으면 `.project/feature`, `.project/clear`, `.project/project.md`, `.project/drafts_list.yaml`을 자동 초기화한 뒤 병렬 처리 흐름으로 진행하도록 구현함(`src/main.rs`).
- 빈 폴더 검증에서 초기화 메시지 출력 후 `no feature draft to run`까지 정상 동작함을 확인함.

- `src/ui/mod.rs`의 Detail 탭 3개 pane에 활성 전환 tween 효과를 추가해, focus가 활성화되거나 좌우 이동될 때 pane margin이 짧게 `2→1→0`으로 줄어들며 커지는 애니메이션처럼 보이도록 구현함.

## 2026-02-23 - 작업한일
- `project list` 출력을 카드형 그리드로 변경해 최대 3x3(9개) 프로젝트를 표시하고, 선택 상태(`*`), 설명, 생성/수정 시각을 카드에 담아 보여주며 초과 항목 개수 안내를 추가함.

- `create project` 모달을 폼 형태로 개편해 `name/description/project path` 라벨과 각 입력칸의 명확한 border를 분리하고, 입력 필드 커서 표시를 추가함.
- 모달 컬러 강조(주황/강조 배경)를 제거하고 기본값 표시를 검은색으로 통일했으며, `description` 기본값을 `프로젝트 설명`으로 설정하고 멀티라인 입력(Enter 개행)을 지원함.
- 단축키 도움말을 모달 내부에서 제거해 하단 상태바로 통합하고, 하단 바 타이틀을 `bar_status`로 변경함.
- 프로젝트 registry 저장 경로를 `configs/Project.yaml`로 전환하고, 기존 `configs/project.yaml` 파일은 읽기 fallback으로 호환되도록 반영함.

- `create-project` 모달의 `Confirm/Cancel`을 우하단 정렬로 배치하고, 현재 편집 중인 입력박스 border를 초록색으로 표시하도록 수정함.
- 모달 입력 커서 표시를 유지하고, 사용되지 않던 `PaneState`/`UiState` 및 관련 전이 함수를 제거함.
- 프로젝트 registry 기본 경로를 `configs/project.yaml`로 설정하고, 기존 `configs/Project.yaml`는 읽기 fallback으로 호환하도록 조정함.

- `Project Select` pane의 프로젝트 목록을 테이블에서 카드형 grid로 변경하고, 카드 본문에는 속성명 없이 값만 표시하도록 수정함.
- 카드 텍스트 계층을 `name=bold`, `description=회색 dim`, `path=dark gray dim`으로 적용해 시각적 우선순위를 강화함.

- `create project` 모달의 `Confirm/Cancel` 버튼을 모달 절대 좌표 기준 우하단(마지막 라인, 우측 정렬)으로 고정 배치함.
- `project path`가 파일로 존재하는 경우 에러를 반환하고, 디렉터리가 없으면 해당 경로를 생성하도록 경로 검증/생성 로직을 보강함.

- `create project` 모달에서 비활성 입력란 값은 옅은색(`dark gray + dim`)으로 렌더링되도록 조정함.
- `Name/Description/Project Path` 라벨 옆 기본값 텍스트(`default: ...`) 표시는 제거하고 라벨만 표시하도록 변경함.

- `create project` 폼의 `project path` 필드에서 포커스 후 타이핑 시 기본값을 지우지 않고, 기본값 뒤에 이어서 입력되도록 입력/백스페이스 처리 로직을 수정함.

- `create project` 폼의 `name` 기본값을 현재 실행 디렉터리명으로 변경하고, 입력 시작 시 기본값을 지우지 않고 뒤에 이어서 입력되도록 수정함.

- `Project Select` 카드 렌더를 최대 3x3(9개) 범위로 제한해 영역 높이/개수가 고정되도록 조정함.
- 카드 내 프로젝트 이름 라인을 검은 배경 + 흰색 굵은 글자로 렌더링하도록 스타일을 변경함.

- 상단 `Current Pane` 헤더에서 `Pane:` 접두어를 제거하고 `Project | Detail`만 표시되도록 수정함.

- `Current Pane` 헤더를 좌/우 영역으로 분리해 우측에 `switch : tab` 안내를 추가함.
- 하단 도움말에서 `tab/1/2 tabs` 표기를 제거하고 `j/k next project`로 문구를 변경함.

- `Project Select` 카드의 `path` 텍스트 색상을 `description`보다 더 흐리게 보이도록 `RGB(70,70,70) + dim` 스타일로 조정함.
- 관련 수정 중 발생한 UI 편집 플로우 컴파일 오류(가변 대여 중복)를 함께 정리해 `cargo test`가 다시 통과하도록 보강함.

- `Project Select` 카드 item의 `Path` 글자색을 기존보다 더 옅게(`RGB 45,45,45 + dim`) 조정함.

- `Project Select` 카드 텍스트 계층 색을 재조정해 `path`가 `description`보다 더 옅게 보이도록 변경함(`description: RGB 130`, `path: RGB 180`, 둘 다 dim).

- `Project` 탭에서 화살표 키(상하좌우)로 `Project Select` 그리드 카드 선택을 이동할 수 있게 하고, 현재 선택 카드를 active 상태로 유지하도록 선택 전이 로직을 추가함.
- `Project` 탭에서 `Enter` 입력 시 `Detail` 탭으로 전환되도록 키 동작을 확장함.
- `Project` 탭의 `m` 키는 선택 프로젝트 편집 모달을 열도록 유지하고, 하단 `bar_status` 키 안내를 화살표 이동/탭 전환 중심 문구로 갱신함.

- registry 저장/로드 경로를 실행 바이너리 기준으로 해석하도록 변경해 `configs/project.yaml`을 실행 위치가 아닌 바이너리 디렉터리 기준으로 읽고 쓰게 수정함(대문자 legacy 파일 fallback 유지).
- `configs`/`assets` 경로 탐색을 바이너리 디렉터리 우선으로 확장해 스타일/템플릿 파일을 잘못된 cwd에서 찾지 않도록 보강함.
- UI `create project` 신규 생성 시 대상 프로젝트 경로에서 현재 바이너리를 `plan-init -n/-d/-s`로 실행하고(goal/rule은 기본 입력), `.project/project.md` 초기 초안이 자동 생성되도록 연결함.

- `Project` 탭에서 `d` 키로 삭제 확인(y/n) 모달을 띄우고, 확인 시 선택 프로젝트 경로의 `.project` 내부 파일/폴더를 전부 삭제한 뒤 registry(`configs/project.yaml`)에서 해당 프로젝트 항목을 제거하도록 구현함.
- 삭제 취소(`n`/`esc`) 처리와 bar_status 단축키 안내(`d: delete-project`)를 함께 반영함.

- `configs/project.yaml` 스키마를 확장해 각 프로젝트에 4자리 영숫자 `id`를 저장하고, 최상위에 `recentActivepane`을 저장하도록 반영함(기존 데이터는 로드 시 누락 id 자동 보정).
- `open-ui` 시작 시 `recentActivepane`에 해당하는 프로젝트 카드를 `Project Select` 목록의 맨 앞으로 배치하도록 정렬 로직을 추가함.
- `Project` 탭에서 `Enter`로 `Detail` 탭 진입 시 현재 프로젝트 `id`를 `recentActivepane`으로 기록해 `project.yaml`에 저장되도록 연결함.

- `config.yaml`/`configs.yaml`에 `ai.model` 기본값(`codex`)을 추가하고, 앱 설정 로더가 `configs.yaml`까지 우선 탐색하도록 확장함.
- `codex exec` 하드코딩 경로를 `ai.model` 기반 실행으로 치환: tmux prompt 전송, codex capture 실행, project dir 실행, parallel build 실행 모델을 공통 설정값으로 동작하게 변경함.
- `plan-init`/`detail-project`의 기본 LLM도 `ai.model` 값을 사용하도록 바꿔, 인자 미지정 시 설정 모델로 실행되게 정리함.

- UI에 `Working` 대기 모달 상태를 추가해 시간이 걸리는 작업(프로젝트 생성/수정 반영, 경로 이동, 삭제 실행) 전에 작업중 메시지를 표시하고 다음 틱에서 실제 작업을 수행하도록 2단계 실행 흐름으로 변경함.
- `pending_action` 큐를 도입해 모달 렌더 후 작업 실행이 이뤄지게 하여, 사용자에게 대기 상태를 명확히 보여주도록 개선함.

- `Project` 생성 완료 직후 `project.md` 상세 보강 여부를 묻는 y/n 확인 모달을 추가하고, `y` 선택 시 AI 대화 모달을 열어 후속 설계 보강 흐름으로 진입하도록 구현함.
- AI 모달은 응답 표시 영역/입력 영역으로 분리하고, 입력은 `Enter` 줄바꿈 + `Enter` 2회 전송 방식으로 동작하게 했으며, 모델 응답을 stdout 스트리밍으로 실시간 렌더링하도록 연결함.
- AI 프롬프트에 `$plan-project-code` 스킬 컨텍스트와 기존 `name/description` 및 현재 `project.md`를 포함해 누락 섹션 보강을 유도하고, 응답에서 markdown 본문 감지 시 `.project/project.md`를 즉시 갱신하도록 추가함.

- y/n/cancel 확인 모달(`path change`, `delete`, `detail fill`)에 공통 버튼 렌더러를 추가해 버튼 라인을 모달 절대 좌표 기준 우하단으로 고정 배치함.

- 확인 모달 공통 래퍼를 추가하고 `create project`/`path change`/`delete`/`detail fill` 대화창을 동일한 confirm/cancel 형식(우하단 버튼, 좌/우 선택, Enter 적용, Esc 자동 취소)으로 통일함.
- `bar_status`의 Enter 안내 문구를 `enter: active`로 단순화함.

- 확인 모달 공통 래퍼의 본문 텍스트를 세로/가로 중앙 정렬 기본값으로 변경해 `Fill Project Detail`, `Project Delete` 포함 모든 y/n 확인창에 동일 적용되도록 정리함.
- `Working` 상태 모달도 본문을 세로/가로 중앙 정렬로 조정함.

- `create/edit project` 입력 폼에서 현재 포커스된 필드만 강조되고, 비활성 필드 border는 `dark gray + dim`으로 옅게 보이도록 조정함.

- AI 채팅 모달에서 응답 완료 직후 bootstrap 확인 모달이 자동으로 열려 입력이 막히던 흐름을 제거해, 응답 후에도 추가 입력을 계속 받을 수 있게 수정함.

- bootstrap 적용 완료 시 `ai_chat_modal` 상태를 정리하고 `Detail` 탭 기본 pane으로 복귀하도록 후처리를 추가해, bootstrap 후 AI detail pane으로 되돌아가지 않게 수정함.
- AI detail 모달에서 첫 전송은 LLM에 기존 컨텍스트를 포함해 보내되, 전송 직후 UI 히스토리는 사용자/AI 대화만 남기도록 정리해 화면이 깔끔하게 유지되도록 변경함.

- AI Detail 모달 진입 시 기존 `System/Context/Current project.md` seed 프롬프트를 먼저 LLM으로 비표시 warmup 전송하고, 완료 후 `Response` 영역을 빈 상태로 시작하도록 흐름을 변경함.
- warmup 중에는 응답 스트림을 `Response`에 렌더링하지 않고, 완료 후 일반 대화 스트림만 표시되도록 분기 처리함.

- 사용자 피드백을 반영해 `AGENTS.md`에 알림 실행 내역을 최종 요약에 노출하지 않는 출력 규칙을 추가함.

- AI 상세 모달 스트리밍 실행에 취소 플래그(`AtomicBool`)를 추가하고, `q`로 UI 종료/AI 모달 Esc/bootstrap 완료 시 LLM 비동기 프로세스가 즉시 kill되도록 종료 훅을 연결함.
- 종료 이벤트(`Cancelled`)를 분리해 스트림 상태(`stream_rx`, `stream_cancel`, `streaming_buffer`)가 안전하게 정리되도록 보강함.

- `create/edit project` 모달에서 현재 입력 중인 필드의 라벨(`Name/Description/Spec/Project Path`)을 `검은 배경 + 흰색 글자`로 강조 표시하도록 UI 스타일을 추가함.

- `create/edit project` 모달 라벨 강조가 적용되지 않던 문제를 수정하기 위해 라벨 렌더를 `Paragraph.style`에서 `Span::styled` 기반으로 전환해, 활성 필드 라벨의 `검은 배경 + 흰 글자` 스타일이 확실히 반영되도록 보강함.

- `~/.codex/skills`의 깨진 심볼릭 링크(`domain_create`, `build-function`, `plan-code`)를 `~/ai/skills`의 실제 폴더(`build-domain`, `add-function`, `plan-project-code`)로 재연결해 Codex skill loader 오류를 해소함.
- `~/ai/skills/build-code-parallel/SKILL.md`의 `name` 오타(`buil-code-parallel`)를 `build-code-parallel`로 수정해 호출명과 정의를 일치시킴.
- `src/main.rs` 템플릿 치환 로직을 보강해 `{{key}}`와 `{{ key }}` 모두 치환하고, 치환 후 미해결 placeholder가 남으면 오류를 반환하도록 검증을 추가함(`detail-project`, `build-parallel-code` 경로).

- 코드/프롬프트 전역 스킬명 참조를 `~/ai/skills` 실제 `name:` 목록과 대조 검사하고, 불일치 1건(`assets/templates/project.md`의 `domain-create`)을 `build_domain`으로 수정해 참조명을 일치시킴.

- AI Detail 모달 입력 영역 우하단에 `대화 종료` 버튼을 추가하고, `Tab/좌우`로 버튼 선택 후 `Enter`로 대화를 종료할 수 있도록 키 처리(기존 `Esc` 종료와 동일 경로)를 구현함.

- `AGENTS.md`에 기능 추가/개선 완료 시 `cargo install --path /home/tree/project/rust-orc`를 자동 실행하도록 `Auto Install Rule`을 추가함.

- AI Detail 모달의 `대화 종료` 버튼 위치를 Input field 내부에서 분리해, AI Detail pane 맨 아래 전용 영역(입력창 밖)으로 이동하도록 레이아웃을 3구역으로 조정함.

- AI Detail 응답 표시를 정리해 스트리밍 중에는 중간 출력(사고/로그)을 렌더링하지 않고 `AI 응답 생성중...`만 보여주도록 변경함.
- LLM 실행의 `stderr`를 UI 스트림으로 노출하지 않도록 `Stdio::null()` 처리해 `[stderr]` 로그가 대화창을 오염시키지 않게 조정함.

- AI Detail 초기 seed 프롬프트를 모달 진입 시 1회만 전송하도록 고정하고(응답은 `READY` 1단어 지시), 이후 사용자 대화 프롬프트에서는 seed를 재삽입하지 않도록 분리해 “첫 명령 시 seed가 전송되는 것처럼 보이는” 문제를 완화함.

- AI Detail pane의 `대화 종료` 버튼 폭 계산을 유니코드 표시폭 기준으로 변경해 버튼 텍스트가 잘리지 않게 수정함.
- AI Detail 입력 상태를 `포커스(Input/CloseButton) + 입력활성/비활성` 상태머신으로 재구성해, `Esc` 비활성화 → `↓` 종료 버튼 포커스 → `↑` 입력 포커스 복귀 → `Enter` 재활성화 흐름을 구현함.
- Input pane border를 상태에 따라 변경(활성=초록 강조, 비활성 포커스=노랑, 비포커스=회색 dim)해 입력 가능 상태를 명확히 표시함.
- AI Detail 초기화/렌더/웜업을 템플릿형 재사용 함수(`action_new_ai_chat_modal_template`, `action_start_ai_chat_warmup`, `action_build_ai_seed_prompt`)로 분리함.
- 사용자 대화 프롬프트에서 매 턴 `.project/project.md` 전체를 재전송하지 않도록 제거하고, 초기 seed 컨텍스트는 모달 진입 시 1회 전달되도록 유지함.

- AI가 규칙을 어기고 전체 `project.md` 덤프를 반환하는 경우를 방지하기 위해 AI Detail 후처리 필터를 추가하고, 사용자가 명시적으로 전체 업데이트를 요청하지 않은 턴에서는 덤프 응답을 화면에 노출하지 않도록 제한함.

- Detail 탭 레이아웃을 재배치해 `temp(Project)` 패널이 좌측 상단에서 가로 2칸(span)으로 표시되도록 조정함.
- `Rule`/`Constraint` 패널을 `temp` 아래 중간 행에 좌우 가로 배치로 변경하고, `Features` 패널은 좌측 영역 맨 아래에서 가로 2칸(span)으로 배치함.
- `project.md`가 없을 때도 `temp` 패널에 프로젝트 기본 정보(name/description/path)가 보이도록 fallback 렌더를 추가하고, 값 가독성 색상을 라이트 배경 기준으로 조정함.

- Detail 탭의 `Project` pane 상단에서 `Name`과 `Description`을 같은 행의 좌/우 2분할로 표시하도록 레이아웃을 조정함.
- `Rule`/`Constraint`/`Features` 목록 항목이 길면 한 줄에서 `...`으로 축약되도록 단일행 ellipsis 렌더를 적용함.
- `Rule`/`Constraint`/`Features` 편집 모달을 list item pane 통합 형식으로 개편해 `a(add)`, `e(edit)`, `d(delete)`를 모달 내부에서 처리하고 우하단 `Confirm/Cancel`로 일괄 적용/취소되도록 변경함.
- `Drafts` pane이 비어있을 때 `no draft item`을 pane 정중앙에 표시하도록 조정함.
- `Drafts` pane 포커스에서 `Enter`를 누르면 y/n 확인 모달을 띄우고, 확인 시 선택 프로젝트 디렉터리에서 `create-draft` 명령을 실행하도록 연결함.

- bootstrap 초기화 가드를 추가해 대상 프로젝트 폴더에서 가시 파일(숨김/`.project` 제외)이 하나라도 있으면 bootstrap을 건너뛰도록 변경함.
- 템플릿/스타일 경로 해석을 루트 `assets/` 우선으로 정리하고, 실제 파일(`pane_style.yaml`, `draft.yaml`, `tasks.txt`)을 `src/assets`에서 `assets`로 이동해 폴더 위치를 일치시킴.

- `assets/templates/drafts_list.yaml` 템플릿을 추가하고, 빈 워크스페이스 초기화 시 `.project/drafts_list.yaml`을 코드 기본값 대신 해당 템플릿에서 생성하도록 변경함(템플릿 미존재 시 기존 기본값 fallback 유지).

- `configs/bootstrap.md`를 추가하고 언어/프레임워크별 bootstrap 규칙(`match_any` + `template`)을 YAML 코드블록으로 정의해 관리할 수 있도록 함.
- UI bootstrap 실행 시 `configs/bootstrap.md`를 우선 로드해 `spec` 매칭 규칙 기반으로 템플릿(`rust`, `node-react`)을 적용하도록 `src/ui/mod.rs`를 수정하고, 규칙 미매칭/알수없는 템플릿은 `.project/bootstrap.md` 수동 안내 노트를 생성하도록 처리함.

- Detail 탭 레이아웃을 3x3 기준으로 재구성해 `Project(좌측 1-2행 span) / Rule(중앙 1행) / Constraint(중앙 2행) / Features(중앙 3행) / Drafts(우측 세로 span)`이 동시에 보이도록 `src/ui/mod.rs`를 수정함.
- Project pane은 form 스타일로 `Name/Description/Spec/Goal` 라벨과 값을 분리 렌더링하도록 변경함.
- Detail 탭 포커스 이동을 5개 pane 기준 화살표 네비게이션으로 확장하고, Enter 동작을 `Rule/Constraint/Features` 편집 모달로 연결함.

- AI Detail 모달의 스트림 수신기를 보강해 stdout뿐 아니라 stderr도 실시간으로 응답 영역에 표시되게 수정함.
- codex 실행 실패 시 기존처럼 무응답처럼 보이지 않도록 stderr 진행 로그/오류가 즉시 화면에 노출되게 조정함.

- Detail 탭을 `Project Info(읽기전용)` / `Rule` / `Constraint` 3-pane 구조로 개편하고, `Rule`/`Constraint` pane에서 Enter로 리스트 편집 모달(add/delete)을 열어 `.project/project.md` 섹션을 직접 갱신하도록 구현함.
- Create Project 폼에 `spec` 입력 필드를 추가하고, 생성 시 `plan-init -s <spec>`으로 전달되도록 연결해 project.md의 spec이 입력값으로 채워지게 수정함.
- 프로젝트 생성 후 `detail fill(y/n)` 단계가 끝나면 `bootstrap(y/n)` 확인 모달을 열고, spec 기반으로 Rust/Node 계열 초기 템플릿(파일 미존재 시 생성) 또는 수동 bootstrap 노트를 생성하도록 자동화함.

- Detail 탭 `Features` pane의 데이터 소스를 `.project/project.md`가 아니라 `.project/drafts_list.yaml`의 `feature` 항목으로 전환함.
- `Rule`/`Constraint`/`Features` list edit 모달을 재구성해 내부 고정 Input pane을 제거하고, `a/n`·`e` 입력 시 오버레이 Edit pane이 뜨는 구조로 변경했으며 리스트가 길면 선택 이동(↑/↓) 기준으로 스크롤되게 조정함.
- list item 렌더에 항목별 구분선(`-`)을 추가하고 모달 크기를 크게(거의 전체 화면) 확장해 긴 목록 가시성을 높임.
- list edit 모달 활성 상태에서 `bar_status`가 추가/생성/편집/삭제 단축키를 안내하도록 분기 문구를 갱신함.
- `Features` 저장 시 `.project/drafts_list.yaml`로 직접 반영되게 변경하고, 항목 형식을 `기능명 : 설명`으로 검증/정규화하도록 강제함.

- pane style을 `active/normal/inactive` 3단계로 확장하고 `normal=black`, `inactive=gray`를 적용해 기본 border와 비활성 border를 분리함(`assets/style/pane_style.yaml`, `src/ui/mod.rs`).
- 모달/에디트 pane이 열려 있을 때 배경 pane border가 모두 `inactive` 색으로 렌더되도록 공통 분기(`calc_has_overlay_modal`)를 추가함.
- Detail 탭 `Drafts` pane에 병렬 작업중 라벨(`Drafts | 작업중`)과 `task : 상태(대기/작업중/완료)` 실시간 목록 렌더를 추가함.
- Project 탭 카드의 선택 프로젝트 우상단에 병렬 진행중 배지(`작업중`)를 표시하도록 추가함.
- Detail 탭 `Drafts` pane 포커스에서 `p` 키로 병렬 상태 시뮬레이션을 시작할 수 있도록 연결함.

- Drafts pane 동작을 `drafts_list.yaml`의 `planned` 기준 3분기로 전환함: `planned` 비어있음(no draft item, inactive 색) / `planned` 존재(번호 리스트, normal 색) / 병렬 진행중(라벨 `작업중`, task별 상태 색상).
- Drafts pane 포커스에서 `Enter` 동작을 분기 처리해, `planned`가 비어있으면 즉시 `create-draft` 실행을 시작하고, `planned`가 있으면 병렬 처리 상태를 시작하도록 변경함.
- 병렬 진행중 Drafts 리스트를 `task : 상태` 형식으로 표시하고, 상태별 색(`작업중=active`, `완료=normal`, `대기=inactive`)을 적용함.
- 병렬 완료 시 현재 `planned`를 다시 검사해 상태 문구를 갱신하고, 렌더 분기를 즉시 재평가하도록 조정함.
- Project 탭 카드 우상단에 선택 프로젝트 기준 실시간 `작업중` 배지를 표시하도록 추가함.

- Drafts pane이 `planned` 비어있는 분기에서도 포커스(선택) 시에는 border가 active(초록)로 보이도록 선택 우선순위를 조정함.
- bar_status에서 `enter`, `arrows` 관련 단축키 문구를 제거하고, Drafts pane 포커스 시 `b: build-draft/run-parallel` 안내를 표시하도록 변경함.
- Drafts pane 포커스 상태에서 `b` 키를 추가해 `Enter`와 동일하게 `create-draft` 또는 병렬 실행 분기 동작을 시작하도록 연결함.

- Detail 탭 `Project` pane을 단일행 항목(`Name/Description/Spec/Goal`) + 항목 사이 하단 구분선(`─`) 형식으로 변경해 각 속성이 한 줄씩 보이도록 조정함.
- Detail 좌측 레이아웃 비율을 조정해 `Project` pane 높이를 줄이고 `Rule/Constraint`가 포함된 중간 영역 높이를 확장함.
- `bar_status` pane의 기본 border 색을 `inactive`에서 `normal`로 변경함.

- `add-function` 흐름을 멀티라인 객체 입력 기반으로 개편함: `# 이름`, `> step`, `- 규칙` 포맷(여러 객체 가능)을 파싱해 객체별 draft를 생성하도록 `src/main.rs`를 수정함.
- `add-function` 실행 시 생성된 feature를 `.project/project.md`의 `## features`에 자동 반영하고, `.project/drafts_list.yaml`의 `planned`에도 추가되도록 연결함.
- CLI `add-function`이 선택 인자로 입력 문자열을 직접 받을 수 있게 확장해(UI 호출 포함) 비대화식 실행을 지원함(`src/cli/mod.rs`).
- Detail 탭 Drafts pane 포커스에서 `a` 단축키로 멀티라인 입력 모달을 열고, 입력을 `add-function` 명령으로 전달해 draft 추가를 수행하도록 구현함(`src/ui/mod.rs`).
- `.agents/log.md`의 동일 날짜 중복 헤딩을 날짜별 단일 헤딩으로 통합하고, `.gitignore`에 `/target`을 추가해 빌드 산출물(`target`)이 git 추적 대상에서 제외되도록 정리함.
