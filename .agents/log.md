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

## 2026-03-03 - 작업한일
- `project.md` 생성 경로의 프롬프트 지시를 강화해 `src/main.rs` fallback 프롬프트와 `assets/prompts/project-md-init.txt`, `assets/prompts/project-md-auto.txt`에 `/home/tree/ai/skills/build-domain/SKILL.md` 참조 문구를 추가함.

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

## 2026-03-05 - 작업한일
- `AGENTS.md`의 중복 규칙(금지어 하드블록/CLI 해석/하드코딩 규칙 반복)을 단일 섹션으로 정리하고, 현재 저장소와 무관한 모노레포 전용 규칙(`Port Ownership Override`)을 제거해 문서를 간소화함.

## 2026-03-04 - 작업한일
- TUI 커맨드 게이트를 `src/tui/mod.rs`로 분리하고, `open-ui` 실행 전 `client.tui` 설정을 검사하도록 연결함. `client.tui: false`면 `orc activate-tui` 안내와 함께 실행을 차단하도록 구현함.
- `orc activate-tui` 명령을 추가해 설정 파일(`configs.yaml` 우선, 없으면 `config.yaml`)의 `client.tui` 값을 `true`로 기록하도록 구현함.
- `src/config/mod.rs`에 `client.tui` 스키마(`ClientConfig`)와 기본값 메서드(`tui_enabled`, 기본 `true`)를 추가함.
- CLI 라우팅/도움말을 갱신해 `activate-tui`를 공식 명령으로 노출하고 `open-ui`는 새 `tui` 모듈 핸들러를 사용하도록 변경함.
- `config.yaml`, `configs.yaml`, `README.md`를 갱신해 `client.tui: true` 기본값 및 `activate-tui -> open-ui` 흐름을 문서/예시와 동기화함.
- 검증: `cargo test` 통과(20 passed), `cargo run --bin orc -- activate-tui` 정상 동작 확인, `client.tui: false` 상태에서 `orc open-ui`가 의도대로 차단됨을 확인함.

## 2026-03-04 - 작업한일
- `create-project` CLI를 옵션 기반으로 확장해 `-n/-p/-s/-d` 인자를 지원하도록 `src/cli.rs`를 수정함.
- `-p` 생략 시 현재 디렉터리 경로, `-n` 생략 시 현재 폴더명, `-s` 생략 시 `nextjs`, `-d` 생략 시 `heolloworld를 출력하는 간단한 web app으로` 기본값이 적용되도록 구현함.
- 기존 위치 인자 방식(`create-project <name> [path] [description]`)도 깨지지 않도록 하위호환 파싱을 유지함.
- `project.md` 생성에 실제 spec이 반영되도록 `src/main.rs`/`src/project.rs`의 `create_project` 호출 시그니처를 확장하고, 내부 `action_generate_project_plan`에 spec 전달을 연결함.
- 검증: `cargo test` 통과(20 passed), `cargo run --bin orc -- create-project -n sampleweb -p <tmp> -s nextjs -d ...` 실행 성공, `cargo run --bin orc -- create-project -p <tmp>` 실행 시 기본값(name/spec/description) 동작 확인.

## 2026-03-04 - 작업한일
- 코드 작업 플로우 전용 명령군을 추가함: `init_code_project`, `init_code_plan`, `add_code_plan`, `create_code_draft`, `add_code_draft_item`, `impl_code_draft`, `check_code_draft`, `check_task`, `check_draft`.
- 새 명령을 기존 엔진과 연결하기 위해 `src/code.rs` 모듈을 신설하고, `.project/plan.yaml`/`.project/drafts.yaml` 상태 전이( planned/worked/complete )와 draft 파일 생성 경로를 구현함.
- `check_code_draft`에 check-code 후속 실행 및 `report.md` 생성을 연결하고, `.project/scenario.md` 형식 검증(명령 | 실행/변경 파일 | 파생 결과)을 반영함.

## 2026-03-05 - 작업한일
- `AGENTS.md`에 레거시 호환 모드/경로를 유지하지 않고 제거하는 `Legacy Compatibility Removal Rule`을 추가함.
- `src/main.rs`의 spec checkpoint 경로 해석에서 레거시 fallback(`assets/checkpoints`) 읽기 로직을 제거하고, canonical 경로(`assets/checkPoints/<spec>.md`)만 사용하도록 정리함.
- `src/code.rs`의 `enforce_project_md_primary_path`에서 `project/project.md` 마이그레이션/삭제 루틴을 제거하고 `.project` 경로만 보장하도록 단순화함.

## 2026-03-05 - 작업한일
- `./.project/project.md`를 상세화해 `# info` 요구사항(astro/react/typescript/tailwind/shadcn/zustand/gsap, main/ask 분리, 반응형)을 반영한 공통 `rules`/`constraints`를 구체 항목으로 보강함.
- `# domains`를 `landing_main_page`, `ask_page`, `navigation`, `scroll_event_animation`, `ui_component_system`, `state_management` 블록으로 확장하고, 각 도메인의 `### states`, `### action`, `### rules`, `### constraints`를 모두 `-` 불릿 형식으로 정리함.
- `assets/code/prompts`에 신규 프롬프트 파일 11종을 추가하고, 각 파일에 코드/문서 형식 준수 지시를 명시함.
- `assets/code/templates`에 `plan.yaml`, `drafts.yaml` 템플릿을 추가해 코드 플로우 템플릿 경로를 통일함.
- `src/main.rs`, `src/cli.rs`, `README.md`를 갱신해 새 명령 라우팅 및 도움말/문서 노출을 동기화함.
- 검증: `cargo test` 통과(20 passed), `orc help`에서 신규 명령 노출 확인, 임시 워크스페이스에서 `init_code_plan -> add_code_plan` 실행 경로와 `.project/plan.yaml`/`.project/drafts.yaml` 생성을 확인함.

## 2026-03-04 - 작업한일
- 사용자 지시에 맞춰 레거시 `tasks_list` 경로를 정리하기 위해 `add-plan/create-draft/add-draft` CLI 라우팅을 새 `code` 명령(`add_code_plan/create_code_draft/add_code_draft`)으로 전환함.
- `action_sync_project_tasks_list_from_project_md`를 no-op으로 비활성화하고, 관련 레거시 동기화 보조 함수들을 제거해 더 이상 레거시 `tasks_list` 동기화를 수행하지 않도록 정리함.
- `src/plan.rs`를 레거시 drafts_list 접근 구현에서 `code::add_code_plan` 위임 형태로 교체함.
- 요구사항 체크 문서를 `check_list.md`로 생성하고, `cargo test` 통과를 확인함.

## 2026-03-04 - 작업한일
- `drafts.yaml` 템플릿을 `draft:` 리스트 단일 구조로 고정하고, `draft_item.yaml` 템플릿을 요청 스키마(`name/type/domain/depends_on/scope/rule/step/tasks/constraints/check`)로 추가/정정함.
- `src/code.rs`의 drafts 입출력 구조를 `draft` 중심으로 변경하고, 구 구조(`planned/worked/complete`)는 읽기 호환으로만 병합 처리하도록 정리함.
- drafts 저장 로직을 수동 직렬화로 보강해 출력 키를 `constraints`로 고정 반영함(오타 키 `constratins`는 읽기 호환만 유지).
- 검증: `cargo test` 통과(20 passed).

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

## 2026-03-03 - 작업한일
- `Rule`/`Constraint`/`Features` 편집 모달을 list item pane 통합 형식으로 개편해 `a(add)`, `e(edit)`, `d(delete)`를 모달 내부에서 처리하고 우하단 `Confirm/Cancel`로 일괄 적용/취소되도록 변경함.
- `Drafts` pane이 비어있을 때 `no draft item`을 pane 정중앙에 표시하도록 조정함.
- `Drafts` pane 포커스에서 `Enter`를 누르면 y/n 확인 모달을 띄우고, 확인 시 선택 프로젝트 디렉터리에서 `create-draft` 명령을 실행하도록 연결함.

## 2026-02-23 - 작업한일
- `create-draft`/`add-draft` 실행 시 tmux 옆 pane을 새로 분할하지 않고, 현재 tmux pane(LLM 대화 pane)에 `codex exec`를 전송하도록 `src/main.rs`의 draft 전송 경로를 변경함.
- draft 실행 결과 메시지를 `current tmux pane` 기준으로 갱신해 실제 실행 대상 pane 의미와 일치시킴.
- 검증: `cargo test` 통과.

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

## 2026-02-23 - 작업한일
- `assets` 하위 코드 리소스를 `assets/code`로 재구성하고, 템플릿/프롬프트 파일을 `assets/code/templates`, `assets/code/prompt`로 이동함.
- `src/main.rs`, `src/ui/mod.rs`의 리소스 경로 탐색 로직을 갱신해 `assets/code/...`를 우선 참조하도록 변경함(기존 경로는 fallback 유지).
- `assets/layouts/code.yaml`를 추가해 Detail 탭 패널 메타(이름/type/selected_view)와 숫자 기반 위치(`cell_start`, `cell_end`)를 선언하도록 구성함.
- `src/ui/mod.rs`에 `layout_load(preset)` 및 grid 숫자 셀 파서를 추가하고, Detail 탭 렌더를 하드코딩 분할 대신 레이아웃 프리셋 기반 계산으로 전환함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- `src/ui/component.rs`를 신설하고 UI 공통 컴포넌트를 분리함: 탭 헤더(`render_tab_header`), y/n 확인 pane(`render_confirm_cancel_wrapper`), 진행 상태 모달(`render_busy_modal`), LLM 대화 pane(`render_llm_chat_pane`).
- `src/ui/mod.rs`에서 기존 렌더 책임을 component 모듈 호출로 전환해 UI 루프 본문의 인라인 렌더 코드를 축소함.
- 기존 `action_render_ai_chat_modal`은 cursor 계산/입력상태 판단만 담당하고 실제 pane 렌더는 component 모듈로 위임하도록 정리함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- AI Detail 모달 `Response` 패널에 하단 자동 스크롤을 추가해, 대화가 길어져도 최신 응답이 항상 화면에 보이도록 수정함.
- 응답 높이 대비 누적 라인 수를 계산해 렌더 시 스크롤 오프셋을 적용하도록 `src/ui/mod.rs`, `src/ui/component.rs`를 갱신함.
- `cargo test` 통과로 변경 회귀 여부를 확인함.

## 2026-02-23 - 작업한일
- Detail 탭 Drafts pane에서 `b`/`Enter`로 draft 생성 트리거 시, `project.md`의 `features`가 비어 있으면 실행을 차단하고 상태바에 원인(`no feature`)과 조치(`Features pane 먼저 편집`)를 명확히 표시하도록 수정함(`src/ui/mod.rs`).
- 기존처럼 바로 요청 후 종료된 것처럼 보이던 UX를 방지해, 생성 불가 원인을 즉시 확인할 수 있게 개선함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- Detail 탭의 `Features` pane 제목을 `Support Features`로 변경하고, 표시 데이터 소스를 `.project/tasks_list.yaml`의 `featured` 목록(레거시 `drafts_list.yaml/feature` fallback 지원)으로 전환함.
- `Drafts` pane 목록 소스를 `.project/tasks_list.yaml`의 `planned` 키로 전환함(레거시 `drafts_list.yaml` fallback 지원).
- `Drafts` pane 포커스에서 `a` 키 입력 시 add-modal 대신 `create-draft` 실행 흐름으로 변경해, 생성 후 `tasks_list.yaml.planned` 기반 목록이 즉시 반영되는 사용 흐름에 맞춤.
- `create-draft` 프롬프트와 planned 갱신 경로를 `.project/tasks_list.yaml(featured/planned)` 기준으로 갱신하고, 빈 워크스페이스 초기화 시 기본 목록 파일을 `tasks_list.yaml`로 생성하도록 조정함.
- 템플릿 `assets/code/templates/drafts_list.yaml` 키를 `featured/planned`로 업데이트함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- `open-ui` 시작 시 각 프로젝트의 `.project/project.md`에서 `## features` 항목을 읽어 `.project/tasks_list.yaml`의 `planned`에 자동 동기화하도록 추가함(이미 `featured/planned`에 있는 값은 중복 추가하지 않음).
- `build-parallel-code` 실행 후 성공 완료된 task 키를 `.project/tasks_list.yaml`에서 `planned`에서 제거하고 `featured`로 이동시키는 후처리를 추가함.
- 단건 task 추가 경로(`add-function`/`add-draft`)는 기존 `flow_add_feature_to_planned`를 통해 `.project/tasks_list.yaml`의 `planned`에 누적되도록 유지함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- Detail 탭에서 `a` 키가 `create-project`를 트리거하지 않도록 키 바인딩을 제한함(`a`는 Project 탭에서만 create-project 동작).
- Detail 탭 Drafts 포커스 도움말에서 `a create-draft` 안내를 제거해 실제 동작과 일치시킴.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- `project.md` 동기화 규칙을 보강해 `.project/tasks_list.yaml`을 초기화할 때 `## featured`는 `featured`로, `## features`는 `planned`로 반영하도록 변경함(중복/교집합 정리 포함).
- `create-draft`를 tmux 전송형에서 실제 생성형으로 전환해 `.project/tasks_list.yaml`의 `planned` 항목을 기준으로 `.project/feature/<name>/draft.yaml`과 `task.yaml`을 생성하도록 연결함.
- `add-draft`도 `planned` 갱신 후 해당 feature 폴더의 `draft.yaml`/`task.yaml`을 생성·갱신하도록 연결함.
- parallel 실행 대상 수집 시 `task.yaml`(및 `tasks.yaml`)도 인식하도록 확장함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- Detail 탭 Drafts pane의 `b`/`Enter` 동작을 UI 시뮬레이션에서 실제 CLI 실행 흐름으로 전환함.
- `planned` 항목에 대해 `task.yaml`/`draft.yaml` 파일이 없으면 `create-draft`를 먼저 실행하고, 파일이 이미 있으면 `build-parallel-code`를 실행하도록 분기 연결함.
- `PendingUiAction::ApplyBuildParallel`와 `action_apply_build_parallel_via_cli`를 추가해 UI에서 병렬 빌드 명령을 실제로 수행하고 status_line에 결과를 반영함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- `AGENTS.md`에 `UI Flow Verification Rule`을 추가해, UI 변경 요청 시 화면 변경만으로 종료하지 않고 내부 기능 연결(트리거→실행→상태/파일 갱신→UI 반영)을 같은 작업에서 검증하도록 규칙화함.

## 2026-02-23 - 작업한일
- `rust-helper` 재현 확인 결과, `create-draft`가 종료되는 주요 원인은 설치형 바이너리에서 draft 템플릿 경로를 못 찾는 문제였고(`draft template not found`), `action_resolve_draft_template_path`에 `CARGO_MANIFEST_DIR` fallback 경로를 추가해 보정함.
- Detail 우측 영역을 `Plan`/`Drafts` 2개 pane으로 분리해 `Plan`에는 `tasks_list.yaml.planned`, `Drafts`에는 `.project/feature/*`의 생성된 draft/task 항목을 표시하도록 변경함.
- `Plan` pane에서 `b`(또는 Enter) 입력 시 `create-draft` 실행으로 연결하고, `Drafts` pane에서 `b`(또는 Enter) 입력 시에만 `build-parallel-code`를 실행하도록 분리함.
- `Drafts` pane 실행 전 검증을 추가해 `.project/feature`에 draft/task 파일이 없거나, planned 항목이 아직 파일로 생성되지 않았으면 경고 메시지를 표시하고 실행을 차단함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- Drafts pane 경고 출력 방식을 status bar 문자열에서 중앙 `Alarm` 모달(메시지 + `[확인]` 버튼)로 전환함.
- Drafts pane에서 `b` 실행 시 planned 미생성/feature 폴더 미생성 경고는 `Alarm` 모달로만 표시하고 status_line 경고 표시는 제거함.
- 멀티라인 한글 입력 시 줄 경계에서 커서가 한 줄 아래로 미리 이동해 위치가 꼬이던 문제를 수정함(`calc_cursor_in_input`의 즉시 줄바꿈 처리 제거).
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- `create-draft`가 LLM 생성을 거치지 않고 템플릿 복사만 하던 회귀를 수정해, `tasks_list.yaml.planned` 항목마다 LLM 응답(`FEATURE_NAME` + yaml)을 생성/검증 후 `.project/feature/<FEATURE_NAME>/draft.yaml` 및 `task.yaml`을 작성하도록 복구함.
- `add-draft`도 동일하게 LLM 생성 경로를 사용하도록 복구해, `FEATURE_NAME`(영문 요약)을 기준으로 파일 생성/갱신 및 planned 반영이 되도록 정리함.
- 기존 한글/슬래시가 포함된 planned 항목이 폴더명으로 직접 쓰이던 문제를 제거하고, LLM이 반환한 영문 `FEATURE_NAME`을 폴더 키로 사용하도록 수정함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- `tasks_list.yaml`에 한국어 문장형 항목이 그대로 들어가던 원인을 수정함: `project.md -> tasks_list` 동기화 단계에서 feature key를 LLM으로 영문 camelCase(`FEATURE_NAME`)로 정규화하도록 추가함.
- `## featured`/`## features` 및 기존 `tasks_list.yaml`의 `featured/planned` 값 모두를 동기화 시 정규화·중복제거·교집합 정리하도록 개선함.
- key-like 값(영문/토큰형)은 LLM 호출 없이 유지하고, 문장형/한글형 값만 LLM 네이밍을 거치도록 분기해 과도한 호출을 줄임.
- LLM 실패 시에도 동작이 멈추지 않도록 안정 fallback 키(`feature<hash>`)를 적용함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- `tasks_list.yaml` 생성/동기화 과정에 LLM 중간 단계를 추가해, `project.md ##features`의 문장형 항목을 `planned_items[{name,value}]` 구조로 먼저 정규화한 뒤 `planned(name 목록)`에 반영하도록 변경함.
- `planned_items`를 `DraftsListDoc` 스키마에 추가해 planned key와 설명 value를 함께 저장하도록 확장함(`main/ui` 모두 반영).
- 기존/레거시 데이터와 병합 시 `featured/planned/planned_items`를 키 기준으로 정리하고, `planned -> featured` 승격 시 대응 `planned_items`도 함께 제거되도록 후처리 보강함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- 로직 흐름 자동 검증을 위해 `src/main.rs`에 단위 테스트 3건을 추가함.
- 검증 대상: (1) `project.md -> tasks_list` 동기화 시 `planned/planned_items` 생성, (2) `planned` 단건 추가 시 `planned_items` 동기화, (3) `planned -> featured` 승격 시 `planned_items` 제거.
- 파일 기반 로직을 테스트 가능하게 하기 위해 `action_add_feature_to_planned_doc/at`, `action_promote_planned_to_featured_doc/at` 보조 함수를 도입함.
- 검증: `cargo test` 3 passed, 0 failed.

## 2026-02-23 - 작업한일
- 검증 누락 재발 방지를 위해 `AGENTS.md`에 `Execution-Path Verification Rule`을 추가하고, 실행 경로(트리거→호출→상태/파일 전이→후속 결과) 기준 검증을 명시함.
- `check-code` 스킬 문서(`~/ai/skills/check-code/SKILL.md`)에도 동일한 `Execution-Path Validation (Mandatory)` 항목을 추가해, UI/상태 문구만으로 통과 처리하지 않도록 기준을 강화함.

## 2026-02-23 - 작업한일
- `plan-drafts` 스킬의 최신 `draft.yaml` 형식을 소스 코드에 반영해 `DraftTask/DraftDoc` 스키마를 확장함(type/domain/scope/rule/step/touches/contracts, features.domain 포함).
- `build-draft(create-draft/add-draft/add-func)` 프롬프트를 최신 포맷으로 갱신하고, task.rule은 자동검증 가능한 식, contracts는 구조화 제약 형식으로 생성하도록 명시함.
- 병렬 실행 전 `check-draft` 자동 단계를 추가해 모든 draft yaml을 검사하고, 불완전/충돌/구조화 위반(rule 자동검증 불가, contracts 비구조화 포함) 발견 시 LLM으로 자동 개선 후 재검증하도록 연결함.
- parallel 대상 수집 시 top-level depends_on이 비어 있으면 task.depends_on 합집합을 fallback으로 사용하도록 보강함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- `orc open-ui` 정지 이슈를 수정: UI 진입 전 tasks_list 동기화 단계에서 LLM 호출이 블로킹되던 경로를 기본 비활성화함.
- 기본 동작은 비LLM 정규화(fallback key)로 즉시 통과하도록 변경하고, 필요 시 `ORC_SYNC_LLM=1` 환경변수로만 동기화 LLM 단계를 활성화하도록 분기 추가함.
- 적용 함수: `calc_sync_llm_enabled`, `action_normalize_feature_key_with_llm`, `action_generate_planned_items_with_llm`.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- 문장형 feature 동기화 트리거를 `open-ui`에서 제거해, UI를 열 때마다 LLM/정규화 처리가 반복 실행되지 않도록 수정함.
- `add-function` 실행 시작 시에만 `project.md -> tasks_list.yaml` 1회 동기화를 시도하도록 이동해, 최초 기능 추가 흐름에서만 문장형 feature 처리가 일어나도록 정렬함.
- 기존 `sync_initialized` 플래그를 그대로 활용해 1회 처리 이후 동일 경로 재실행 시 즉시 스킵되도록 유지함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- 문장형 feature 초기 동기화 트리거를 `add-function`에서 제거하고, `project.md` 생성 직후 초기화 단계(`flow_plan_init`)로 이동함.
- bootstrap 초기화 경로(`action_initialize_parallel_workspace_if_empty`)에서도 `project.md` 템플릿 작성 직후 동일 동기화를 실행하도록 연결해 첫 설정 단계에서 `tasks_list.yaml`이 즉시 완성되게 함.
- `sync_initialized` 기반 1회 처리 정책은 유지되어 이후 반복 실행은 스킵됨.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- Detail 탭 상태바에서 `a: create-project` 안내를 제거해, Detail 문맥에서 비활성 동작이 노출되지 않도록 수정함.
- `add-plan` CLI 명령을 추가해 `.project/tasks_list.yaml.planned`가 비어 있을 때만 LLM으로 planned key 목록(3~7개 camelCase)을 생성/저장하도록 구현함.
- Plan pane(포커스 4)에서 `a` 입력 시 planned가 비어 있으면 `add-plan`을 실행하고, 이미 값이 있으면 실행을 건너뛰도록 UI 액션(`ApplyAddPlan`)을 연결함.
- Detail 탭 상태바 도움말을 포커스별로 분리해 Plan pane은 `a add-plan, b create-draft`, Drafts pane은 `b build-parallel-code`로 표시되게 정리함.
- CLI/문서 동기화: `src/cli/mod.rs` usage와 `README.md` 명령 목록에 `add-plan [hint]` 추가.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- Plan pane `a` 동작을 CLI 즉시 실행(`add-plan`)에서 AI 대화 모달 방식으로 전환함.
- `AiChatModal`에 모드(`DetailProject`/`AddPlan`)를 도입하고, `AddPlan` 모드에서는 대화형 질문을 우선 진행한 뒤 적용 시 구조화 YAML(`add_plan_update`)를 반환하도록 프롬프트를 분리함.
- AddPlan 응답에 YAML codeblock이 포함되면 자동 파싱해 `project.md`의 `## featured`와 `.project/tasks_list.yaml`의 `featured/planned`를 동시 갱신하도록 연결함.
- AddPlan 모달 종료 시 bootstrap confirm으로 넘어가지 않고 모달만 닫히도록 분기 처리함.
- `DraftsListDoc`(UI) 직렬화 필드에 `sync_initialized`를 추가해 tasks_list 저장 시 기존 플래그가 소실되지 않도록 보완함.
- Detail 탭 Plan 도움말 문구를 `a ai-add-plan, b create-draft`로 갱신함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- add-plan 대화 응답 파서를 보강해, YAML codeblock이 깨졌거나(`featured:- ...planned:- ...`) 인라인으로 붙은 응답도 복구 파싱하도록 개선함.
- `featured:`/`planned:` 섹션을 raw 응답에서 직접 탐지하고, 토큰 정규화(camelCase) 및 중복/교차 제거 후 `project.md ## featured`와 `tasks_list.yaml(featured/planned)` 동시 반영 경로를 유지함.
- malformed 응답에서도 적용 실패로 끝나지 않고 가능한 키를 최대한 추출해 적용하도록 fallback 로직을 추가함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- add-plan AI 모드를 `추천`과 `적용` 단계로 분리함.
- 사용자가 `적용/반영`을 명시하지 않은 요청(예: "어떤 기능 추천?")에서는 YAML 출력을 금지하고 추천 목록+확인 질문만 응답하도록 프롬프트 규칙을 강화함.
- AddPlan 응답 적용은 `적용 요청` 플래그가 켜진 경우에만 수행하도록 변경해, 일반 대화 응답이 의도치 않게 `project.md/tasks_list.yaml`에 반영되지 않게 막음.
- 스트림 에러/취소 시 적용 플래그를 즉시 해제해 다음 대화로 상태가 누수되지 않도록 보강함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- add-plan 깨진 응답 복구 파서에 key 품질 검증을 추가함: 영문/숫자만, 시작은 소문자, 대문자 1개 이상 포함(camelCase 형태) 조건을 만족하는 토큰만 반영.
- 이에 따라 `Descrip` 같은 잡음 토큰은 제외되고 `textPreprocessCli`, `fileInputProcessing`, `logFilterPrepTool`, `productivityReportCommand` 같은 유효 키만 적용되도록 보정함.
- YAML 정상 파싱 경로와 fallback 복구 경로 모두 동일한 key 검증을 적용함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- LLM 대화 추적을 위한 미들웨어 로깅을 추가해 `.project/chat.log`를 자동 생성/append 하도록 적용함.
- 적용 범위 확장:
  - `src/main.rs`: `action_run_codex_exec_capture`, `action_run_codex_exec_capture_in_dir`, `action_run_llm_exec_capture`에 prompt/response/error 로깅 추가.
  - `src/ui/mod.rs`: AI 모달 warmup prompt, 사용자 입력, 생성된 prompt, LLM raw 응답, 에러/취소 이벤트 로깅 추가.
- 로깅 형식은 `[unix_ts] ROLE` + 본문으로 통일해 실제 LLM 입출력 추적이 가능하도록 구성함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- add-plan 적용 누락의 근본 원인을 수정함: `calc_normalize_feature_key`가 camelCase 입력을 전부 소문자로 변환하던 버그를 제거해, `textPreprocessCli` 같은 정상 key가 유효성 검사에서 탈락하지 않도록 보정함.
- 결과적으로 `LLM_RESPONSE_RAW`의 정상 YAML(`featured/planned` camelCase key)이 실제 `project.md ## featured`와 `tasks_list.yaml(featured/planned)` 반영 단계까지 통과되도록 복구함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- `project.md` 생성/보강 단계에서 도메인 네이밍 규칙이 누락되어 문장형(`CLI Domain`)이 생성되던 원인을 수정함.
- `src/main.rs`의 `flow_plan_init` 프롬프트 제약에 `# Domains` 형식을 `- <camelCase명사>: <설명>`으로 강제하고, `Domain` 접미사/공백/한글명 금지 규칙을 추가함.
- `assets/code/prompt/detail-project.txt`에도 동일 규칙을 반영해 detail 보강 시 기존 문장형 도메인으로 회귀하지 않도록 정렬함.
- `assets/code/templates/project.md` 기본 `# Domains` 플레이스홀더를 camelCase 예시(`cliRouting`, `textProcessing`, `fileOps`)로 교체함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- 사용자 지시대로 도메인 명명 규칙을 프롬프트에 직접 기술하던 내용을 제거하고, `# Domains`는 `$build_domain` 스킬 우선 적용만 명시하도록 수정함.
- 반영 파일:
  - `src/main.rs` (`flow_plan_init` prompt)
  - `assets/code/prompt/detail-project.txt`
  - `assets/code/templates/project.md` (`# Domains` 예시 키 하드코딩 제거)
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- add-plan 적용 흐름을 분리함: LLM 응답은 `project.md ## featured` 추가 항목만 다루고, `tasks_list.yaml.planned` 반영은 LLM 종료 후 별도 함수가 처리하도록 변경함.
- `action_apply_add_plan_update_from_yaml`에서 기존 `tasks_list featured/planned` 전체 치환 로직을 제거하고, `project.md ## featured`에는 dedupe append만 수행하도록 전환함.
- 새 후처리 함수 `action_append_planned_from_add_plan_items`를 추가해, `project.md`에 실제 추가된 featured key만 `tasks_list.yaml.planned`(+planned_items)로 누적 반영하도록 구현함.
- add-plan 프롬프트를 featured-only 응답 구조로 조정하고, `planned`는 시스템 함수가 자동 동기화한다는 규칙으로 통일함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- React Native bootstrap 프리셋을 추가함.
- `configs/bootstrap.md`에 `react-native` 템플릿 규칙을 추가하고, spec에 `react native`/`react-native`/`expo`가 포함되면 우선 매칭되도록 정리함.
- `src/ui/mod.rs`에 `action_apply_bootstrap_react_native_template`를 추가해 Expo 기반 RN 초기 파일(`package.json`, `app.json`, `App.js`, `.gitignore`)을 생성하도록 구현함.
- bootstrap 완료 시 의존성 미설치 문제를 해결하기 위해 `action_install_js_dependencies`를 추가하고 `bun/pnpm/npm/yarn install` 순서로 자동 설치를 시도하도록 연결함.
- `action_apply_bootstrap` 분기 및 spec fallback에도 react-native 처리를 우선 적용함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- Detail 탭 Plan pane의 `a` 키를 `add-function` 멀티라인 입력 모달로 연결함.
- 기존에 미연결 상태였던 `DraftBulkAddModal` 오픈 경로를 Plan pane 키바인딩에 붙여, `# 이름 / > step / - 규칙` 포맷 다중 입력이 실제 `add-function` CLI 호출로 전달되도록 복구함.
- 상태바 도움말을 `plan: a add-function, b create-draft`로 갱신함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- LLM 기반 project.md 보강 저장 경로를 `project/project.md` 우선으로 변경하고, 저장 시 `./.project/project.md`로 동기화되도록 통합 저장 함수(`action_write_project_md_with_sync`)를 추가함.
- UI의 project.md 읽기 경로를 `project/project.md` 우선, `.project/project.md` fallback으로 변경함.
- list edit 및 AI 응답 반영 경로도 동일 동기화 저장 함수를 사용하도록 교체함.
- Bootstrap 확인 모달 실행을 즉시 동기 호출에서 pending-action 기반으로 전환해, 실행 중 busy modal 메시지가 화면에 표시되도록 변경함.
- Bootstrap 실행 전에 `preset + spec` 기준 LLM 준비 호출(`action_run_bootstrap_llm_prepare`)을 추가해 실제 LLM 명령 호출과 로그 기록(`.project/chat.log`)이 발생하도록 연결함.
- react-native/node-react/rust 각 bootstrap 분기 및 spec fallback 분기에서 LLM 준비 호출이 수행되도록 반영함.
- 검증: `cargo test` 통과.

## 2026-02-23 - 작업한일
- Plan pane에서 `Enter`/`b` 실행 시 planned가 비어 있으면 no-op 상태문구만 띄우던 동작을 제거함.
- 동일 조건에서 즉시 `add-function` 멀티라인 입력 모달을 열도록 연결해, 버튼 입력이 실제 작업 흐름으로 이어지게 수정함.
- 적용 위치: `src/ui/mod.rs`의 Plan pane `Enter` 분기와 `b` 분기.
- 검증: `cargo test` 통과.
## 2026-02-24 - build parallel 비동기 상태/실행경로 preflight 및 draft 후속 check 호출
- Drafts pane의 build-parallel 실행을 비동기 수신 기반으로 고정해 빌드 중 `작업중` 상태 유지 및 pane 전환 가능하도록 수정.
- `create-draft`/`build-parallel-code` 실행 전에 preflight 검증 함수 추가(planned 비어있음/이름 형식/중복/파일 미생성 검증).
- `create-draft`, `add-draft`, `add-function` 완료 직후 추가 LLM 호출로 check-code 후속 점검/개선 명령을 직접 실행하도록 연결.
- 관련 테스트 2건 추가 및 전체 테스트 통과 확인(총 5개).
## 2026-02-24 - bootstrap skip 원인 수정 및 병렬 실행 중단 방지
- bootstrap 비어있음 판정에서 `project/`, `.agents/`를 내부 폴더로 간주해 초기 bootstrap이 skip되지 않도록 수정.
- 병렬 빌드 실행 중 프로젝트 그리드 선택 이동 시 런타임 상태를 초기화하지 않도록 변경하고, 실행 중에는 선택 이동을 잠금 처리.
- 결과적으로 상세/탭 전환 중에도 병렬 작업이 수신 채널 기준으로 유지되도록 보강.
## 2026-02-24 - 병렬 실행 중 프로젝트 전환 허용
- 병렬 build 실행 중 프로젝트 선택 이동 잠금 로직 제거.
- 병렬 작업은 백그라운드에서 유지하고 사용자는 다른 프로젝트로 이동해 작업 가능하도록 동작 조정.
## 2026-02-24 - project.md 완료 문구 실시간 감지 자동 전환
- AI 스트리밍 응답에서 `project.md 생성을 완료하겠습니다`(오타 `projet.md` 포함) 문구를 실시간 감지하면 즉시 다음 단계(bootstrap confirm)로 자동 전환하도록 연결.
- detail AI 응답에서 `plan-drafts-code` 등 다음 단계 유도 문구를 후처리로 제거해 project.md 보완 대화만 노출되도록 보강.

## 2026-02-24 - 작업한일
- `create-project` CLI에서 `path` 인자를 생략한 경우 기본 경로를 `./<name>`이 아니라 실행 시점 `current_dir()`로 사용하도록 `src/main.rs`를 수정함.
- 기본 작업 정책 파일(`/home/tree/ai/codex/AGENTS.override.md`)에 구현 전 `.agents/plan.yaml` 작성, 사전 `tmux` plan 검토, 구현 후 사후 `tmux` 점검 절차를 추가함.
- `tmux` pane 생성 규칙을 세로 분할(좌우 분할)로 고정하도록 정책에 반영함.

## 2026-02-24 - 작업한일
- post-check tmux pane 전송 방식을 보강해, 일반 문장 대신 `codex exec "$(cat ./.agents/plan.yaml; echo ... )"` 실행 명령을 보내도록 기준을 명시하고 실제 전송 흐름에 반영함.

## 2026-02-24 - 작업한일
- 사용자 요구 처리 기본 흐름을 `request.md` 체크리스트 + tmux 3단계(검증 pane → 구현 pane → 사후점검 pane)로 강제하도록 `/home/tree/ai/codex/AGENTS.override.md` 정책을 보강함.
- 사후점검 통과 전에는 체크 완료 금지, 통과 후 즉시 체크 처리 규칙을 추가함.
- 경로 표기를 `./.agents/plan.yaml`로 통일함.

## 2026-02-24 - 작업한일
- tmux 작업 지시/검증 지시 전송은 직접 `tmux send-keys` 대신 `orc send-tmux` CLI를 사용하도록 `/home/tree/ai/codex/AGENTS.override.md` 정책을 갱신함.
- 병렬 구현/처리 실행은 `orc` 내부 CLI 함수인 `build-parallel-code`를 우선 사용하도록 정책에 명시함.

## 2026-02-24 - 작업한일
- bootstrap 경로에서 Spec 반영 누락을 수정함(`src/ui/mod.rs`).
- `project.md` 파서를 보강해 `- spec:`뿐 아니라 `- spec :` 형식도 인식하도록 변경함.
- node/react bootstrap 템플릿이 Spec 키워드(`next`, `react`, `typescript`, `axios`, `zod`, `zustand`, `react query`, `tailwind`)를 읽어 `package.json` 의존성/스크립트에 반영되도록 확장함.
- rust bootstrap 템플릿도 Spec 키워드(`tokio`, `serde`, `reqwest`, `axum`)를 읽어 `Cargo.toml` dependencies에 반영되도록 확장함.
- 회귀 방지 테스트 2건 추가: spec 공백콜론 파싱, node bootstrap 의존성 반영 검증.

## 2026-02-24 - 작업한일
- `AGENTS.override.md`에 `error:` 발생 시 원인 해결+재검증까지 수행하는 강제 규칙을 추가함.
- fish 환경에서 `codex exec` 인자 분리 오류를 막기 위해 사후 점검 명령 템플릿을 `codex exec -- "$(string collect < $PROMPT_FILE)"` 형태로 수정함.

## 2026-02-24 - 작업한일
- `main.rs`의 tmux 관련 전송 함수(`flow_tsend`)를 `src/tmux/mod.rs`로 이동해 tmux 책임을 모듈로 일원화함.
- `main.rs`의 비동기 병렬 실행 함수(`action_run_one_parallel_task`, `flow_run_parallel_build_code`, `flow_press_key`)를 신규 `src/parallel/mod.rs`로 이동함.
- CLI 라우팅을 갱신해 `send-tmux`는 `tmux::flow_tsend`, `build-parallel-code`/`press-key`는 `parallel` 모듈 함수를 직접 호출하도록 변경함.

## 2026-02-24 - 작업한일
- Project preset 로드 시 허용 라이브러리 화이트리스트를 강제하도록 `src/ui/mod.rs`를 보강하고, 목록 외 항목은 자동 제외되게 수정함.
- `assets/presets/project.yaml` 파일을 신설해 Project 탭 `l`키 preset 로드가 실제 파일과 연결되도록 기본 three-fiber preset을 추가함.
- bootstrap 검증 누락 방지를 위해 테스트 3건을 추가함: three-fiber 의존성 반영, rust hello world `main.rs` 생성, preset allowlist 필터링.
- `.agents/implementation.md`를 보강해 `/tmp` 재현 절차, React/Next/Vite 분기 기준, rust `cargo run` 출력 확인, preset 허용 목록 기준을 명시함.

## 2026-02-24 - 작업한일
- 한글/영문 혼합 입력에서 커서 위치가 어긋나는 문제를 수정함.
- `calc_cursor_in_input`를 래핑 옵션 기반(`calc_cursor_in_input_with_wrap`)으로 분리하고, create/edit modal의 단일행 필드는 no-wrap 커서 계산을 사용하도록 변경함.
- 줄 경계 계산 시 셀 폭 클램핑을 적용해 혼합 폭 문자 입력에서도 커서가 하단 줄로 잘못 이동하지 않도록 보정함.
- 회귀 방지 테스트 추가: mixed-width no-wrap 커서 위치 검증.

## 2026-02-24 - 작업한일
- `planned`/`featured` 키 정규화 규칙을 `camelCase`에서 `동사_명사` snake_case로 변경함.
- `src/ui/mod.rs`의 add-plan 프롬프트를 수정해 LLM이 `verb_noun` snake_case를 출력하도록 강제하고, 응답 파서/검증기도 snake_case 기준으로 갱신함.
- `src/main.rs`의 planned 키 동기화/LLM 정규화/`add-plan` 생성 프롬프트를 snake_case 기준으로 변경하고 fallback 키를 `plan_feature_<hash>` 형태로 조정함.
- draft preflight의 planned 이름 검증을 snake_case 규칙으로 전환하고 관련 테스트 기대값을 snake_case로 업데이트함.
- 검증: `cargo test` 통과(11 passed).

## 2026-02-24 - 작업한일
- `FEATURE_NAME` 규칙을 단일 스킬로 분리해 `/home/tree/ai/skills/feature-name-prompt-rules/SKILL.md`를 추가함.
- `src/main.rs`에 스킬 스니펫 로더(`calc_feature_name_prompt_rules_from_skill`)를 추가하고, 스킬 미존재 시 fallback 규칙으로 동작하도록 처리함.
- `FEATURE_NAME`를 생성/정규화하는 모든 프롬프트(`action_normalize_feature_key_with_llm`, `flow_draft_create`, `flow_draft_add`, `flow_add_func`)가 스킬의 `Prompt Snippet`을 공통 주입하도록 변경함.
- 검증: `cargo test` 통과(11 passed).

## 2026-02-24 - 작업한일
- `project.md` 도메인 출력 형식 누락 원인을 수정함: 생성/보강 프롬프트가 `plan-project-code references/project.md` 형식을 강제하지 않던 문제를 보완.
- `src/main.rs`의 `flow_plan_init` 프롬프트에 `$plan-project-code` + `/home/tree/ai/skills/plan-project-code/references/project.md` 참조를 명시하고, `# Domains`를 `### domain + name/description/state/action/rule/variable` 구조로 강제함.
- `assets/code/prompt/detail-project.txt`와 `src/ui/mod.rs` 상세 보강 프롬프트에도 동일한 도메인 구조 강제 및 금지 패턴(`제안 도메인/근거/책임`)을 추가함.
- `assets/code/templates/project.md`의 `# Domains` 기본 템플릿을 구조화 포맷으로 교체함.
- `plan-project-code` 스킬 파일(`/home/tree/ai/skills/plan-project-code/SKILL.md`)에 `Domains Output Contract`를 추가해 스킬 자체에서도 동일 구조를 강제함.
- 검증: `cargo test` 통과(11 passed).

## 2026-02-24 - 작업한일
- LLM 프롬프트 기반 YAML/Markdown 생성 경로를 전수 점검하고, 형식 강제/검증 누락 지점을 보완함.
- `src/main.rs`에 `action_validate_project_md_format`를 추가해 `project.md` 생성/보강(`flow_plan_init`, `flow_detail_project`) 결과를 저장 전에 검증하도록 변경함.
- `src/ui/mod.rs`에도 동일한 `project.md` 포맷 검증을 추가해, Detail AI 응답이 형식 위반일 때 파일 저장을 차단하고 에러 상태를 표시하도록 수정함.
- draft YAML 생성 함수(`flow_draft_create`, `flow_draft_add`, `flow_add_func`)를 `DraftDoc` 스키마 파싱 + `action_validate_draft_doc` 검사 통과 후 저장하도록 강화함.
- 앞으로 필수 규칙으로 `AGENTS.md`에 `YAML/MD Format Enforcement Rule`을 추가해, 프롬프트 형식 강제 + 저장 전 파싱/검증 의무를 명문화함.
- 검증: `cargo test` 통과(11 passed).

## 2026-02-24 - 작업한일
- bootstrap LLM 호출 프롬프트를 계획 점검용에서 구현 지시용으로 변경함.
- `src/ui/mod.rs`의 `action_run_bootstrap_llm_prepare`에서 `project.md #info` 블록을 직접 읽어 프롬프트에 포함하고, `#info.spec` 기준으로 hello world 빌드 초기화를 구현하라는 명령을 명시하도록 수정함.
- rust/react 계열 실행 완료 기준(`cargo run` 출력/화면 표시)을 프롬프트 요구사항에 포함함.
- 검증: `cargo test` 통과(11 passed).

## 2026-02-24 - 작업한일
- `/home/tree/temp/.project/tasks_list.yaml` 점검 결과 Plan pane이 내부 key(`planned`)를 직접 표시해 `plan_feature_*` 난수형 이름이 그대로 노출되는 원인을 확인함.
- `src/ui/mod.rs`에서 Plan pane 렌더 시 표시 문자열을 `planned_items.value` 우선으로 사용하도록 변경해, 사용자가 이해 가능한 설명형 이름이 보이도록 수정함.
- 내부 실행 로직은 기존대로 `planned` key를 사용하고, 화면 표시만 개선해 동작 회귀 없이 가독성을 확보함.
- 검증: `cargo test` 통과(11 passed).

## 2026-02-24 - 작업한일
- `plan-init`에서 사용하는 `project.md` 생성 LLM 프롬프트(`src/main.rs`)의 제약/섹션 강제 블록을 제거하고, 요청한 입력/출력 형식 안내 문구(레퍼런스 파일에서 설명문/예시문 제외 후 속성값 채우기)로 교체함.

## 2026-02-24 - 작업한일
- `src/main.rs`의 주요 프롬프트(autos/run, create-draft/add-draft/add-function/add-plan, plan-init, draft-fix, check-code follow-up)를 축약하고, 스킬 담당 규칙은 상세 하드코딩 대신 `스킬 사용` 한 줄 지시로 교체함.
- YAML/Markdown 템플릿 지시를 `파일 참조` 방식에서 `대상 폴더로 템플릿 복사 -> 주석/예시 제거 -> 값 수정` 방식으로 변경함.
- `assets/code/prompt/detail-project.txt`, `src/ui/mod.rs`의 detail/add-plan 관련 프롬프트에도 동일한 템플릿 복사 지시와 스킬 우선 지시를 반영함.
- 이관된 상세 규칙을 스킬 문서로 이동/추가함: `/home/tree/ai/skills/plan-drafts/SKILL.md`, `/home/tree/ai/skills/check-code/SKILL.md`, `/home/tree/ai/skills/plan-project-code/SKILL.md`.

## 2026-02-24 - 작업한일
- 오케스트레이션 성격 함수의 접두사를 `flow_`에서 `stage_`로 일괄 변경함(`src/main.rs`, `src/cli/mod.rs`, `src/parallel/mod.rs`, `src/tmux/mod.rs`, `src/ui/mod.rs`).
- CLI 라우팅/내부 호출 참조도 동일하게 `stage_*` 명으로 동기화해 컴파일 경로를 유지함.
- 검증: `cargo test` 통과(11 passed).

## 2026-02-24 - 작업한일
- 사용자 피드백에 따라 오케스트레이션 함수 접두사를 `stage_`에서 `flow_`로 되돌리고, 모든 내부 호출 참조를 함께 복구함(`src/main.rs`, `src/cli/mod.rs`, `src/parallel/mod.rs`, `src/tmux/mod.rs`, `src/ui/mod.rs`).

## 2026-02-24 - 작업한일
- 추가 요청에 따라 오케스트레이션 함수 접두사 `flow_`도 제거하고 무접두 함수명으로 일괄 변경함(`src/main.rs`, `src/cli/mod.rs`, `src/parallel/mod.rs`, `src/tmux/mod.rs`, `src/ui/mod.rs`).
- 관련 호출 참조를 모두 동기화해 CLI 라우팅/내부 모듈 호출이 동일하게 동작하도록 유지함.
- 검증: `cargo test` 통과(11 passed).

## 2026-02-24 - 작업한일
- 사용자 요청에 따라 스킬 문서 내 `flow_` 접두사 사용 규칙을 전수 점검하고, `/home/tree/ai/skills/build-code-parallel/SKILL.md`의 관련 규칙 문장을 제거함.
- 추가 점검으로 `/home/tree/ai/skills`, `/home/tree/.codex/skills` 전역에서 `flow_` 함수명 규칙 문구가 남아있지 않음을 확인함.

## 2026-02-24 - 작업한일
- 함수명 규칙 요청에 맞춰 `/home/tree/ai/skills/feature_architecture_rules/SKILL.md`에 `Function Naming Rule` 섹션을 추가하고 기본 접두사(`creat_`, `get_`, `set_`, `filter_`, `convert_`)를 명시함.

## 2026-02-24 - 작업한일
- 함수명 접두사 기본 규칙에 `update_`, `remove_`를 추가해 총 허용 기본 접두사를 확장함(`/home/tree/ai/skills/feature_architecture_rules/SKILL.md`).

## 2026-02-24 - 작업한일
- 함수명 접두사 기본 규칙을 사용자 요청에 맞게 갱신함: `creat_` 오타를 `create_`로 수정하고 `load_`, `save_`, `flow_`를 추가함(`/home/tree/ai/skills/feature_architecture_rules/SKILL.md`).

## 2026-02-24 - 작업한일
- 파일명 규칙을 함수명 규칙과 분리해 관리하도록 `/home/tree/ai/skills/feature_architecture_rules/SKILL.md`에 `File Naming Rule` 섹션을 추가함.
- 기본 규칙으로 `명사_동사` 순서와 `snake_case` 형태를 명시함.

## 2026-02-24 - 작업한일
- 함수명 접두사 기본 규칙에 `add_`를 추가함(`/home/tree/ai/skills/feature_architecture_rules/SKILL.md`).

## 2026-02-24 - 작업한일
- 함수명 접두사 기본 규칙에 `enter_`를 추가함(`/home/tree/ai/skills/feature_architecture_rules/SKILL.md`).

## 2026-02-24 - 작업한일
- 리팩토링 1단계(문서/템플릿 고정)로 `AGENTS.md`에 planning framework, init-plan/stage_draft/check-build 순서 규칙을 추가함.
- `assets/code/templates/project.md`를 feature -> domain -> flow 기준으로 보강하고 `## plan`, `# UI`, stage 기반 `# Flow` 초안 구조를 추가함.
- `assets/code/templates/draft.yaml`, `assets/code/templates/drafts_list.yaml`에 `flow`/`constraints` 관련 필드를 추가해 도메인+흐름 제약 합성 모델을 반영함.
- `assets/code/prompt/tasks.txt`, `assets/code/prompt/detail-project.txt`를 업데이트해 템플릿 복사 후 값 치환 규칙과 `plan/ui/flow` 보강 목표를 명시함.
- 신규 템플릿 `assets/code/templates/stage.md`, `assets/code/templates/task.md`를 추가함.

## 2026-02-24 - 작업한일
- 사용자 요청에 따라 `featured`/`features` 용어를 `features`로 단일화함: 함수명/변수명/프롬프트/템플릿/AGENTS 규칙 및 코드 경로에서 `featured` 표기를 제거함(`src/main.rs`, `src/ui/mod.rs`, `src/parallel/mod.rs`, `assets/code/templates/drafts_list.yaml`, `AGENTS.md`).
- `.project/tasks_list.yaml` 스키마를 `features/planned` 기준으로 일치시키고, `project.md` 동기화 로직도 `## features` + `## plan` 조합으로 정리함(호환 fallback 유지).
- 스킬 문서도 동일 규칙으로 동기화함(`/home/tree/ai/skills/plan-drafts/SKILL.md`, `/home/tree/ai/skills/build-code-parallel/SKILL.md`).
- 검증: `cargo test` 통과(11 passed).

## 2026-02-24 - 작업한일
- Project 초기화 경로를 점검해 `plan-init` 이후 bootstrap이 수동 단계로 남아 있던 흐름을 보완함.
- 공용 함수 `action_apply_bootstrap_by_spec`를 추가해 bootstrap spec 판정/적용 로직을 단일화하고, UI bootstrap 경로도 동일 함수를 사용하도록 중복을 줄임(`src/ui/mod.rs`).
- `plan_init` 완료 직후 공용 bootstrap을 자동 호출해 `.project/project.md` 생성부터 bootstrap까지 한 번에 완료되도록 연결함(`src/main.rs`).
- 검증: `cargo test` 통과(11 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- 생성/추가 직후 초기화 체인을 보강함: UI `create project` 처리에서 project.md 파일 유무를 점검하고 없으면 `plan-init`을 자동 실행해 메타를 생성하도록 수정함.
- `project/project.md`와 `.project/project.md` 동기화를 보장하는 보조 함수를 추가해 detail 패널에서 프로젝트 정보가 보이지 않던 경로를 차단함.
- 신규 생성(또는 초기화가 필요한 갱신) 후에는 확인 모달 대신 AI detail 대화를 즉시 열도록 변경해 보완 질의 단계가 자동으로 시작되도록 조정함.
- 검증: `cargo test` 통과(11 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- 응답 톤 재발 방지를 위해 `AGENTS.md`에 `Response Phrase Rule`을 추가함.
- 금지 문구 예시(`맞습니다`, `맞아요`, `인식했습니다`)를 명시하고 결과/액션부터 시작하도록 규칙화함.

## 2026-02-24 - 작업한일
- 사용자 요청에 따라 루트에 `Agents.override.md`를 생성하고 응답 금지 문구 규칙(`맞습니다/맞아요/인식했습니다`)을 오버라이드로 고정함.

## 2026-02-24 - 작업한일
- `configs/project.yaml` 저장 루트를 실행 파일 경로 난립(`target/debug`)에 종속되지 않도록 보정함.
- `action_source_root()`에 우선순위를 추가: `ORC_HOME` 환경변수 -> `/home/tree/.cargo/bin/orc` 설치 경로 -> 현재 실행 파일 경로.
- 결과적으로 프로젝트 레지스트리(`project.yaml`)가 설치 바이너리 기준 단일 위치(`/home/tree/.cargo/bin/configs/project.yaml`)에 기록되도록 복구함.
- 검증: `cargo test` 통과(11 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- 입력이 필요한 pane의 닫기 키 정책을 맞추기 위해 create modal에서 `q` 닫기 분기를 제거함.
- 이제 create modal은 `Esc`로만 닫히고, `q`는 일반 문자 입력으로 처리됨(`src/ui/mod.rs`).
- 검증: `cargo test` 통과(11 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- Detail AI에서 `project.md` 전체 출력 허용 판정을 엄격화함: 명시 문구(`project.md 전체 업데이트/출력`, `full project.md`)일 때만 전체 출력 허용.
- 전체 출력이 감지됐지만 허용되지 않은 경우, 화면 표시 제한뿐 아니라 실제 `project.md` 파일 적용도 차단하도록 수정함.
- Project pane 표시 안정화를 위해 `project/project.md`와 `.project/project.md`를 모두 읽어 점수가 높은 문서를 선택하도록 개선하고, 파싱 결과가 비어 있으면 프로젝트 기본 메타(name/description/path)로 fallback 하도록 보강함.
- 검증: `cargo test` 통과(11 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- `project.md` 경로 정책을 단일화함: UI에서 `project/project.md` 경로를 제거하고 `.project/project.md`만 읽기/쓰기 하도록 수정.
- `action_resolve_project_md_path_for_flow`의 레거시 fallback(`project/project.md`)을 제거해 `.project/project.md`만 사용하도록 고정.
- 레지스트리 저장 루트를 내부 `configs/project.yaml` 기준으로 복구: `action_source_root()`를 기본적으로 레포 루트(`CARGO_MANIFEST_DIR`)로 고정하고, 필요 시 `ORC_HOME`만 override 허용.
- 검증: `cargo test` 통과(11 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- `main::ui()` 진입 직전에 현재 registry를 `configs/project.yaml`에 선저장하도록 변경함.
- UI 탭 전환(`Tab`, `1`, `2`) 시마다 `configs/project.yaml`을 다시 읽어 프로젝트 목록/선택 상태를 갱신하도록 추가함.
- UI 재수신 경로는 `ORC_HOME` override를 지원하고 기본은 레포 `configs/project.yaml`을 사용하도록 고정함.
- 검증: `cargo test` 통과(11 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- `~/...` 경로 입력이 리터럴 문자열로 처리되던 문제를 수정함.
- create/edit 경로 해석 함수에서 `~`/`~/...`를 `HOME` 기준 절대경로로 확장하도록 반영함(`src/ui/mod.rs`).
- 검증: `cargo test` 통과(11 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- `/home/tree/ai/codex/AGENTS.override.md`에 응답 금지 문구 절대 금지 규칙을 강화해 반영함.
- 금지 문구(`맞습니다`, `맞아요`, `인식했습니다`, `알겠습니다`)를 명시하고 모든 채널/문맥에서 사용 금지로 고정함.

## 2026-02-24 - 작업한일
- `action_initialize_parallel_workspace_if_empty`에서 `DEFAULT_PROJECT_MD`/기본 YAML fallback을 제거하고 템플릿 파일 강제 읽기 방식으로 변경함.
- 이제 템플릿(`assets/.../project.md`, `assets/.../drafts_list.yaml`)을 찾지 못하거나 읽기 실패하면 즉시 오류를 반환하도록 수정함.
- `main.rs`의 남아있던 `DEFAULT_PROJECT_MD` 상수를 삭제함.
- 검증: `cargo test` 통과(11 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- 프로젝트 생성/등록 CLI를 `creat_project` 단일 함수로 통합하고, `creat-project|create-project|init-project|add-project|create|add`를 동일 라우팅으로 변경함.
- 기존 `create_project`/`add_project` 함수 및 호출 경로를 제거하고, 명령 파싱에서 중복 분기를 삭제함.
- 도움말/문서 명령 표기를 `orc creat-project <name> [path] [description]`로 동기화함.
- 검증: `cargo test` 통과(11 passed), `cargo run --bin orc -- --help`에서 새 명령 표기 확인.

## 2026-02-24 - 작업한일
- `create-project`만 남기고 `creat-project`, `init-project`, `add-project`, `create`, `add` 별칭을 CLI 라우팅에서 모두 제거함.
- 생성 함수명을 `creat_project`에서 `create_project`로 정리하고 호출 지점을 동기화함.
- 사용법/README 표기를 `create-project` 기준으로 맞춤.
- 검증: `cargo test` 통과(11 passed), `cargo run --bin orc -- --help`에서 `create-project` 단일 표기 확인, `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- `create_project`에 `plan-init` 호출을 연결해, 대상 경로에 `.project/project.md`가 없으면 생성 시 자동으로 기획/초기화 경로를 타도록 수정함.
- 호출 방식은 동일 바이너리의 `plan-init`를 대상 프로젝트 디렉터리에서 실행하고, 비대화 입력(빈 spec/goal/rule/features)을 stdin으로 전달하도록 구성함.
- 기존 레지스트리(`configs/project.yaml`) upsert/selected 저장 동작은 유지함.
- 검증: `cargo test` 통과(11 passed), `cargo run --bin orc -- --help` 확인, `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- `create-project`에서 `plan-init`를 별도 프로세스로 감싸 실행하던 래퍼(`action_run_plan_init_in_dir`)를 제거함.
- `create-project`가 직접 LLM 기반 `project.md` 생성/검증/저장/tasks 동기화/bootstrap을 수행하도록 `action_generate_project_plan` 공용 코어를 추가함.
- `plan-init`도 동일 코어를 사용하도록 맞춰, 두 경로의 YAML/MD 생성 규칙과 결과 처리가 동일하게 유지되도록 정리함.
- 검증: `cargo test` 통과(11 passed), `cargo run --bin orc -- --help` 확인, `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- `plan-init` CLI 분기와 `plan_init` 함수를 제거하고, 생성 초기화 흐름을 `create-project` 단일 경로로 통합함.
- UI 생성 경로의 내부 호출도 `plan-init` 실행 대신 `create-project` 실행으로 교체함.
- README/CLI help에서 `plan-init` 표기를 제거해 명령 노출과 실제 구현을 일치시킴.
- 검증: `cargo test` 통과(11 passed), `cargo run --bin orc -- --help` 확인, `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- `create-project`에 `spec/goal/rule/features` 입력 수집 단계를 직접 병합해, 신규 `project.md` 생성 시 질문 후 LLM 생성이 실행되도록 복구함.
- UI 내부의 비대화 실행 경로는 `ORC_NON_INTERACTIVE=1` 환경변수로 질문을 건너뛰고 빈값으로 생성하도록 처리해 UI 멈춤을 방지함.
- 검증: `cargo test` 통과(11 passed), `cargo run --bin orc -- --help` 확인, `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- UI의 `create-project` 내부 호출에서 경로 인자를 `.` 대신 절대경로로 전달하도록 변경해 `configs/project.yaml` 등록값이 상대경로로 깨지지 않게 수정함.
- `create_project`의 레지스트리 등록 호출(`action_upsert_project` -> `action_save_registry`)은 유지되어 생성 시점에 `configs/project.yaml` 반영이 계속 수행됨.
- 검증: `cargo test` 통과(11 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- 프로젝트 생성 직후 AI 창이 `READY` warmup으로 끝나지 않도록, 신규 생성 시 `action_open_ai_onboarding_modal`을 열어 즉시 질문형 온보딩 LLM 대화를 시작하도록 변경함.
- 온보딩 프롬프트에 `spec/goal/rule/features` 수집 -> `$build_domain` 기준 도메인 제안/확정 -> 최종 `project.md` 전체 출력 순서를 명시함.
- 검증: `cargo test` 통과(11 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- 생성 직후 `project.md format invalid`가 발생하던 원인을 분리: UI의 `create-project` 내부 호출에 `ORC_DEFER_PROJECT_PLAN=1`을 주입해 초기 즉시 생성/검증을 지연시킴.
- 이제 UI 생성 단계는 프로젝트 등록(`configs/project.yaml`) 후 온보딩 LLM 대화창을 먼저 열고, 대화 완료 시점의 문서 반영 경로를 사용하도록 정리함.
- 검증: `cargo test` 통과(11 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- `create_project` 초기화 순서를 조정해 `.project/project.md` 생성 전에 `configs/project.yaml` 등록(upsert + selected 저장)을 먼저 수행하도록 변경함.
- 결과적으로 프로젝트 생성 중 LLM/포맷 단계에서 실패하더라도 레지스트리에는 프로젝트 정보가 선반영됨.
- 검증: `cargo test` 통과(11 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- Draft pane(stage_draft)에서 `.project/feature`가 비어 있거나 planned 대비 파일이 누락된 경우에도 `enter_draft`로 `create-draft`를 직접 실행하도록 분기를 변경함.
- Draft pane에서 `a` 키로 `add_draft` 입력 모달을 열 수 있게 추가하고, 상태바 도움말을 `a add_draft`, `b enter_parallel(빈 draft면 create-draft 선실행)`으로 갱신함.
- 검증: `cargo test` 통과(11 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- `project.md` 생성 후 `## plan` 항목이 5개 미만이면 `feature_1..` 형식의 기본 항목을 채워 최소 5개를 보장하도록 추가함(Planned pane 최소 5개 표시 보장).
- UI Draft bulk add 실행에서 `add-function` 호출을 제거하고, 입력 객체를 분해해 `add-draft <feature> <request>` 반복 호출만 사용하도록 변경함.
- Plan pane 안내 문구의 `a add-function` 표기를 `a add_draft`로 정정함.
- 검증: `cargo test` 통과(11 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- `~/temp/.project` 점검 결과 `project.md`, `chat.log`만 있고 `tasks_list.yaml`이 없음을 확인함.
- 원인: UI 경로에서 `.project/project.md`를 저장해도 `tasks_list.yaml` 동기화 호출이 누락되어 파일이 생성되지 않던 구조.
- 수정: 
  - AI 응답으로 `project.md` 저장 성공 시 즉시 `action_sync_project_tasks_list_from_project_md` 호출하도록 연결.
  - `action_load_tasks_list_doc`에서 `tasks_list.yaml`이 없으면 `project.md` 기준 동기화를 먼저 시도한 뒤 재로딩하도록 보강.
- 검증: `cargo test` 통과(11 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- “최소 5개” 요구를 함수 보정이 아닌 LLM 지시로만 처리하도록 변경: `action_generate_project_plan` 프롬프트와 UI 온보딩/상세 프롬프트에 `## plan 최소 5개` 규칙을 명시함.
- 코드 강제 보정 로직(`calc_ensure_min_plan_items`, `planned_keys.len()<5` 자동 주입)을 제거함.
- `tasks_list.yaml`이 이미 존재해도 로드 시마다 `project.md` 동기화를 먼저 시도하도록 변경해 빈 planned 상태 고착을 완화함.
- 검증: `cargo test` 통과(11 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- 사용자 요청에 맞춰 UI 온보딩/상세 프롬프트의 “전체 project.md 출력” 지시를 제거하고, spec+domain 확정 시 `둘다 완료되었습니다. 다음으로 진행하세요.` 한 줄 메시지만 출력하도록 변경함.
- 요청된 “5개 보정은 함수가 아니라 프롬프트 지시” 정책을 유지한 상태로 문구를 정리함.
- 검증: `cargo test` 통과(11 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-24 - 작업한일
- Draft pane(포커스 4)에서 `create-draft` 실행 시 확인 모달을 실제로 열도록 `action_open_draft_create_confirm` 경로를 복구하고, Enter/`b` 분기를 동일 동작으로 정리함.
- `tasks_list.yaml`이 placeholder planned(`project_project_md` 등)로 초기화된 경우 `project.md` 동기화가 영구 스킵되던 문제를 수정: placeholder 패턴 감지 시 강제 재동기화(기존 planned/features/planned_items 초기화 후 재구성)하도록 변경함.
- 회귀 테스트 추가: placeholder 상태의 `tasks_list.yaml`이 `project.md` 기준 planned 항목으로 정상 치환되는 케이스를 단위 테스트로 고정함.
- 검증: `cargo test` 통과(12 passed).

## 2026-02-24 - 작업한일
- `create-draft` 실행 시작 시 `action_sync_project_tasks_list_from_project_md(.)`를 선호출하도록 변경해, placeholder planned(`project_project_md` 등)가 남아있어도 실제 `.project/project.md` 기준으로 자동 치유 후 draft 생성을 진행하게 수정함.
- 결과적으로 `tasks_list.yaml` stale 상태 때문에 ORC 내부용 planned 값으로 draft가 생성되던 경로를 차단함.
- 검증: `cargo test` 실행.

## 2026-02-24 - 작업한일
- 초기 create form에서 입력한 `spec` 값이 온보딩 시작 프롬프트에 전달되도록 연결함(`initial_spec` 전달). 이제 `react, @react-three/fiber, three-fiber, zustand` 같은 값이 첫 질문 컨텍스트에 포함됨.
- 온보딩 완료 신호(`둘다 완료되었습니다. 다음으로 진행하세요.`) 수신 시 대화 이력을 기반으로 `project.md`를 최종 생성/검증/저장하고 `tasks_list.yaml` 동기화 후 다음 단계로 진행하도록 보강함.
- `project.md` 파서 테스트 추가: spec 값에 `-`, `,`가 포함된 문자열을 그대로 파싱하는 케이스를 단위 테스트로 고정함.
- 검증: `cargo test` 통과(13 passed).

## 2026-02-24 - 작업한일
- 온보딩 대화 프롬프트에 수집 상태(spec/domain/features_count)를 명시하고, completion_ready=true일 때만 완료 문구를 허용하도록 규칙을 강화함.
- 완료 문구(`둘다 완료되었습니다...`)가 와도 실제 수집 상태가 미달(spec/domain/features<3)이면 finalize/다음 단계 전환을 차단하도록 런타임 가드를 추가함.
- 결과적으로 domain만 입력된 상태에서 조기 종료 반복되던 흐름을 막고, 누락 정보 수집이 계속되도록 보정함.
- 검증: `cargo test` 통과(13 passed).

## 2026-02-24 - 작업한일
- 온보딩 `initial_spec`를 `AiChatModal` 상태로 보관하도록 추가해, 첫 질문 이후 프롬프트/수집판정/최종 project.md 생성 프롬프트 전 구간에서 동일 spec 힌트를 참조하게 수정함.
- `spec` 수집 판정(`calc_collect_onboarding_signals`)에 `modal.initial_spec`를 포함해, 사용자가 도메인/기능만 먼저 입력해도 초기 form spec이 누락으로 판정되지 않도록 보정함.
- 회귀 테스트 추가: `initial_spec=react,zustand,three-fiber`일 때 spec_ready=true로 판정되는 케이스를 단위 테스트로 고정함.
- 검증: `cargo test` 통과(14 passed).

## 2026-02-24 - 작업한일
- 온보딩 AI 스트림에서 완료 문구가 chunk 단계로 먼저 도착했을 때(`둘다 완료되었습니다...`) 조건 충족(spec/domain/features) 시 즉시 스트림을 정리하고 finalize 경로를 실행하도록 보강함.
- 기존처럼 Done 이벤트 대기 상태에 묶여 모달이 `AI 응답 수신중...`으로 고착되는 케이스를 차단함.
- 검증: `cargo test` 통과(14 passed).

## 2026-02-24 - 작업한일
- 대화 종료 후 bootstrap confirm 모달의 spec이 빈값으로 표시되던 문제 수정: `project.md`의 spec이 비어 있으면 AI 온보딩의 `initial_spec`을 fallback으로 사용하도록 `action_open_bootstrap_confirm_with_spec_hint`를 추가함.
- `action_close_ai_chat_modal_and_open_bootstrap`에서 모달 종료 전에 `initial_spec`을 추출해 bootstrap confirm 호출로 전달하도록 연결함.
- 검증: `cargo test` 실행.

## 2026-02-24 - 작업한일
- Project detail pane fallback 매핑 오류 수정: `project.md` core info가 없을 때 `Spec`에 `project.path`가 표시되던 버그를 제거하고 `spec not set`/`goal not set`으로 교체함.
- 결과적으로 Project pane에서 경로가 Spec으로 잘못 노출되는 문제를 해결함.
- 검증: `cargo test` 실행.

## 2026-02-24 - 작업한일
- 정보 표시 Pane 전체(Project/Rule/Constraint/Feature/Plan/Drafts) 데이터 매핑 검증을 위한 통합 테스트(`detail_panes_data_mapping_is_consistent`)를 추가함.
- 테스트는 `project.md`, `tasks_list.yaml`, `.project/feature/*/(task|draft).yaml`를 구성해 각 Pane 소스 함수(`action_parse_project_md`, `action_collect_feature_items_from_drafts`, `action_collect_planned_*`, `action_collect_generated_draft_items_from_project`)가 기대값을 반환하는지 검증함.
- 검증: `cargo test` 통과(15 passed).

## 2026-02-24 - 작업한일
- Pane 매핑 테스트를 사용자 요구 구조로 재구성함.
  1) `VirtualPaneInput` 객체에 가상 입력(project.md/tasks_list/generated file 정보) 구성
  2) `collect_display_values_from_virtual_input`으로 UI 표시 소스값을 `DisplayPaneValues` 객체로 수집
  3) 두 객체의 필드 매핑 일치 여부를 `detail_panes_data_mapping_is_consistent`에서 검증
- 검증: `cargo test` 통과(15 passed).

## 2026-02-24 - 작업한일
- `draft add` 동작을 Draft pane(포커스=5)에서만 허용하고, 생성된 draft item이 1개 이상 있을 때만 실행되도록 제한함. item이 없으면 상태바에 차단 메시지를 표시함.
- 상태바 문구를 조정해 `add_draft` 안내는 Draft pane(포커스=5)에서만 노출되게 변경하고, Plan pane(포커스=4)에서는 `b create-draft`만 표시되도록 수정함.
- pane 객체(`DetailLayoutPanelDoc/DetailLayoutPanel`)에 `shortcut: String` 속성을 추가하고, 선택된 pane의 shortcut 문자열을 status bar에 자동 표시하도록 변경함.
- 기본 레이아웃/`assets/layouts/code.yaml`에 shortcut 값을 채움:
  - project: `enter: move-detail`
  - rule: `enter: edit-rule`
  - constraint: `enter: edit-constraint`
  - features: `enter: edit-feature`
  - drafts: `b: create-draft/enter-parallel`
- shortcut 매핑 검증 테스트(`detail_layout_panel_shortcut_is_compiled_and_selected`) 추가.
- 검증: `cargo test` 통과(16 passed).

## 2026-02-24 - 작업한일
- `tasks_list.yaml`에 draft 진행 상태 객체(`draft_state.generated`, `draft_state.pending`)를 추가해, 이미 생성된 draft와 아직 생성되지 않은 planned를 분리 저장하도록 확장함.
- `create-draft` 실행 시 상태를 선반영/중간반영/실패반영 하도록 변경:
  - 시작 전 상태 저장
  - 각 feature 처리 후 상태 저장
  - 중간 실패 시점에도 현재까지 상태 저장 후 에러 반환
- draft 상태 계산 함수(`action_sync_draft_state_doc`)를 추가해 `.project/feature/*` 실파일을 기준으로 generated/pending을 계산하도록 구현함.
- UI의 `DraftsListDoc`에도 동일 스키마(`draft_state`)를 추가해 읽기/쓰기 호환을 유지함.
- 검증 테스트 추가: `sync_draft_state_doc_tracks_generated_and_pending`.
- 검증: `cargo test` 통과(17 passed).

## 2026-02-24 - 작업한일
- planned item 동기화 시 `## plan/## features` 항목에서 `|`가 포함된 문장은 첫 세그먼트를 key 후보로 추출하도록 수정해 파일 경로/결과 문구가 key로 섞이는 문제를 차단함.
- `calc_is_feature_key_like`에 파일경로형 패턴(`src_*`, `*_ts`, `*_tsx`, `*_md` 등) 차단을 추가해 경로성 문자열이 "이미 유효한 기능 키"로 우회되지 않도록 보정함.
- 회귀 테스트 2건 추가:
  - `extract_project_md_list_prefers_action_segment_before_pipe`
  - `feature_key_like_rejects_fileish_path_style_names`
- 검증: `cargo test` 대상 테스트 3건 통과.

## 2026-02-24 - 작업한일
- `tasks_list.yaml`가 초기 placeholder 상태에서 `sync_initialized: true`로 고정되면 이후 `.project/project.md` 완성본과 불일치해도 재동기화를 건너뛰던 문제를 수정함.
- 강제 재동기화 조건을 추가:
  - `planned_items.value`가 템플릿성 문구(`프로젝트 정보 입력`, `features 리스트 입력`, `draft.yaml 읽기`)일 때
  - `project.md`에서 추출한 features/planned key 집합과 현재 tasks_list key 집합의 교집합이 0일 때
- fallback key 생성 보강: `동사_명사` 유효 key가 아니면 해시(`plan_feature_<hash>`)로 강제해 `task` 같은 단일 키 충돌을 방지함.
- 회귀 테스트 추가: `sync_project_md_overrides_stale_template_like_tasks_list`.
- 검증: 관련 sync 테스트 3건 통과.

## 2026-02-24 - 작업한일
- draft 목록 기준 파일을 `drafts_list.yaml`로 고정하고, 기존 `tasks_list.yaml`은 레거시 호환 경로로만 유지하도록 변경함.
- 주요 동작 경로를 `drafts_list.yaml`로 전환:
  - `action_collect_project_features`
  - `action_sync_project_tasks_list_from_project_md`
  - `draft_create`
  - `add_plan`
  - `add_feature_to_planned`
  - `action_promote_planned_to_features`
  - parallel preflight 입력 경로
- `.project/tasks_list.yaml`이 기존에 존재하는 경우 `drafts_list.yaml` 미존재 시 자동 마이그레이션(복사)하도록 추가함.
- 저장 시 `drafts_list.yaml`을 우선 저장하고, 레거시 `tasks_list.yaml`이 존재하면 미러 저장하도록 보강함.
- UI 경로/문구도 `drafts_list.yaml` 기준으로 수정하고 로딩 우선순위를 `drafts_list.yaml -> tasks_list.yaml`로 변경함.
- 검증: `cargo test` 전체 통과(20 passed).

## 2026-02-24 - 작업한일
- 저장 파일명 표기를 `tasks_list.yaml`에서 `drafts_list.yaml`로 통일함.
- 문서/템플릿 갱신:
  - `AGENTS.md` 내 init/draft 단계 파일명 표기를 `drafts_list.yaml` 기준으로 수정.
  - `assets/code/templates/stage.md` 출력 항목 표기를 `drafts_list.yaml.planned`로 수정.
- 코드 갱신:
  - 레거시 `tasks_list.yaml` 상수/마이그레이션/미러 저장 로직 제거.
  - draft list 로더 fallback에서 `tasks_list.yaml` 제거, `drafts_list.yaml` 단일 로딩으로 변경.
- 검증: `cargo test` 전체 통과(20 passed).

## 2026-02-24 - 작업한일
- planned name 생성 경로에 "한글 기능문장 -> 영문 축약 -> 도메인 기반 네이밍" 절차를 반영함.
- `project.md`의 `# Domains` 블록에서 도메인명(`- **name**:`)을 추출하는 함수(`calc_extract_project_md_domain_names`)를 추가함.
- planned item 생성 LLM 프롬프트를 강화:
  - 한국어 문장 영문화
  - 2~4 토큰 축약
  - 현재 가능한 도메인 목록 기반 도메인 선택
  - `<domain>_<verb>_<noun>` 또는 `<verb>_<noun>` 네이밍 규칙 명시
- sync 경로에서 project.md 도메인 + drafts list 도메인을 합쳐 planned item 생성에 전달하도록 연결함.
- 회귀 테스트 추가: `extract_project_md_domain_names_reads_domain_blocks`.
- 검증: `cargo test` 전체 통과(21 passed).

## 2026-02-24 - 작업한일
- LLM 비활성/실패 fallback 시 planned key 접두사를 `plan_feature_`에서 `func_`로 변경함.
- 적용 함수: `calc_fallback_feature_key` (`func_<8hex>` 형식).
- 관련 테스트 fixture 문자열도 `func_` 접두사로 동기화함.
- 검증: `cargo test` 전체 통과(21 passed).

## 2026-02-27 - 작업한일
- `main.rs`의 프로젝트/드래프트/플랜 커맨드 핸들러를 분리하기 위해 `src/project.rs`, `src/draft.rs`, `src/plan.rs`를 추가하고, `auto_mode/create-project/select-project/delete-project/create-draft/add-draft/delete-draft/add-plan` 로직을 해당 모듈로 이동함.
- `create-draft`가 `planned_items.value`를 설명 입력으로 사용하고, 폴더/feature key는 `planned`의 name을 그대로 사용하도록 변경해 단일 이름으로 뭉개지는 문제를 방지함.
- 한국어 기능 문장에서 해시 키(`func_xxx`)만 생성되던 fallback을 보완하기 위해 키워드 기반 snake_case 매핑을 추가함.
- `action_sync_project_tasks_list_from_project_md` 저장 직전에 `draft_state.generated/pending`를 재계산하도록 수정해 planned 대비 pending 상태가 stale로 남지 않게 함.
- UI에서 `create-draft` 실행 후 status line이 고정 문구만 보이던 동작을 개선해 CLI stdout 결과를 그대로 표시하도록 수정함.
- 검증: `cargo test` 통과(21 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-27 - 작업한일
- 상단 `Current Pane` 헤더 border 색을 활성색과 분리해 `normal` 팔레트(검은색)로 고정하고, pane 활성 border는 `lightmagenta`(밝은 보라)로 보이도록 `src/ui/component.rs`, `src/ui/mod.rs`, `configs/style.yaml`, `assets/style/pane_style.yaml`을 수정함.
- 색상 파서에 `lightmagenta/light_magenta/brightmagenta/bright_magenta`를 추가해 스타일 설정으로 밝은 보라색 지정이 가능하도록 확장함.
- `build-parallel-code`의 feature-level 의존성 수집에서 `task[].depends_on`을 잘못 끌어오던 로직을 제거하고 top-level `draft.depends_on`만 사용하도록 `src/main.rs`를 수정해 다중 draft 병렬 완료가 막히던 문제를 해결함.
- 검증: `cargo test` 통과(21 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-27 - 작업한일
- `create-draft` 루프 중 실패 시 `drafts_list.yaml.planned`를 생성 완료 항목으로 덮어쓰던 로직을 제거하고, planned 원본을 유지한 채 `draft_state.generated/pending`만 동기화하도록 `src/draft.rs`를 수정함.
- 실패 메시지에 실패 feature key를 포함(`create-draft failed at <feature>`)해 어떤 항목에서 중단됐는지 즉시 추적 가능하게 개선함.
- check-code 검증 규칙에 맞춰 `.project/scenario.md`를 `명령 | 실행/변경 파일 | 파생 결과` 한 줄 형식으로 정규화함.
- 검증: `cargo test` 통과(21 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-27 - 작업한일
- `create-draft` 검증 실패 시 대상 feature 폴더(`.project/feature/<feature>/failure.md`)에 실패 원인을 기록하도록 `src/draft.rs`에 failure 리포트 작성/정리 로직을 추가함.
- `create-draft` 프롬프트 지시문을 강화해 `rule`/`contracts`를 검증 가능한 형식으로 출력하도록 명시함.
- 구조화 계약식 검증에서 `key=value` 형식을 허용하도록 `calc_is_structured_constraint`의 연산자 판별을 보강함(`=` 허용).
- `/home/tree/temp`에서 재실험하여 `failure.md` 생성과 원인 기록을 확인했고, 이후 재실행에서 `func_48d25650`/`jump_motion`/`jump_state_store`/`win_condition_check`/`win_screen`까지 생성 진전됨을 확인함(대기 구간은 `menu_setup` LLM 응답 대기).
- 검증: `cargo test` 통과(21 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-27 - 작업한일
- `config.yaml`/`configs.yaml`/`src/config/mod.rs`에 `debug` 설정(on/off, 기본 true)을 추가함.
- debug on일 때 `create-draft`와 `add-draft` 프롬프트에 `DEBUG_LOG` 선행 출력 지시를 주입하도록 `src/draft.rs`를 수정함.
- debug on일 때 `build-parallel-code`의 작업 프롬프트에도 `DEBUG_LOG` 진행/대기 로그 지시를 주입하도록 `src/main.rs`의 `action_build_task_prompt`를 수정함.
- `/home/tree/temp` 재실행으로 debug 지시 반영을 확인했고(`chat.log`에 `DEBUG_LOG` 출력), 해당 응답에서 템플릿 스키마 이탈(`name/description/type...` 단일객체 + unquoted `step` 콜론 문자열)로 YAML 파싱 실패가 발생함을 확인함.
- 검증: `cargo test` 통과(21 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-27 - 작업한일
- debug on 상태에서 `build-parallel-code` task 시작 시 `.project/runtime/<task>.log`를 생성/append하도록 `src/parallel/mod.rs`에 task runtime 로거를 추가함.
- task 실행 경로에 단계별 로그를 기록하도록 연결함:
  - 시작/프롬프트 전송
  - LLM 응답 수신
  - 검증 단계
  - 파일 반영 단계(성공 시)
  - 완료/실패(성공/에러/타임아웃 포함)
- `run_parallel_build_code`에서 config의 `debug` 값을 전달해 debug off 시 로그 기록을 완전히 비활성화하도록 구성함.

## 2026-02-28 - 작업한일
- `create-draft` 병목(LLM 무기한 대기) 추적을 위해 feature 단위 runtime 로그를 `src/draft.rs`에 추가함: `.project/runtime/<feature>.log`에 시작/응답/검증/파일반영/완료/실패 단계 기록.
- `create-draft`의 LLM 호출에 강제 타임아웃을 적용함(`src/main.rs`): `Command::output()` 블로킹 대신 spawn+try_wait 루프로 timeout 제어, timeout 시 kill 후 에러 반환.
- `action_run_codex_exec_capture_with_timeout`를 추가하고, draft 생성 경로에서 30~240초 범위(clamp) timeout을 사용하도록 연결함.
- 설정 파일 로딩 정책을 수정해 실행 cwd(`./config.yaml`)를 더 이상 읽지 않고, 바이너리 소스 루트(`action_source_root`) 기준 설정만 참조하도록 `action_load_app_config` 후보를 정리함.
- `/home/tree/temp` 재실험에서 `.project/runtime/start_button_click_handle.log`에 `시작/프롬프트 전송` 기록 후 응답 단계 미진입 병목을 확인함.

## 2026-02-28 - 작업한일
- 전역 옵션 `draft_retry_on_fail`을 추가함(`src/config/mod.rs`, `config.yaml`, `configs.yaml`).
- `create-draft`에서 실패 시 즉시 중단하던 기존 동작을 보강해, `draft_retry_on_fail=true`일 때:
  - 전체 planned를 1차 시도하고
  - `draft_state.pending`을 기준으로 2차 재시도(1회)를 수행하도록 `src/draft.rs`를 수정함.
  - 2차 이후에도 실패가 남으면 `create-draft retry exhausted; pending: ...` 에러를 반환하도록 처리함.
- 설정 경로 일관성을 위해 `src/ui/mod.rs`의 모델 설정/레지스트리 경로에서 `ORC_HOME`/cwd fallback을 제거하고 `action_source_root()` 기준 경로만 사용하도록 수정함.
- `/home/tree/temp` 재테스트 결과:
  - `start_button_click_handle`는 생성 완료(검증 실패 자동 보정 1회 포함)까지 진행됨.
  - 다음 feature(`start_overlay_render`)에서 `시작/프롬프트 전송` 후 `LLM 응답 수신` 전 대기 병목이 재현됨.

## 2026-02-28 - 작업한일
- `create-draft`를 2단계로 보강함(`src/draft.rs`):
  - 1차: feature별 LLM 초안 생성을 병렬 스레드로 실행
  - 2차: 생성된 YAML에 대해 의존관계/scope(빈값, 중복, 비파일형 경로) 점검 후 파일 반영
- debug 모드 무응답 보호를 추가함:
  - `.project/runtime/<feature>.log`에 15초 간격 heartbeat(`무응답 보호`) 로그 기록
- CLI 확장:
  - `orc detail-project -d <description> -s <spec> [--llm <bin>]`
  - `orc auto -d <description> -s <spec>` (현재 폴더 기준 프로젝트 생성 -> project.md 보강 -> draft 생성까지 일괄)
  - 관련 사용법을 `src/cli/mod.rs` help 출력에 동기화
- config 경로 정책 정합화:
  - `src/ui/mod.rs`의 모델/레지스트리 로딩 경로에서 cwd/ORC_HOME fallback 제거, `action_source_root()` 기준으로 통일
- project.md 검증 유연화/정규화:
  - `# Flow` 또는 `# Stage` 허용
  - 누락 헤더/도메인 블록을 최소 형식으로 보강하는 정규화 로직 추가
- React bootstrap 엔트리 경로 버그 수정:
  - JS 템플릿 생성 시 `index.html`이 `/src/main.jsx`를 참조하도록 수정(`src/ui/mod.rs`)
- `/home/tree/temp2` 실검증:
  - `orc auto -d \"todo app\" -s \"react,zustand,shadcn\"` 완료 확인
  - runtime 로그에서 무응답 보호/재시도 기록 확인
  - Playwright 테스트(`tests/todo.spec.ts`) 작성/실행 결과:
    - 기본 화면 로드: 통과
    - Todo 입력 UI 존재: 실패(현재 생성된 앱이 hello-world 수준으로 Todo UI 미구현)

## 2026-02-28 - 작업한일
- check-code 후속 점검 병목 추적을 위해 `src/main.rs` `action_run_check_code_after_draft_changes`를 보강함:
  - `--dangerously-bypass-approvals-and-sandbox` 적용 경로 유지(`action_run_codex_exec_capture_with_timeout` 경유)
  - 전용 타임아웃(30~300s clamp) 적용
  - debug on 시 15초 heartbeat 로그(`.project/runtime/check-code.log`) 추가
  - 시작/응답수신/완료/실패 단계 로그를 남기도록 개선
- `action_run_codex_exec_capture_in_dir_with_timeout`를 추가하고 `auto-improve`에서 300초 타임아웃으로 실행되게 변경함.
- 새 CLI 단계 추가:
  - `orc auto-check`: Playwright 기반 점검 실행 후 `.project/runtime/auto-check.md` 리포트 생성
  - `orc auto-improve <request>`: 사용자 요청 기반 코드 보완 실행 후 `.project/runtime/auto-improve.md` 리포트 생성
  - `orc draft-report`: draft 폴더 전수 점검(의존/중복 task/touch 충돌) 후 `.project/runtime/draft-report.md` 생성
- `orc auto -d/-s` 흐름에 `draft-report` 단계를 통합하고, draft-create 실패 시에도 보고 단계를 실행하도록 조정함.
- `/home/tree/temp2`를 재생성해 실제 실행 검증:
  - `orc auto -d \"todo app\" -s \"react,zustand,shadcn\"` 실행 완료
  - `check-code.log`에서 check-code 대기 지점과 heartbeat/응답수신을 확인
  - `orc auto-check` 실행(현재 테스트 미존재로 실패 보고서 생성)
  - `orc auto-improve` 실행(300초 타임아웃으로 종료)
  - `orc draft-report` 실행 및 리포트 생성 확인

## 2026-02-28 - 작업한일
- bootstrap 하드코딩 템플릿 경로를 제거하고 프롬프트 파일 기반 LLM bootstrap으로 전환함.
  - 추가 파일: `assets/prompts/bootstrap.txt`
  - `action_apply_bootstrap_by_spec`가 위 프롬프트를 로드해 `project.md`와 `spec`을 전달하고 LLM이 직접 초기 파일/엔트리를 생성하도록 변경.
- UI bootstrap prepare 경로(`action_run_bootstrap_llm_prepare`)도 동일 프롬프트 파일(`assets/prompts/bootstrap.txt`)을 사용하도록 변경.
- 기존 하드코딩 bootstrap 함수들(react/node/rust 템플릿 직접 생성 로직)을 제거함.
- bootstrap 프롬프트 경로 해석 기준을 실행 바이너리 경로가 아닌 소스 루트(`action_source_root`)로 고정해 설치 실행 환경에서도 프롬프트를 찾도록 수정.
- 관련 테스트 갱신:
  - 하드코딩 템플릿 테스트 3건 제거
  - `bootstrap_prompt_template_exists` 테스트 추가
- `/home/tree/temp2` 재실행에서 `bootstrap completed via llm: BOOTSTRAP_DONE: ...` 응답을 확인해 프롬프트 기반 bootstrap 동작을 검증함.

## 2026-02-28 - 작업한일
- `func_8b18d07b`(할 일 생성 기능) 초안 작성을 위해 `.project/feature/func_8b18d07b/draft.yaml`을 템플릿 복사 후 주석/예시 제거 방식으로 채우고, 제목 비어있음 금지/상태값 제한/zustand-저장소 동기화 규칙 및 action step/contract를 추가함.
## 2026-02-28 - 작업한일
- `func_93c4e967`(할 일 수정/삭제 기능) draft를 템플릿 기반으로 `.project/feature/func_93c4e967/draft.yaml`에 생성하고, 규칙/step/contract를 자동 검증 가능한 식 중심으로 채움.
- 검증: `cargo test` 통과(21 passed).

## 2026-02-28 - 작업한일
- `orc auto -d ... -s ...` 병목을 `~/temp`에서 재현해 1차 LLM 호출이 `project.md 생성기` 단계에서 `spec` 빈 값으로 호출되는 것을 확인함.
- 원인 수정: `src/project.rs` `auto_bootstrap`에서 `create_project`가 먼저 기본 plan 생성(빈 spec)하지 않도록, `create_project_with_defer_option(..., true)` 경로를 추가해 auto 경로에서는 초기 plan 생성을 defer 처리.
- `create_project`는 기존 동작(환경변수 `ORC_DEFER_PROJECT_PLAN`)을 유지하도록 wrapper로 정리해 기존 CLI 동작 호환성을 유지함.
- 추가 보강: `src/main.rs` `action_run_llm_exec_capture`를 타임아웃 실행(`action_run_command_with_timeout`, 30~300s clamp)으로 변경해 LLM 무응답 시 무한 대기 대신 실패로 반환되도록 수정.

## 2026-02-28 - 작업한일
- `orc auto`에 자동 재시도/폴백 경로를 추가함:
  - `src/project.rs`에서 plan 생성을 최대 3회(기본 1회 + 재시도 2회) 시도.
  - 반복 실패 시 `manual bootstrap` 트리거 + `auto-improve`를 순차 실행하고 `.project/runtime/auto-bootstrap-fallback.md` 보고서를 남기도록 구현.
- LLM 응답 대기 시간을 config 기반으로 통일:
  - 기본값을 300초(5분)로 변경(`src/config/mod.rs`).
  - `main/draft/ui/project`의 주요 LLM 실행 경로가 공통 timeout 설정을 사용하도록 조정.
- `~/temp` 실검증(검증용 timeout 30초 임시 적용 후 원복):
  - `orc auto -d "todo manager app" -s "react,zustand,shadcn"` 실행 시 plan 3회 재시도 후 fallback 경로가 실행됨.
  - 결과: `auto bootstrap failed after retries ...`로 즉시 종료되고 `~/temp/.project/runtime/auto-bootstrap-fallback.md` 생성 확인.
  - 검증 후 `configs.yaml` timeout 기본값을 300초로 원복.

## 2026-02-28 - 작업한일
- 사용자 지시에 따라 fallback의 하드코딩 생성 로직을 제거하고 LLM 재실행 기반 fallback만 유지하도록 `src/project.rs`를 정리함.
- `project.md` 생성 프롬프트를 init/auto로 분리:
  - `assets/prompts/project-md-init.txt`
  - `assets/prompts/project-md-auto.txt`
  - `src/main.rs` `action_generate_project_plan(..., auto_mode)`가 모드별 프롬프트 파일을 로드해 사용하도록 변경.
- auto 모드 질문 금지 규칙을 prompt 파일들에 반영:
  - `assets/prompts/bootstrap.txt`
  - `assets/code/prompt/detail-project.txt`
  - `assets/code/prompt/tasks.txt`
- `~/temp` 재검증에서 `.project/chat.log`에 `너는 auto 모드용 project.md 생성기다`와 질문 금지 지시가 실제 주입되는 것을 확인함.

## 2026-02-28 - 작업한일
- LLM 실행 방식을 2가지로 분기하도록 추가:
  - 기존 직접 실행(`Command` 기반)
  - tmux pane 실행(디버그 on + tmux 세션 내)
- `src/main.rs`에 tmux 실행 경로(`action_run_llm_via_tmux`)를 추가하고, 다음 함수들이 조건부로 tmux 경로를 사용하도록 연결:
  - `action_run_llm_exec_capture`
  - `action_run_codex_exec_capture_with_timeout`
  - `action_run_codex_exec_capture_in_dir_with_timeout`
- tmux pane 정리를 위해 `src/tmux/mod.rs`에 `action_kill_pane` 함수를 추가.
- 검증: `cargo test` 통과(19 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-28 - 작업한일
- `drafts_list.yaml` 키가 `func_XXXXXXXX` 해시로 생성되는 원인을 분석하고 fallback 정규화 로직을 보강함.
- `src/main.rs`:
  - `calc_fallback_feature_key`가 한국어/혼합 문장에서도 의미 기반 키를 우선 생성하도록 수정.
  - todo/task 관련 키워드 매핑(`Todo/todo/task/생성/삭제/완료/토글/목록/검색/필터/영속화`)을 추가.
  - 단일 토큰 키는 `_task`를 붙여 snake_case 규칙을 만족하도록 보강.
- 검증: `cargo test -q` 통과(19 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-28 - 작업한일
- 사용자 요청에 따라 응답 금지 문구 규칙을 `AGENTS.md`에 강화함.
- `Response Phrase Rule`에 `맞습니다` 절대 금지(hard ban)와 변형 문구(`네, 맞습니다`, `맞습니다.`, `네 맞습니다`, `그렇습니다`) 금지를 명시적으로 추가.
- 동일 섹션에 전송 직전 금지어 재검사(`Pre-send guard`) 규칙을 추가해, 금지어가 포함되면 문장을 재작성 후 재검사하도록 강화.

## 2026-02-28 - 작업한일
- `auto_bootstrap` fallback을 4단계 실행 흐름으로 정렬함: (1) 단계 산출물 존재/미완성 확인 (2) `plan.md`에 원인/해결/검증 기록 (3) 동일 단계 LLM 재시도 (4) 재실행 검증.
- `src/project.rs`에 stage별 산출물 점검 함수(`action_fallback_stage_output_status`)와 `plan.md` 자동 갱신 함수(`action_append_plan_md_fallback_record`)를 추가함.
- plan 실패 fallback에서 수동 bootstrap/auto-improve 경로를 제거하고, LLM 재시도 전용 정책으로 고정함.
- draft-create도 auto 경로에서 재시도 루프를 적용하고, 실패 시 runtime 보고서(`draft-fallback.md`)를 남기도록 보강함.
- 검증: `cargo test -q` 통과(19 passed).

## 2026-02-28 - 작업한일
- `orc auto` 실행 시작 시 대상 프로젝트 루트에 `plan.md`를 선생성하도록 `src/project.rs`를 수정함.
- `auto_bootstrap`에 검증/피드백 단계를 추가함:
  - 검증: `.project/project.md`, `.project/drafts_list.yaml`, draft 파일 개수 확인
  - 피드백: `feedback.md`에 status/summary/verification 기록
- 실패 경로(plan 실패, draft 실패)에서도 `feedback.md`를 남기도록 처리함.
- `~/temp`에서 `orc auto -d "calculator app" -s "react,zustand"` 실검증:
  - `plan.md` 생성 확인
  - `check-code` 완료 확인
  - `feedback.md` 생성 확인
- 추가 보강: `draft-report`가 실패하면 최종 검증을 `fail`로 판단하도록 검증식 강화(기존 false positive 제거).
- 검증: `cargo test -q` 통과(19 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-28 - 작업한일
- 사용자 지시에 따라 `plan.md`를 먼저 작성하고(문제/해결책/검증), 이후 구현을 진행하도록 작업 순서를 정리함.
- `/home/tree/ai/codex/AGENTS.override.md`에 영구 규칙 추가:
  - Plan First Rule: 소스 수정 전 `plan.md` 선작성 강제
  - Retry Loop Rule: 문제/검증 설정 -> 해결 시도 -> 검증 -> 실패 시 전체 재시작
- `src/project.rs` `auto_bootstrap`를 전체 사이클 반복 구조로 보강:
  - cycle 상태 파일(`.project/runtime/auto-cycle.state`)로 반복 횟수 관리
  - plan/draft/verify 실패 시 `feedback.md` 기록 + auto-improve 시도 후 전체 플로우 재시작
  - 최대 반복 횟수(`AUTO_FULL_CYCLE_MAX`) 초과 시 실패 반환
- 검증 기준 강화:
  - 앱 산출물 파일(`package.json`/`Cargo.toml`/`pyproject.toml`/`go.mod`) 존재 여부를 포함
  - `draft-report` 실패는 검증 실패로 반영
- 검증: `cargo test -q` 통과(19 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-28 - 작업한일
- 사용자 지시대로 실패 원인 발생 시 해결책을 `plan.md`에 반드시 병합하도록 정책을 강화함.
- `/home/tree/ai/codex/AGENTS.override.md`에 `Failure-Solution Mandatory Rule (Highest Priority)`를 추가함.
- `src/project.rs`에 실행단 정책 하드가드(preflight) 추가:
  - `plan.md` 필수 섹션(문제/해결책/검증) 검사
  - `feedback.md` 존재 시 plan 최신성/병합 흔적 검사
  - 재시도에서 plan 변경이 없으면 프로세스 위반으로 중단
- `/home/tree/temp`에서 `orc auto` 반복 실행 검증 수행:
  - 실패 원인/해결책이 `plan.md`에 누적 병합됨
  - `feedback.md` 최신 상태는 pass(`app artifact exists`, `draft report pass`, `build-parallel-code pass`) 확인
  - `package.json` 생성 확인
- 검증: `cargo test -q` 통과, `cargo install --path /home/tree/project/rust-orc` 완료 상태 유지.

## 2026-02-28 - 작업한일
- 재시도 성공 경로에서 `verification | pass` 로그가 누락되던 문제를 수정함.
- `src/project.rs` retry 성공/실패 분기에서 `action_append_auto_bootstrap_log("verification", ...)`를 명시적으로 기록하도록 보강.
- 결과: 외부 루프 판별(`verification | pass` grep)과 내부 성공 상태가 일치하도록 정합화.
- 검증: `cargo test -q` 통과(19 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-28 - 작업한일
- draft 스키마 오류 원인을 프롬프트/템플릿에서 점검하고 수정함.
- 원인 확인:
  - `/home/tree/ai/skills/plan-drafts/references/draft.yaml` 템플릿이 깨져 있었음(`task.rule` 중복, `contract` 단수 오타).
  - 일부 프롬프트가 해당 외부 템플릿 경로를 참조해 잘못된 구조를 유도.
- 수정:
  - `src/draft.rs` draft 생성/보정 프롬프트 강화
    - 템플릿 경로를 `assets/code/templates/draft.yaml`로 통일
    - duplicate key 금지, 허용 task 키 목록 고정, `contract` 금지/`contracts` 강제
  - `src/main.rs` add-function draft 프롬프트 및 check-code draft 보정 프롬프트에 동일 제약 추가
  - 외부 스킬 템플릿(`/home/tree/ai/skills/plan-drafts/references/draft.yaml`) 자체를 정상 스키마로 교정
- 검증: `cargo test -q` 통과(19 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-28 - 작업한일
- 사용자 지정 draft.yaml 스키마로 전역 정합성 점검 및 갱신 수행.
- 반영 스키마:
  - `rule: []`
  - `features: { domain: [], flow: [] }`
  - `task[]: name,type,domain,depends_on,scope,rule,step,touches,contracts`
- 수정 파일:
  - `assets/code/templates/draft.yaml`에서 `task.flow`, `constraints` 제거 및 `step` 기본 형식을 리스트로 고정.
  - `src/main.rs` `DraftTask`에서 `flow`, `constraints` 필드 제거 + `DraftTask/DraftFeatures/DraftDoc`에 `deny_unknown_fields` 적용.
- 외부 스킬 템플릿(`/home/tree/ai/skills/plan-drafts/references/draft.yaml`)도 동일 스키마로 정렬되어 유지됨을 확인.
- 검증: `cargo test -q` 통과(19 passed), `cargo install --path /home/tree/project/rust-orc` 완료.

## 2026-02-28 - 작업한일
- `orc auto`의 `project.md` 생성 실패(`missing domain field - **rule**:`) 재현 원인을 보강함.
- 수정:
  - `src/main.rs` `action_normalize_project_md_min_sections`에 도메인 필수 키 자동 보정 항목 추가
    - `- **rule**:`
    - `- **variable**:`
  - `assets/prompts/project-md-auto.txt`, `assets/prompts/project-md-init.txt`에 스키마 강제 문구 추가
    - 필수 헤더 전체 명시
    - `### domain` + 필수 도메인 필드 6종 강제
    - 템플릿 참조를 `assets/code/templates/project.md` 기준으로 정렬
  - `src/main.rs`의 내장 fallback prompt도 동일 스키마 제약으로 동기화
- 기대 효과: LLM 출력이 흔들려도 normalize/validate 단계에서 자동 보정되고, auto 모드 재시도 중 schema fail 확률을 낮춤.

## 2026-02-28 - 작업한일
- `check-code` 점검 결과, auto 모드에서 `auto_improve`가 질문형 응답을 성공으로 처리할 수 있는 병목 위험을 확인.
- `src/project.rs` `auto_improve` 개선:
  - 프롬프트에 비대화형 계약 추가(질문/선택지/확인요청 금지).
  - 응답 말미 `RESULT: APPLIED|NO_CHANGE` 계약 강제.
  - 질문형 문구 감지 시 즉시 실패 처리해 자동 재시도 루프로 복귀하도록 보강.
- 기대 효과: auto-improve 단계에서 "진행 방법 선택"류 응답으로 흐름이 정지/왜곡되는 상황 감소.

## 2026-02-28 - 작업한일
- `.project/feature/*`의 draft 파일 기준을 `task.yaml`에서 `draft.yaml`로 통일.
- `src/draft.rs`에서 draft 생성/추가 시 이중 파일 쓰기 제거(`draft.yaml` 단일 저장).
- `src/main.rs`, `src/project.rs`의 탐색/검증 메시지에서 `task.yaml` 참조 제거 및 `draft.yaml` 기준 정렬.
- `src/parallel/mod.rs`에 완료 feature 이동 후처리 연결:
  - 병렬 빌드 성공 목록을 `.project/feature/<name>` -> `.project/clear/<name>`로 실제 이동.
  - 상태 승격(`planned -> features`) 이후 이동 결과 메시지 포함.
- `src/main.rs`에 `action_move_finished_features_to_clear` 추가.
- 검증: `cargo test -q` 통과(19 passed).

## 2026-03-01 - 작업한일
- 완료 이동 로직 도입 후 검증 실패하던 카운트 기준을 수정.
- `src/project.rs`에서 draft file count를 `.project/feature`뿐 아니라 `.project/clear`까지 합산하도록 보강.
- draft stage fallback 상태 출력에도 `.project/clear` 존재 여부 및 합산 카운트를 반영.
- 결과: `build-parallel-code` 성공 후 feature 이동(`.project/clear`) 시 verification 오탐(false fail) 제거.

## 2026-03-02 - 작업한일
- `build-parallel-code` 완료 후 상태 동기화를 보강해, 완료 항목이 `drafts_list.yaml`뿐 아니라 `.project/project.md`의 `## plan`에서 제거되고 `## features`로 승격되도록 `src/main.rs`에 `action_promote_project_md_plan_to_features`를 추가함.
- 위 동작이 실제로 유지되는지 검증하기 위해 `src/main.rs` 테스트 `promote_project_md_plan_to_features_moves_completed_items`를 추가함.
- UI add-plan 적용 경로에서 실제 CLI `add-plan` 호출이 누락되어 있던 부분을 `src/ui/mod.rs`에 `action_run_add_plan_via_cli`로 연결하고, 기존 planned 직접 추가 로직은 fallback으로 유지함.
- 구현 완료 프로세스에 `.project/feedback.md` 자동 생성 단계를 추가함:
  - `src/main.rs`에 feedback prompt 경로 해석/출력 검증/파일 쓰기 함수(`action_write_parallel_feedback`)를 추가.
  - `src/parallel/mod.rs` `run_parallel_build_code` 종료 시 feedback 생성을 실행하도록 연결.
  - 프롬프트 파일 `assets/prompts/parallel-feedback.txt`를 추가해 출력 스키마(`# 구현 완료 피드백`, `## 해결된 문제`, `## 개선점`, `## 다음 점검`)를 강제함.
- 검증: `cargo test` 통과(20 passed), `cargo run --bin orc -- --help` 실행 확인.

## 2026-03-02 - 작업한일
- UI에서 `focus` 또는 `active` 상태 색상을 주황색으로 통일함.
- `src/ui/mod.rs`에 `orange` 컬러 파싱(`Color::Rgb(255, 165, 0)`)을 추가하고, 활성/포커스 입력 테두리에서 기존 초록/노랑 값을 주황색으로 변경함.
- `configs/style.yaml`, `assets/style/pane_style.yaml`의 `active.border`를 `orange`로 변경해 패널 활성 테두리도 주황색으로 적용되도록 맞춤.

## 2026-03-03 - 작업한일
- 스킬 규칙 단일화를 위해 `/home/tree/ai/skills/rule-naming/SKILL.md`를 신설하고, 기존 `feature_architecture_rules`, `rule-name-prompt` 스킬 디렉터리를 제거함.
- 삭제된 스킬 경로를 참조하던 스킬 문서(`add-function`, `build-code-parallel`, `plan-drafts`, `plan-project-code`)를 `rule-naming` 경로로 교체함.
- `rust-orc` 코드에서 FEATURE_NAME 규칙 로딩 경로를 `/home/tree/ai/skills/rule-naming/SKILL.md`로 변경(`src/main.rs`).
- draft 생성 프롬프트의 스킬 참조를 `$feature-name-prompt-rules`에서 `$rule-naming`으로 변경(`src/draft.rs`, `src/main.rs`).
- 검증: `skill-creator quick_validate` 통과, `cargo test` 통과(20 passed).

## 2026-03-03 - 작업한일
- `build-function-auto`/`build-todo-auto`/`build-functon-auto` CLI를 추가해 `input.md` 자동 감지 -> `.project/todo.yaml(.project/todos.yaml 동시 기록)` 생성 -> `build-parallel-todo` 실행 -> `.project/feedback.md` 작성 -> feedback 재평가 루프를 한 명령으로 수행하도록 구현함.
- `src/main.rs`에 `TodoDoc/TodoItem` 스키마, `input.md` 파서 연동 draft 생성, todo preflight/수집, feedback markdown 검증(`# 구현 기능`, `## 문제 해결`, `## 미해결`, `## 개선점`) 및 다음 사이클 `input.md` 재생성/삭제 로직을 추가함.
- `src/parallel/mod.rs`에 `run_parallel_build_code`를 기반으로 한 `run_parallel_todo`를 추가해 `todo.yaml` 기반 task 병렬 실행 경로를 구현함.
- `src/cli/mod.rs`/`README.md`를 갱신해 신규 명령(`build-parallel-todo`, `feedback`, `build-function-auto`)과 alias를 help/문서에 동기화함.
- 검증: `cargo test` 통과(20 passed), `cargo run --bin orc -- --help`로 CLI 출력 동기화 확인.

## 2026-03-03 - 작업한일
- `show_current_state` 공용 함수를 `src/main.rs`에 추가하고, 출력 형식을 `[state]description`으로 고정함.
- `build-function-auto` 사이클에 상태 로그를 연결해 단계별로 `[plan]`(todo 생성), `[build]`(병렬 구현), `[chcek]`(feedback 작성/검토) 진행 상태를 한 줄 출력하도록 변경함.
- `src/parallel/mod.rs`의 `run_parallel_todo`에 병렬 작업 상태 로그를 추가해 preflight/check/빌드 라운드 및 task 시작 시 `[state]description` 형태로 현재 상태를 출력하도록 변경함.
- 검증: `cargo test` 통과(20 passed).

## 2026-03-03 - 작업한일
- `input.md`를 YAML 단일 문서 형태에서 Markdown 설계 문서 형태로 되돌리고, 채팅방 저장 파일 포맷만 YAML 스키마/예시로 명시하도록 수정함.
- 채팅 저장 경로를 `.temp`로 통일하고, ID를 `session_id`/`sender_id`/`message_id`로 분리해 역할을 명확화함.
- 파일 미존재 시 동작을 자동 생성이 아닌 `에러 출력 후 종료`로 통일함(chat/send 공통).
- chat 읽기 루프 기준에 `last_read_message_id` 추적과 `MAX_read_time` 폴링 조건을 명시함.

## 2026-03-03 - 작업한일
- 사용자 지정 입력 규칙(`#`, `-`, `>`)에 맞춰 `input.md` 형식을 재정렬함.
- 문서 포맷만 수정하고 기존 합의 내용(`.temp` 통일, ID 분리, 에러 후 종료, chat room YAML 포맷)은 유지함.

## 2026-03-03 - 작업한일
- `input.md`를 사용자 지정 문서 패턴(`#`, `-`, `>`)만 사용하도록 재수정하고 코드블록(````yaml`)을 제거함.
- 기존 합의 내용(`.temp` 경로, ID 분리, 파일 미존재 시 에러 종료, YAML 채팅방 형식)은 유지함.

## 2026-03-03 - 작업한일
- `orc` 단독 실행 시 도움말 목록이 최신 명령/alias를 반영하도록 `src/cli/mod.rs` `print_usage`를 갱신함.
- `build-function-auto` alias(`build-todo-auto`, `build-functon-auto`)와 `list/select/delete`, `draft-*`, `add-func`, `ui`, `tsend` alias를 도움말에 함께 표시하도록 추가함.
- 인자 없이 `orc`를 실행하면 `missing command` 오류 대신 도움말만 출력되도록 `src/main.rs` 진입 분기를 보강함.
- 검증: `cargo test` 통과(20 passed), `cargo run --bin orc --` 출력 확인.

## 2026-03-03 - 작업한일
- CLI 도움말 출력/명령 라우팅 관리 파일을 `src/cli/mod.rs`에서 `src/cli.rs`로 이동해 단일 파일(`cli.rs`)에서 관리하도록 정리함.
- `mod cli;` 로딩 구조는 유지하고, `src/cli` 디렉터리는 제거함.
- 검증: `cargo test` 통과(20 passed), `cargo run --bin orc --` 도움말 출력 정상 확인.

## 2026-03-03 - 작업한일
- 사용자 요청 해석 오류 재발 방지를 위해 `AGENTS.md`에 `CLI Execute-First Interpretation Rule`을 추가함.
- `호출해서 실행/실행해봐/돌려봐` 표현은 구현이 아니라 기존 CLI 실행 우선으로 처리하도록 명시함.

## 2026-03-03 - 작업한일
- 사용자 요청에 따라 `AGENTS.override.md`에도 CLI 실행 우선 해석 규칙을 추가함.
- 반영 위치: `/home/tree/ai/codex/AGENTS.override.md`, `/home/tree/home/AGENTS.override.md`.

## 2026-03-03 - 작업한일
- `/home/tree/home/AGENTS.override.md` 삭제 전 고유 규칙(`Port Ownership Override`)을 `/home/tree/ai/codex/AGENTS.override.md`로 이관함.
- 이관 완료 후 `/home/tree/home/AGENTS.override.md` 파일을 삭제함.

## 2026-03-03 - 작업한일
- `build-function-auto`의 `input.md -> todo.yaml` 생성 경로에서 하드코딩 파싱을 제거하고, LLM이 `assets/prompts/build-funciton-todo.txt` 프롬프트로 todo YAML을 직접 생성하도록 변경함(`src/main.rs`).
- 생성된 todo 항목 기준으로 draft YAML을 후속 생성하고 `draft_path/depends_on`을 채우는 흐름으로 정리함.
- `assets/prompts/build-funciton.txt`를 build-function draft 생성 프롬프트로 고정하고, `task.step` 문자열 배열 강제/맵 금지 규칙을 포함함.
- `assets/prompts/build-funciton-todo.txt`를 추가해 todo 스키마 생성 규칙을 분리했고, `input.md` 형식 해석은 `build-funciton.txt` 입력 객체 규칙을 참조하도록 갱신함.
- 검증: `cargo test` 통과(20 passed).

## 2026-03-03 - 작업한일
- `input.md`를 `#` 블록 단위로 파싱해(`다음 # 전까지`) 객체별 `{name}`, `{rule}`, `{step}`를 프롬프트 템플릿 변수로 주입하도록 `action_build_todo_from_input_md`를 변경함.
- `assets/prompts/build-funciton-todo.txt`를 객체 단위 생성 프롬프트로 전환하고, 출력을 `assets/code/templates/task.yaml` 형식의 단일 YAML item으로 강제함.
- 프롬프트에 스킬 참조를 `$rule-naming`, `$build_domain`, `$build-architecture`로 추가함.
- `assets/code/templates/task.yaml` 템플릿 파일을 추가함.
- 검증: `cargo test` 통과(20 passed).

## 2026-03-03 - 작업한일
- `# 제목` 값이 `{name}`으로 전달되고 `TodoItem.name`에 강제 매핑되도록 `build-function-auto` 경로를 보강함.
- todo item 파싱을 `GeneratedTodoItem`(옵셔널 필드)로 수신 후 `TodoItem`으로 매핑하도록 변경해, LLM 출력에 `name` 누락이 있어도 `# 제목` 기반 snake_case 이름으로 보정되게 수정함.
- `assets/prompts/build-funciton-todo.txt`에 `{name}`은 input.md `# 제목`이며 todo item `name`에 매핑해야 한다는 규칙을 추가함.
- 검증: `cargo test` 통과, `cargo run --bin orc -- build-function-auto` 실행 시 `missing field name` 오류는 해소됨(후속 draft step 타입 오류는 별도).

## 2026-03-03 - 작업한일
- `build-function-auto` 실행 중 LLM이 생성한 `draft/task.step` map 타입으로 파싱 실패하던 문제를 막기 위해 `action_normalize_draft_task_step_yaml`를 추가하고, draft 파싱 전 정규화 경로를 `build-function-auto`/`add-func`에 적용함.
- LLM이 생성한 todo item의 `rule/step/depends_on`에 map/scalar가 들어와도 문자열 배열로 보정하도록 `action_normalize_todo_item_yaml`를 추가하고 파싱 전 적용함.
- 검증: `cargo test` 통과(20 passed), `cargo run --bin orc -- build-function-auto` 실행 시 기존 step/rule 타입 파싱 실패는 해소되고 후속 규칙 검증 단계로 진행됨을 확인함.

## 2026-03-03 - 작업한일
- `.project/project.md`가 없을 때 템플릿 고정 생성 대신, 현재 폴더 파일 목록을 기반으로 project.md를 생성하는 함수(`action_generate_project_md_from_workspace`)를 추가함.
- `build-function-auto` 실행 중 `project.md` 누락 시 위 함수를 자동 호출하도록 `action_ensure_project_md_exists`를 변경함.
- 워크스페이스 파일 힌트 수집(`action_collect_workspace_file_hints`)과 spec 추론(`action_infer_workspace_spec`)을 추가해 project.md 생성 입력에 반영함.
- 검증: `cargo test` 통과(20 passed), `cargo run --bin orc -- build-function-auto` 실행에서 `project.md` 누락 오류는 재발하지 않음을 확인.

## 2026-03-03 - 작업한일
- todo 산출 파일 경로를 `.project/todos.yaml` 단일 경로로 통일하고, `build-function-auto`/`run_parallel_todo`의 읽기/쓰기 경로를 전부 `todos.yaml` 기준으로 갱신함.
- 프롬프트 파일을 `assets/code/prompts`로 일원화: 기존 `assets/prompts/*`와 `assets/code/prompt/*` 파일을 모두 이동하고 원본 디렉터리를 정리함.
- 프롬프트 경로 해석 코드를 전부 새 경로 기준으로 갱신함.
  - `src/main.rs`: build-function, build-function-todo, detail-project, project-md-init/auto, tasks, parallel-feedback resolver
  - `src/ui/mod.rs`: bootstrap prompt resolver
- 검증: `cargo test` 통과(20 passed).

## 2026-03-03 - 작업한일
- draft 관련 구조체(`DraftTask`, `DraftFeatures`, `DraftDoc`, `DraftsListDoc`, `PlannedItem`, `DraftStateDoc`)를 `src/draft.rs`로 이동하고, `src/main.rs`는 `pub(crate) use draft::{...}`로 재노출하도록 정리함.
- chat 관련 함수 분리를 위해 `src/chat.rs`를 신설하고 LLM 실행/채팅 로그 함수(`action_run_codex_exec_capture*`, `action_run_llm_exec_capture`)를 이관함.
- `src/main.rs`의 기존 chat/LLM 실행 본문은 `chat` 모듈 위임 함수로 변경함.
- todo 파일 경로를 `.project/todos.yaml` 단일로 통일하고 관련 읽기/쓰기 경로를 갱신함(`src/main.rs`, `src/parallel/mod.rs`).
- 프롬프트 파일을 `assets/code/prompts`로 일원화: `assets/prompts/*`, `assets/code/prompt/*`를 모두 이동하고 참조 resolver를 새 경로 기준으로 갱신함(`src/main.rs`, `src/ui/mod.rs`).
- 검증: `cargo test` 통과(20 passed).

## 2026-03-03 - 작업한일
- Fish 설정(`/home/tree/Config/data/fish/config.fish`)에 `obfa` alias를 추가해 `orc build-function-auto`를 단축 호출할 수 있게 설정함.
- 검증: `fish -c 'source /home/tree/Config/data/fish/config.fish; type obfa'`로 alias 함수 인식 확인.

## 2026-03-03 - 작업한일
- `build-function-auto`의 todo 생성을 input 객체 1:1 루프에서 input.md 전체 해석 기반 단일 LLM 생성 방식으로 변경함.
- `assets/code/prompts/build-funciton-todo.txt`를 수정해 task 개수를 LLM이 분해/통합 판단하도록 하고, `name` 생성 시 `$rule-naming` 기준 snake_case를 강제하도록 명시함.
- todo 생성 파서를 보강해 `tasks:` 문서/시퀀스/단일 객체 출력과 코드블록 섞인 출력까지 정규화 후 검증하도록 변경함.
- draft 생성 파서를 보강해 파싱 실패 시 YAML 복구 프롬프트를 1회 수행한 뒤 재파싱하도록 추가함.
- 검증: `cargo test` 통과(20 passed), `cargo run --bin orc -- build-function-auto`는 LLM 응답 지연으로 180초 timeout 발생(실행 경로 자체는 시작 확인).

## 2026-03-03 - 작업한일
- `AGENTS.md`에 기능 추가/개선 시 `orc` 도움말 목록을 같은 변경에서 갱신하도록 `CLI Help Update Rule`을 추가함.
- `src/cli.rs`의 `print_usage`를 명령 목록 배열 정렬 방식으로 변경해 `orc` 도움말이 알파벳순으로 출력되도록 수정함.

## 2026-03-03 - 작업한일
- `orc chat -n <name>`(수신 모드) 및 `orc chat -n <name> -m <message> [-i <receiver_id>] [--data <data>]`(전송 모드) CLI 명령을 추가하고 `src/cli.rs` help/파서를 동기화함.
- `src/main.rs`에 `.temp/<name>.yaml` 기반 chat room YAML 로드/저장, 8자리 session/sender/message ID 생성, 메시지 append, 수신 폴링(`max_read_time`) 로직을 추가함.
- `src/config/mod.rs`, `config.yaml`, `configs.yaml`에 `max_read_time` 설정을 추가하고 chat 수신 대기 시간 기본값(3초)으로 연동함.
- `README.md` 명령 목록에 `orc chat` 사용 예시를 추가함.

## 2026-03-03 - 작업한일
- `orc chat -n <name>` 기본 동작을 변경해 `.temp/<name>.yaml`이 없거나 파일 내용이 비어 있으면 기본 chat room YAML을 자동 생성한 뒤 계속 실행하도록 수정함.
- `src/main.rs`의 `action_save_chat_room`에 부모 디렉터리 생성 로직을 추가해 `.temp/`가 없어도 자동 생성 경로가 실패하지 않도록 보강함.
- `README.md` Notes에 chat room 자동 생성 동작을 명시함.

## 2026-03-03 - 작업한일
- `orc chat -n <name> --background` 옵션을 추가해 chat watcher를 백그라운드 프로세스로 실행하고 즉시 터미널을 반환하도록 구현함.
- watcher 내부 실행 플래그(`--watch`)를 추가해 백그라운드 모드에서 stdin 없이 `.temp/<name>.yaml` 변화만 주기적으로 감지/출력하도록 분리함.
- 백그라운드 watcher 출력 로그를 `.temp/<name>.watch.log`에 append하도록 추가하고, `src/cli.rs` help/`README.md` 명령 목록 및 Notes를 동기화함.

## 2026-03-03 - 작업한일
- chat 발신자 ID를 세션 기반으로 고정하기 위해 `.temp/<room>.sessions.yaml` 저장소를 추가하고, 세션 키(`TMUX_PANE` 우선, fallback: TTY/PID)별 `sender_id`를 재사용하도록 변경함.
- `orc chat -n <name> -m ...`와 수신 모드 모두에서 무작위 `sender_id` 재생성 대신 세션 저장소 조회/갱신 경로를 사용하도록 수정함.
- `README.md` Notes에 세션 기반 `sender_id` 재사용 동작을 문서화함.

## 2026-03-03 - 작업한일
- 세션 키 충돌을 줄이기 위해 chat 세션 식별자를 `TMUX_PANE + PPID + TTY` 조합(또는 `ORC_CHAT_SESSION_KEY` 강제 지정)으로 강화함.
- 동일 경로/동일 room이라도 다른 pane/쉘에서 실행하면 서로 다른 `sender_id`를 갖도록 세션 키 계산 로직을 보강함.

## 2026-03-03 - 작업한일
- 사용자 요구에 맞춰 chat 세션 키를 pane 단위가 아닌 tmux window 단위(`session_id + window_id`)로 변경함.
- 같은 tmux window 내 여러 pane에서 `orc chat`을 호출해도 동일 `sender_id`가 재사용되도록 `calc_chat_session_key`의 tmux 키 계산 로직을 `tmux display-message` 기반으로 수정함.

## 2026-03-03 - 작업한일
- 요구사항 정정에 따라 chat 세션 키를 다시 tmux pane 단위(`TMUX_PANE`)로 조정함.
- 같은 window라도 pane이 다르면 `sender_id`가 서로 독립적으로 관리되도록 `calc_chat_session_key` 로직과 README 설명을 동기화함.

## 2026-03-03 - 작업한일
- `orc chat-wait -n <name> -a <true|false>` 명령을 추가해 채팅 파일 변화 대기 모드를 구현함.
- `chat-wait` 내부에서 `reaction()` 함수를 호출하도록 연결하고, 기본 동작으로 새 메시지 도착 시 메시지를 출력하도록 구성함.
- `-a true`는 모든 메시지 반응, `-a false`는 receiver가 자기 `sender_id`인 메시지에만 반응하도록 필터를 추가함.
- `src/cli.rs` help와 `README.md` 명령/설명을 동기화함.

## 2026-03-03 - 작업한일
- `chat-wait`에 `-c <count>` 옵션을 추가해 지정 개수만큼 반응 메시지를 수신하면 자동 종료되도록 확장함.
- `orc run_parallel_test` 명령을 추가해 `test` room 준비 -> 10개 백그라운드 프로세스 실행(3초 대기 후 `orc chat` 완료 메시지 전송) -> `chat-wait -a false -c 10` 대기 흐름을 구현함.
- 병렬 테스트용 프롬프트 파일 `assets/code/prompts/parallel_order.txt`, `assets/code/prompts/parallel_oredr_unit.txt`를 추가하고 실행 경로에서 로드하도록 연결함.
- `src/cli.rs` help와 `README.md` 명령/설명을 동기화함.
- 동시 메시지 전송 시 YAML read-modify-write 충돌로 누락이 발생하지 않도록 `orc chat` 전송 경로에 room 단위 파일 락(`.temp/<room>.lock`)을 추가해 `run_parallel_test` 10/10 수신 완료를 보장함.

## 2026-03-03 - 작업한일
- `AGENTS.override.md`를 신설하고, 사용자가 `current.png` 확인을 요청하면 기본 경로를 `/mnt/c/Users/tende/Pictures/Screenshots/current.png`로 해석하도록 영구 규칙을 추가함.
- 기본 경로 파일이 없을 때는 `/mnt/c/Users/tende/Pictures/Screenshots/` 디렉터리에서 검색하도록 보조 규칙을 추가함.

## 2026-03-04 - 작업한일
- `init_code_project`를 명세 중심으로 단순화해 빈 폴더면 템플릿(`assets/code/templates/project.md`) 기반으로 `./.project/project.md`를 생성하고 `# info(name/description/path/spec)`를 채우도록 수정함.
- 현재 폴더가 비어있지 않으면 `load_code_project()`를 호출해 동일 템플릿 기반으로 `project.md`를 생성하도록 정리함.
- `-a -m` 자동 모드에서 메시지 기반 `name/description/spec` 추론 후 `detail_code_project` -> `create_code_domain` -> `init_code_plan -a` 순서로 동작하도록 고정함.
- 레거시 `check_list.md` 자동 생성 경로(`write_current_requirement_checklist`)를 제거해 프로젝트 초기화 시 불필요 파일이 생성되지 않도록 정리함.

## 2026-03-04 - 작업한일
- `init_code_project` 옵션 파서를 수정해 `-a "msg"` 형태를 지원하고, 자동 모드 메시지를 `-a <msg>` 또는 `-m <msg>`로 입력할 수 있도록 정리함.
- CLI usage에 `init_code_project ... [-a [message]]` 형식을 반영함.
- `~/temp/verify-a-msg`에서 `orc init_code_project -a "zustand react todo app"` 실행 검증을 완료함(`project.md #info`, `plan.yaml` 생성 확인).

## 2026-03-04 - 작업한일
- `project.md` 관련 프롬프트(`init_code_project`, `init_code_project_auto`, `load_code_project`, `add_detail_project_code`)에 `# domains` 하위 `states/action/rules/constraints`가 모두 `-` 리스트 형식임을 명시함.

## 2026-03-04 - 작업한일
- `project.md`의 `# domains` 형식을 `## <domain_name> -> ### states/action/rules` 구조로 템플릿에 반영함.
- 도메인 추출/서브섹션 추출 파서(`calc_extract_project_md_domain_names`, `extract_domains_from_project_md`, `extract_domain_subsection_items`)를 새 형식 우선으로 수정하고, 기존 `- **name**:` 기반은 레거시 fallback으로만 유지함.
- `create_code_domain`이 새 도메인 블록(`## app`, `### states/action/rules`)을 생성하도록 수정함.
- main/ui의 `project.md` 형식 검증 로직에서 도메인 규칙을 새 형식(`## <name> + ### states/action/rules`)으로 교체함.
- 도메인 파싱 단위 테스트 2건 추가 및 기존 도메인 테스트를 새 형식으로 갱신함.
- CLI 실검증(`~/temp/domain-format-check`)으로 `init_code_plan`이 새 `#domains`에서 도메인 2개를 정상 반영하는 것을 확인함.

## 2026-03-04 - 작업한일
- `project.md/plan.yaml/drafts.yaml/draft_item.yaml` 생성 경로에서 하드코딩 문자열 출력을 줄이고 `assets/code/templates` 파일을 읽어 초기 문서를 로드/매핑하도록 변경함.
- `load_plan_doc()`는 `plan.yaml` 미존재 시 템플릿(`assets/code/templates/plan.yaml`)을 파싱해 초기값으로 사용하도록 수정함.
- `load_drafts_doc()`는 `drafts.yaml` 미존재 시 템플릿(`assets/code/templates/drafts.yaml`)을 파싱해 초기값으로 사용하도록 수정함.
- `write_feature_draft_file()`는 하드코딩 YAML 대신 `assets/code/templates/draft_item.yaml`을 읽어 `draft_item.yaml`을 생성/매핑하도록 변경함.
- `save_drafts_doc()`는 수동 매핑 하드코딩을 제거하고 `CodeDraftsDoc` 직렬화 결과를 저장하도록 단순화함.

## 2026-03-04 - 작업한일
- `init_code_plan`을 1회 초기화 전용으로 변경해 `.project/plan.yaml`이 이미 존재하면 에러를 반환하도록 수정함.
- 워크플로우를 `초기 1회 init_code_plan -> 이후 add_code_plan`으로 고정하고 `~/temp/init-once-flow`에서 1차 init 성공/2차 init 실패/add_code_plan 성공을 검증함.

## 2026-03-04 - 작업한일
- `add_code_draft`에서 레거시 `.project/feature/*/draft_item.yaml` 생성 경로를 제거함(관련 함수 및 호출 삭제).
- `add_code_draft`는 `plan.yaml planned`를 기반으로 `.project/drafts.yaml`만 생성/갱신하도록 정리함.
- `add_code_draft` 완료 시 레거시 산출물 정리를 위해 `.project/feature` 디렉터리 cleanup 경로를 추가함.
- `~/temp/check-add-code-draft` 시나리오로 `-f` 입력에서 `drafts.yaml` 항목 생성(rule/step/type/domain/constraints 반영) 동작을 재확인함.

## 2026-03-04 - 작업한일
- `add_code_draft`의 레거시 파일 출력(`.project/feature/*/draft_item.yaml`) 경로를 제거하고 `drafts.yaml` 중심 갱신으로 고정함.
- `check_code_draft` 전/후에 `.project/feature` 정리 가드를 추가하고, check-code follow-up 프롬프트에서 `.project/feature` 생성 금지 및 `.project/drafts.yaml` 기준 점검을 명시함.
- `$check-code` 스킬 규칙에 맞춰 실행 경로(`add_code_draft -> drafts.yaml 반영 -> check_code_draft`)와 scenario(`.project/scenario.md`) 형식을 검증함.

## 2026-03-04 - 작업한일
- CLI `test` 명령을 추가해 현재 작업 디렉터리에서 `cargo test -q`를 실행하도록 연결함(`Cargo.toml`이 없으면 skip 처리).
- `check_code_draft` 내부에서 점검 후 `test` 실행 결과를 `report.md`에 함께 기록하도록 연동함.
- `check_code_draft` 실행 시 debug 모드면 tmux 분할 pane(`check-code-debug`)를 열어 `.project/runtime/check-code.log` tail을 볼 수 있도록 추가함(실패 시 무시).

## 2026-03-04 - 작업한일
- `orc test` CLI 라우팅을 수정해 `check_code_draft(false)`를 직접 호출하도록 변경함.

## 2026-03-04 - 작업한일
- `src/cli.rs`에서 alias 라우팅(`list/select/delete/ui/tsend/draft-add 계열 등`)을 제거하고 정식 명령만 허용하도록 정리함.
- usage 출력의 alias 안내 문구를 제거해 CLI 도움말을 실제 지원 명령과 일치시킴.

## 2026-03-04 - 작업한일
- 레거시 호환 CLI 명령 `create-draft`, `add-plan`, `add-draft`, `delete-draft`를 `src/cli.rs`에서 제거함.
- `create-project <name> [path] [description]` 포지셔널 호환 파싱을 제거하고 `-n/-p/-d/-s` 기반만 유지함.

## 2026-03-04 - 작업한일
- 레거시 호환 명령/문구를 전수 제거하고 표준 명령(`create_code_draft`, `add_code_draft`, `add_code_plan`)만 사용하도록 CLI/UI/README/layout을 정리함.
- `code.rs`에서 레거시 drafts 파서(`planned/worked/complete` 병합)와 `constratins` alias 파싱을 제거해 새 drafts 포맷만 허용하도록 고정함.
- `main.rs`의 레거시 registry fallback(`configs/Project.yaml`)과 레거시 미러 함수명을 제거하고 단일 경로 저장 함수(`action_save_drafts_list_primary`)로 정리함.

## 2026-03-04 - 작업한일
- `src/project.rs`의 auto execution plan 문자열에서 레거시 명령/경로(`create-draft`, `drafts_list`, `.project/feature`)를 제거하고 새 명령/경로(`create_code_draft`, `plan.yaml`, `drafts.yaml`)로 교체함.
- `feedback.md`의 레거시 명령 문자열을 `create_code_draft`로 정리함.
- `src/assets/README/feedback` 범위에서 레거시 키워드 재검색 결과 0건 확인.

## 2026-03-04 - 작업한일
- todo 레거시 경로를 제거함: CLI `build-parallel-todo`, `build-function-auto` 명령 삭제, `parallel::run_parallel_todo` 삭제, `main.rs`의 todo 전용 타입/상수/함수 일괄 삭제.
- todo 관련 프롬프트/문구 정리: `assets/code/prompts/build-funciton-todo.txt` 삭제, `build-funciton.txt`의 todo 표현 제거, README 명령 목록에서 todo 관련 항목 제거.
- `src/README/assets` 범위에서 todo 키워드 재검색 결과 0건 확인.

## 2026-03-04 - 작업한일
- `src/code.rs`의 code 플로우 경로를 `./.project`로 고정하고 `.procjet` 분기 로직을 제거함.
- `add_code_draft` 검증 경로를 정리해 `./.project/drafts.yaml` 중심으로만 동작하도록 보정하고, `feature/*/draft.yaml` 생성 로직을 제거함.
- `impl_code_draft`를 `./.project/plan.yaml`의 `planned -> worked -> complete` 상태 전이를 기준으로 실행하도록 정리하고, 실패 시 `feedback.md` 기록 후 즉시 오류 반환하도록 수정함.
- `assets/code/prompts/add_code_draft_by_file.txt`, `add_code_draft_by_message.txt`, `impl_code_draft.txt`의 경로 문구를 `./.project/plan.yaml`, `./.project/drafts.yaml` 기준으로 통일함.
- 검증: `cargo test -q` 통과(22 passed).

## 2026-03-04 - 작업한일
- `detail_code_project`에서 템플릿 placeholder 항목(`프로젝트 내부의 공통 규칙`, `프로젝트 내부의 공통 제약`)이 최종 `project.md`에 남지 않도록 제거 로직을 추가함.
- `create_code_domain`은 템플릿 형식은 유지한 채, 출력 시 `# domains` 섹션을 값 기반 도메인 블록으로 교체하도록 유지함.
- `init_code_project` 실행 검증 결과 `# rules/#constraints`에 placeholder가 제거된 상태로 생성됨을 확인함.
- 검증: `cargo test -q` 통과(22 passed) + `/home/tree/temp2`에서 `init_code_project` 실실행 확인.

## 2026-03-05 - 작업한일
- `plan.yaml` 상태 전이를 위해 `change_state_plan()` 함수를 추가하고, `impl_code_draft`에서 항목 이동을 수동 리스트 조작 대신 상태 전이 함수로 처리하도록 변경함.
- 병렬 구현 시 성공한 항목만 즉시 `drafts.worked -> drafts.complete`로 이동하도록 로직을 수정함(부분 실패 시 성공 항목은 complete 유지).
- `sync_plan_doc`에 중복 제거/우선순위 정규화(`complete > worked > planned`)를 추가해 상태 중복을 방지함.
- `CodePlanDoc`의 루트 `planned/worked/complete` 필드는 역호환 입력만 허용하고 저장 시 직렬화되지 않게 변경해, 출력 YAML은 `drafts` 내부 상태를 단일 기준으로 유지함.
- 검증: `cargo test -q` 통과(23 passed).

## 2026-03-05 - 작업한일
- `init_code_project`가 비어있지 않은 워크스페이스에서 `load_code_project()`를 타더라도 최종 `#info`(`name/description/spec/path`)는 사용자 입력/auto message 추론값으로 다시 덮어쓰도록 보정함.
- `infer_workspace_spec()`를 보강해 `package.json` 기반 추론 시 `react`, `zustand` 의존성이 있으면 `spec`에 함께 반영하도록 수정함(기존 `next js` 단일 고정 개선).
- `bootstrap_code_project()`에 post-verify를 추가해 `spec`에 `zustand`가 포함된 경우 `package.json`에 의존성이 누락되면 자동 보정하도록 추가함.
- `build_input_md_auto()`에서 계산기 프로젝트(react+zustand+calculator/계산기)일 때 최소 기능 객체(`calculator_ui`, `calculator_engine`)를 보강해 `input.md`가 비어있거나 과소 생성되는 경우를 방지함.
- `add_code_draft()`를 spec-aware로 보강해 React 계열에서는 `scope/tasks/check`가 실제 구현 파일(`app/page.tsx`, `store/calc.ts`, `lib/calc.ts`) 중심으로 생성되도록 수정함.
- 프롬프트 강화: `assets/code/prompts/bootstrap.txt`에 spec 의존성 반영(`zustand` 필수)과 계산기 최소 구조 생성 규칙을 추가하고, `assets/code/prompts/impl_code_draft.txt`에 scope 파일 실제 수정 및 React+Zustand 계산기 흐름 연결 규칙을 추가함.
- 검증: 테스트는 사용자 요청으로 미실행, `cargo check` 통과.

## 2026-03-05 - 작업한일
- debug 모드(`AppConfig.debug_enabled=true`)에서 `orc auto` 실행 시 단계 로그를 강화함.
- `init_code_plan`에서 `plan.yaml` 저장 직후 `[auto:plan-yaml]` 로그를 추가해 생성 시점이 즉시 출력/기록되도록 수정함.
- `add_code_draft`에서 `drafts.yaml` 저장 직후 `[auto:draft-yaml]` 생성 로그를 남기고, check 이후 별도 완료 로그를 남기도록 분리함.
- 기존 단계 로그와 합쳐 auto 경로에서 다음 시점 로그를 확인 가능: project 생성, bootstrap 시작/완료, plan.yaml 생성, drafts.yaml 생성, 병렬처리 시작.
- 검증: `cargo check` 통과.

## 2026-03-05 - 작업한일
- `spec` 추론 하드코딩 분기를 제거하고 LLM 기반 추론 함수(`infer_spec_with_llm`)를 추가함.
- `infer_from_message()`는 메시지를 LLM에 전달해 `spec: ...` 형식 응답을 파싱하도록 변경함.
- `infer_workspace_spec()`도 워크스페이스 힌트(`package.json`, deps, Cargo.toml 등)를 LLM에 전달해 spec을 우선 추론하도록 변경함(LLM 실패 시에만 기존 fallback 사용).
- `assets/code/prompts/infer_code_spec.txt` 프롬프트를 추가해 spec 추론 출력 형식을 `spec: <value>`로 고정함.
- 검증: `cargo check` 통과.

## 2026-03-05 - 작업한일
- 단계 전환을 새 세션(별도 프로세스)으로 실행하도록 `run_code_subcommand_in_new_session()` 공통 함수를 추가하고, `init_code_project(-a)->init_code_plan(-a)`, `init_code_plan(-a)->add_code_draft(-a)`, `add_code_draft(-a)->impl_code_draft`, `add_code_plan->add_code_draft` 경로를 모두 해당 함수로 전환함.
- 새 세션 실행 시 stdout/stderr를 부모 콘솔로 상속해 debug 로그가 실시간으로 보이도록 수정함.
- `action_debug_log_auto_stage()`가 `.project` 폴더를 선생성하지 않도록 변경해 auto 시작 로그가 작업 디렉터리 비어있음 판정에 부작용을 만들지 않게 수정함.
- `~/temp`에서 `orc auto "three fiber로 cube를 선택하면 회전하는 코드를 구현해줘"` 실행 시 bootstrap 진입/완료 로그가 실제로 출력되는 것을 확인함.

## 2026-03-05 - 작업한일
- `$check-code` 스킬 문서(`/home/tree/ai/skills/check-code/SKILL.md`)를 전면 단순화해 점검 범위를 3개 체크리스트(논리적 병목, 환형대기, 동시 파일 접근 충돌)로 축소함.
- 기존 PHASE 1~7 대규모 정적 품질 감사 규칙을 제거하고, 출력 포맷을 `CHECK_CODE_RESULT` 고정 형식으로 변경함.
- Draft follow-up 계약도 동일한 Fast Checklist 기준으로 통일해, 후속 점검에서 과도한 분석/수정 루프가 생기지 않도록 조정함.
- 검증: `cargo check` 통과.

## 2026-03-05 - 작업한일
- 사용자 지시를 반영해 `AGENTS.md`에 `No-Hardcoding Default Rule`을 추가함.
- 규칙 내용: 사용자가 명시하지 않으면 하드코딩 분기 구현 금지, `assets/code/prompts` 기반 LLM 추론 경로 우선.
- 검증: `cargo check` 통과.

## 2026-03-05 - 작업한일
- `/home/tree/ai/codex/AGENTS.override.md`에 `No-Hardcoding Default Rule`을 추가함.
- 규칙 내용: 사용자 명시 없는 도메인 하드코딩 금지, `assets/code/prompts` 기반 LLM 처리 우선.

## 2026-03-05 - 작업한일
- `project.md/plan.yaml/drafts.yaml` 경로에서 하드코딩 채우기 로직을 줄이고, `assets/code/prompts` 기반 LLM 생성 함수로 전환함.
- `detail_code_project`는 `add_detail_project_code.txt` 기반 LLM 결과로 `.project/project.md` 본문을 보강하도록 변경함.
- `create_code_domain`은 `create_domain.txt` 기반 LLM 결과로 도메인 블록을 생성하도록 변경함.
- `init_code_plan`은 `infer_plan_yaml.txt` 기반 LLM 결과로 plan 문서를 생성/갱신하도록 변경함.
- `add_code_draft`는 `infer_draft_item.txt`/`infer_draft_fields.txt` 기반 LLM 결과로 draft item 필드를 채우도록 변경함.
- 파일/폴더 생성 순서 강제: 함수가 먼저 `.project`/`project.md`/`plan.yaml`/`drafts.yaml` 템플릿 파일을 생성(`ensure_*_initialized`)하고, 이후 LLM이 내용만 수정하도록 순서를 고정함.
- 경로 강제: `./project/project.md`가 생성되면 자동으로 `./.project/project.md`로 정규화/정리(`enforce_project_md_primary_path`)하도록 추가함.

## 2026-03-05 - 작업한일
- `check_code_draft`의 reference 경로를 `./.project/reference`로 변경하고, debug tail 대상도 `./.project/reference/check-code.log`로 전환함.
- `main.rs`의 check-code runtime log 경로를 `./.project/runtime`에서 `./.project/reference`로 변경함.
- `assets/code/templates/report.md` 템플릿을 추가하고, `check_code_draft`가 템플릿 치환(`{{implementation_check}}`, `{{issues}}`)으로 `report.md`를 생성하도록 변경함.
- `report.md` 형식은 `# 구현 확인`, `# 발견된 문제` 2개 섹션만 사용하도록 고정함.
- `check-code` skill 문서에 Report Format Rule을 추가해 동일한 헤더 제한을 반영함.
- 검증: `cargo check` 통과.

## 2026-03-05 - 작업한일
- 구형 CLI 워크플로우 잔여 경로를 제거하기 위해 `src/main.rs`에서 `mod project;` 연결 및 레거시 래퍼(`project::*` 위임), `feedback/add_func/plan_project/detail_project/validate_tasks/run_parallel_test` 관련 함수를 삭제함.
- `src/project.rs` 파일을 삭제해 더 이상 구형 auto/check/improve/report/create/select/delete 프로젝트 경로가 코드베이스에 남지 않도록 정리함.
- UI 내부 레거시 명령 호출을 교체함: `create-project` -> `init_code_project`, `build-parallel-code` -> `impl_code_draft`.
- `UiRunResult.auto_mode_project` 필드를 제거하고, UI 도움말 문자열의 구형 커맨드명 노출을 정리함.
- 미사용 구형 프롬프트 파일을 삭제함: `assets/code/prompts/detail-project.txt`, `assets/code/prompts/parallel_order.txt`, `assets/code/prompts/parallel_oredr_unit.txt`.
- 검증: `cargo check` 통과(경고만 존재).

## 2026-03-05 - 작업한일
- bootstrap 경로를 `project.md`의 `spec` 단일 정보 기반으로만 동작하도록 변경함.
- `src/code.rs`에 `extract_project_spec_from_md(project_md)` 함수를 추가하고, `bootstrap_code_project()`가 이 함수로 spec을 추출하도록 교체함.
- `src/ui/mod.rs`의 bootstrap 프롬프트 생성에서 `project_md`, `info_block` 전달을 제거하고 `project_name/project_root/spec/preset`만 전달하도록 수정함.
- `assets/code/prompts/bootstrap.txt`를 수정해 입력/지시에서 `project.md` 및 `info_block` 의존 문구를 제거하고 spec 기준 실행 규칙으로 정렬함.
- 검증: `cargo check` 통과.

## 2026-03-05 - 작업한일
- bootstrap spec 입력 경로를 인자 기반에서 `.project/project.md` 하드코딩 파싱 기반으로 변경함.
- `src/ui/mod.rs`에 `calc_extract_spec_from_project_md()`와 `action_extract_bootstrap_spec_from_project_md()`를 추가함.
- `action_apply_bootstrap_by_spec()`는 외부 `spec` 인자를 받지 않고, 위 추출 함수 결과만 프롬프트 `{{spec}}`에 주입하도록 변경함.
- `action_run_bootstrap_llm_prepare()`도 동일하게 project.md에서 spec을 추출해 프롬프트에 포함하도록 변경함.
- 호출부 정리: `src/main.rs`, `src/code.rs`, `src/ui/mod.rs`에서 변경된 시그니처에 맞게 업데이트함.
- 검증: `cargo check` 통과.

## 2026-03-05 - 작업한일
- `src` 전역에서 함수명 `calc_*`, `action_*` 접두사 제거를 일괄 적용함(정의/호출 참조 동시 변경).
- 충돌 검사 후(중복 대상 0건) 자동 치환을 수행해 네이밍을 일반 snake_case로 통일함.
- 잔여 정의 1건(`calc_modal_cursor`)까지 수동 정리해 `fn (calc_|action_)` 패턴 0건으로 맞춤.
- 네이밍 스킬 문서(`/home/tree/ai/skills/rule-naming/SKILL.md`)에 `calc_`, `action_` 접두사 금지 규칙을 추가함.
- 검증: `cargo check` 통과.

## 2026-03-05 - 작업한일
- `orc auto` 실패 시 즉시 종료되던 흐름을 개선해 `src/code.rs`의 `auto_code_message`에 최대 3회 재시도 루프를 추가함.
- 각 실패 시 `feedback.md`에 실패 요약/상세를 누적 기록하도록 `write_feedback_md`를 append 방식으로 변경함.
- 실패 원인 기반 재시도 메시지를 LLM이 생성하도록 `infer_auto_retry_message`를 추가하고, 프롬프트 `assets/code/prompts/auto_retry_from_feedback.txt`를 신설함.
- 검증: `cargo check`, `cargo test` 모두 통과.

## 2026-03-05 - 작업한일
- `orc auto`가 `impl_code_draft` 단계에서 멈춰도 실패 처리/복구 루프가 동작하도록 `src/code.rs`에 타임아웃 제어를 추가함.
- `run_code_subcommand_in_new_session`를 `status()` 블로킹 방식에서 `spawn + try_wait` 루프로 변경하고, 기본 600초(`ORC_CODE_SUBCOMMAND_TIMEOUT_SEC` override 가능) 초과 시 프로세스를 종료하고 에러를 반환하도록 수정함.
- `impl_code_draft_parallel`의 각 LLM 작업을 `run_codex_exec_capture_with_timeout` 기반으로 변경하고 기본 240초(`ORC_IMPL_DRAFT_LLM_TIMEOUT_SEC` override 가능) 타임아웃을 적용함.
- 위 변경으로 타임아웃 발생 시 상위 auto 루프가 실패를 감지해 `feedback.md` 기록 및 재시도를 이어갈 수 있게 개선함.
- 검증: `cargo check`, `cargo test` 통과.

## 2026-03-05 - 작업한일
- `./.project/project.md`를 최소 범위 Astro 랜딩 요구사항에 맞춰 전체 상세화하고, `#info` 기반 규칙/제약 강화 및 `#domains`를 도메인별 `states/action/rules/constraints` 불릿 구조로 재정의함.

## 2026-03-05 - 작업한일
- `auto` 모드의 단계 간 자동 연쇄 진입을 비활성화함: `init_code_project -a`, `init_code_plan -a`, `add_code_plan -a`, `add_code_draft -a`가 다음 명령을 자동 호출하지 않고 현재 단계만 처리하도록 수정함.
- `auto` 실패 재시도 루프를 제거하고 단일 시도 실패 시 `feedback.md`만 기록하도록 `auto_code_message` 흐름을 단순화함.
- `assets/code/templates/drafts.yaml` 스키마를 확장해 `planned/worked/complete/failed` 상태를 추가함.
- `src/code.rs`의 drafts 문서 구조/동기화 로직을 확장해 `failed` 상태를 관리하고, `impl_code_draft` 실패 시 `drafts.yaml.failed`로 항목 이동되도록 상태 전이 로직을 구현함.
- `src/code.rs`의 세션 실행 경로를 tmux 워커 pane 위임 방식으로 확장함( tmux 환경일 때 ): 하위 작업을 split pane에서 실행하고 완료/실패 메시지를 부모 pane에 전달한 뒤 워커 pane을 자동 종료하도록 변경함.
- `src/tmux/mod.rs`에 pane 대상 완료 알림용 `display_message` 함수를 추가함.
- orc 단계형 사용법과 tmux 위임 운영 규칙을 담은 새 스킬 `/home/tree/ai/skills/orc-cli-workflow/SKILL.md`를 추가함.
- 검증: `cargo check`, `cargo test` 통과.

## 2026-03-05 - 작업한일
- tmux 워커 pane 분할 안정화를 위해 `src/tmux/mod.rs`에서 `split-window` 호출에 현재 pane 타깃(`-t <current pane>`)과 현재 경로(`-c #{pane_current_path}`)를 명시하도록 보강함.
- 워커 pane 무출력 체감 문제를 줄이기 위해 `src/code.rs`, `src/chat.rs`의 tmux 실행 스크립트 시작부에 실행 시작/작업 경로 로그(`echo`)를 추가함.
- tmux 래퍼 로깅 기반 재현으로 실제 호출 인자를 검증했고, `cargo build` 후 `split-window -h -t ...`로 좌우 분할 및 pane 출력 노출이 되는 경로를 확인함.
- 검증: `cargo test` 통과, `cargo build` 통과, tmux 실행 경로 확인 완료.

## 2026-03-05 - 작업한일
- 응답 금칙어 재발 방지를 위해 `/home/tree/project/rust-orc/AGENTS.md`의 `Response Phrase Rule`에 강제 치환 맵(`맞습니다` 계열 -> `확인했습니다`)과 발송 전 4단계 검증 순서(초안->스캔->치환->재스캔)를 추가함.
- 스킬/프롬프트 경로 규칙 변경 후에도 동일 실수가 반복되지 않도록 규칙을 설정 파일 수준에서 고정함.

## 2026-03-05 - 작업한일
- 응답 금칙어 규칙을 재강화함: `/home/tree/project/rust-orc/AGENTS.md`의 `Response Phrase Rule`에 `확인했습니다`를 금칙어/금지 시작 표현으로 추가함.
- 금칙어 치환 맵(금칙어 -> 확인했습니다)을 제거하고, 발송 전 단계 3을 "중립 결과 문장으로 재작성" 규칙으로 교체해 동일 유형 문구 재발을 차단함.

## 2026-03-05 - 작업한일
- `AGENTS.override.md`에 `Response Phrase Override Rule`을 추가해 금칙어(`맞습니다`, `확인했습니다` 등)와 발송 전 검증 절차를 override 파일에 직접 고정함.
- override 우선 적용 문구를 함께 추가해 동일 유형 응답 문구 재발 가능성을 줄임.

## 2026-03-05 - 작업한일
- `/home/tree/project/rust-orc/AGENTS.override.md`를 일반 파일에서 심볼릭 링크로 교체해 `/home/tree/ai/codex/AGENTS.override.md` 전역 규칙을 직접 참조하도록 수정함.
- 검증으로 `ls -l`, `readlink -f`에서 링크 대상이 전역 파일로 해석되는 것을 확인함.

## 2026-03-05 - 작업한일
- tmux pane 자동 생성 억제: `src/code.rs`, `src/chat.rs`에서 tmux pane 실행 조건을 기본 비활성화로 변경하고, `ORC_USE_TMUX_PANES=1`일 때만 worker/llm split pane을 사용하도록 수정함.
- `check_code_draft`의 `check-code-debug` tail pane은 `ORC_ENABLE_CHECK_DEBUG_PANE=1`일 때만 생성되도록 변경함.
- 기존에 남아 있던 `check-code.log tail`/`sleep` 잔여 tmux pane을 정리해 기본 세션만 남도록 cleanup함.
- 검증: `cargo test` 통과(23 passed).

## 2026-03-05 - 작업한일
- 사용자 요청에 따라 `regret` 스킬을 신설함: `/home/tree/ai/skills/regret/SKILL.md`.
- 스킬 참조 보고서 템플릿 `/home/tree/ai/skills/regret/references/report.md`를 생성하고 `# 잘못한점`, `# 개선할점` 섹션을 초기화함.
- 전역 규칙 파일 `/home/tree/ai/codex/AGENTS.override.md`에 `Regret Skill Trigger Rule`을 추가해 `잘못` 표현 감지 시 report.md 양 섹션에 항목을 추가하도록 고정함.

## 2026-03-05 - 작업한일
- spec별 재발 이슈 관리 기능을 추가함: `src/main.rs`에 프로젝트 `.project/project.md`의 `spec`을 추출해 `assets/checkPoints/<spec>.md` 경로를 계산/관리하는 유틸을 구현함.
- `check_code_draft` 점검 경로(`run_check_code_after_draft_changes`)에 spec checkpoint history 주입을 추가해, check-code가 과거 발생 이슈 재발 여부를 우선 점검하도록 프롬프트를 강화함.
- `src/code.rs`의 `check_code_draft` 완료 시 실제 이슈가 있으면 `append_spec_checkpoint_issues("check_code_draft", issues)`로 spec 체크포인트 파일에 누적 기록하도록 연결함.
- `checkPoints`를 기준 경로로 사용하고, 점검 시에는 `checkpoints`(legacy) 파일이 존재하면 함께 읽도록 호환 처리함.
- 검증: `cargo test` 통과(23 passed).

## 2026-03-05 - 작업한일
- `orc auto -f` 경로를 추가해 기존 `input.md`를 사용한 자동 실행을 구현함.
- `src/code.rs`에 `auto_code_from_input_file()`를 추가하고, `input.md` 존재/파싱 검증 후 `init_code_project -> init_code_plan(없을 때만) -> add_code_plan -f -> add_code_draft -f -> impl_code_draft` 순으로 실행하도록 구성함.
- `add_code_plan -f` 시 대화형 `add_code_draft()` 확인 프롬프트를 띄우지 않도록 조건을 조정해 자동 흐름이 멈추지 않게 수정함.
- `src/cli.rs` usage/라우팅을 갱신해 `auto <message> | auto -f`를 지원하고, `auto -f`의 불필요한 추가 인자는 에러 처리함.
- `README.md` 명령 목록에 `orc auto -f` 동작(기존 input.md 사용, input.md 생성 생략, 구현까지 진행)을 반영함.
- 검증: `cargo run --bin orc -- --help`, `cargo test -q` 통과. `cargo run --bin orc -- auto -f` 실경로는 `init_code_project`, `add_code_plan -f` 완료 로그까지 확인했으며 이후 `add_code_draft -f` 단계는 장시간 실행으로 수동 종료함.

## 2026-03-05 - 작업한일
- `orc` 스킬 문서(`/home/tree/ai/skills/orc-cli-workflow/SKILL.md`)에 `orc auto -f` 사용 규칙을 추가함.
- 폴더가 사실상 비어 있고 `./input.md`만 존재하는 초기 상태에서는 `orc auto -f`를 우선 사용하고, `input.md` 생성 단계를 생략한 채 구현 단계까지 진행하도록 표준 실행 순서(0번 단계)로 명시함.

## 2026-03-05 - 작업한일
- 미사용 레거시 유틸 `scripts/run_drafts_parallel.py`와 파이썬 캐시(`scripts/__pycache__/run_drafts_parallel.cpython-313.pyc`)를 삭제함.
- `scripts` 디렉터리까지 함께 정리해 저장소에서 미참조 스크립트 경로를 제거함.
- 검증: `rg --files scripts` 결과 없음, `cargo test -q` 실행.

## 2026-03-05 - 작업한일
- 사용자 요청에 맞춰 소스코드(`src/*`) 한정으로 레거시 feature 경로 문자열 언급을 제거함.
- `src/draft.rs`의 삭제 확인 문구에서 `.project/feature/.../draft.yaml` 경로 표기를 제거함.
- `src/main.rs`의 check-code 지시 문자열 및 placeholder 설명 문자열에서 feature 경로 표기를 `.project/drafts.yaml` 기준으로 정리함.
- 검증: `rg -n "\.project/feature" src` 결과 없음, `cargo test -q` 통과.

## 2026-03-05 - 작업한일
- `add_code_draft`의 draft item 생성 경로에서 `assets/code/templates/draft_item.yaml` 템플릿을 실제로 로드해 LLM 프롬프트 입력으로 강제함(`src/code.rs`).
- `infer_draft_item` 응답 파싱을 확장해 템플릿 키(`name/scope/tasks/check`)를 직접 수용하고, 누락 시 보조 추론 결과로만 보완하도록 정리함.
- 관련 프롬프트(`assets/code/prompts/infer_draft_item.txt`, `add_code_draft_by_file.txt`, `add_code_draft_by_message.txt`)에 draft_item 템플릿 기반 스키마 규칙을 명시함.
- 검증: `cargo test -q` 통과, `draft_item.yaml` 참조 경로 확인 완료.

## 2026-03-05 - 작업한일
- 사용자 지시에 따라 `src/*` 내 `draft.yaml` 문자열 참조를 전부 제거하고 `drafts.yaml` 기준으로 일괄 치환함.
- `src/main.rs`, `src/ui/mod.rs`, `src/draft.rs`의 경로 후보/프롬프트/테스트 문자열을 `drafts.yaml` 기준으로 정리함.
- 검증: `rg -n "draft\.yaml" src` 결과 없음, `cargo test -q` 통과.

## 2026-03-05 - 작업한일
- 사용자 요청에 따라 `AGENTS.override.md`에 금지 표현 차단 및 중간 보고 금지 규칙을 강화함.
- 응답 전 금지 표현 점검/재작성 규칙을 명시해 위반 방지 절차를 고정함.
- 검증: `cargo test -q` 실행.

## 2026-03-05 - 작업한일
- `assets/code/templates/draft_item.yaml`에 필드 의미 주석을 추가해 LLM이 주석을 읽고 값 채우기를 수행할 수 있도록 복원함.
- `assets/code/prompts/infer_draft_item.txt`에 "주석 읽기 -> 값 채우기 -> 주석 제거" 규칙을 명시해 최종 출력에 주석이 남지 않도록 강제함.
- 검증: `cargo test -q` 통과.

## 2026-03-05 - 작업한일
- `/home/tree/.codex/AGENTS.override.md`의 완료 알림 강제 문구를 완화해 task 정책 충돌을 줄임.
- `/home/tree/ai/skills/plan-drafts/SKILL.md`, `/home/tree/ai/skills/build-code-parallel/SKILL.md`의 레거시 `draft.yaml`/`.project/feature` 중심 문구를 `drafts.yaml` 기준으로 정리함.
- `/home/tree/.config/fish/functions/level_init.fish`의 `vim .agents/AGENTS.md` 안내 문구를 제거함.
- 검증: 관련 문자열 검색 + `cargo test -q`.

## 2026-03-05 - 작업한일
- `build_input_md_auto()`를 `orc create_input_md` CLI 명령으로 직접 호출할 수 있도록 `src/code.rs`/`src/cli.rs`에 라우팅을 추가함.
- `orc auto -f` 실행 경로를 수정해 기존 `input.md` 유무와 무관하게 먼저 `orc create_input_md`를 실행한 뒤 다음 단계로 진행하도록 자동 흐름을 고정함.
- `README.md`와 `/home/tree/ai/skills/orc-cli-workflow/SKILL.md`에 `create_input_md` 및 갱신된 auto 흐름(선생성 후 진행)을 동기화함.
- 검증: `cargo run --bin orc -- --help`, `cargo test -q` 통과.

## 2026-03-05 - 작업한일
- `if)` 가상 시나리오 출력 규칙을 `AGENTS.override.md`에 추가해 줄 단위 `a -> b` 형식을 강제함.
- `/home/tree/ai/skills/virtual-scenario/SKILL.md`의 Mandatory Output/예시를 `a -> b` 다음줄 포맷으로 수정함.
- 동일 규칙으로 `.project/scenario.md`를 생성해 `orc auto "react todo"` 호출 흐름을 단계별 화살표 형식으로 기록함.
- 검증: `rg`로 규칙/예시 반영 확인.

## 2026-03-05 - 작업한일
- `~~~을 만들어줘` 입력 시 manager pane이 워커 pane을 단계별 생성/위임하도록 `AGENTS.override.md`에 동작 규칙을 추가함.
- `/home/tree/ai/skills/orc-cli-workflow/SKILL.md`에 `tmux split-window + orc send-tmux` 기반 manager-worker 강제 흐름을 추가함.
- `auto -> plan.yaml -> drafts.yaml -> impl(병렬) -> check_draft`를 각각 새 pane에서 수행하고 완료/실패를 회수해 재시도 판단하는 루프를 문서화함.
- 검증: 스킬 문서 키워드 검색 + `cargo test -q` 실행.

## 2026-03-05 - 작업한일
- `orc-cli-workflow` 트리거를 `~~~을 만들어줘`, `~~~을 추가해줘`, `~을 읽고 처리해줘`로 확장함.
- `읽고 처리해줘` 분기에서 기존 `input.md` 읽기 기반 명령(`add_code_plan -f`, `add_code_draft -f`)을 사용하고 `create_input_md`를 호출하지 않도록 스킬 규칙을 명시함.
- manager pane이 트리거별로 워커 pane 위임 명령을 분기해 수행하도록 단계/운영 예시를 갱신함.
- 검증: 문자열 검색 + `cargo test -q`.
