# info

- name : rust-orchestra
- description : LLM 협업 기반으로 프로젝트 설계, 초안 작성, task 실행을 관리하는 Rust CLI/TUI 프로그램
- spec : rust(edition 2021), ratatui, tokio
- goal : 프로젝트 설계부터 draft/task 실행까지를 한 흐름으로 연결하고, 누락된 작업 정의를 LLM으로 보강해 구현 속도와 일관성을 높인다.

## rule

- 모든 핵심 기능은 CLI 명령으로도 실행 가능해야 하며, TUI는 동일 기능의 시각적 제어 레이어로 동작한다.
- codex 실행 시 자동 확인(`-y` 계열) 옵션은 설정값으로 on/off 가능해야 한다.
- 실행 실패 또는 대기시간 초과는 `프로그램 실행 위치/.project/log.md`에 기록 후 실패 상태로 종료한다.
- 기능 추가 요청은 직접 기능 확장보다 `draft-create`, `draft-add` 흐름으로 유도해 문서 기반으로 반영한다.

## features
1. 프로젝트 생성/추가/삭제/선택 명령 실행 | `src/main`, `src/ui`, `configs/project.yaml` | 프로젝트 관리 진입점 확보
2. draft 생성/추가/삭제 플로우 실행 | `src/ui`, `src/assets/templates/draft.yaml`, `.project/features/work/*/draft.yaml` | 기능 명세 문서 자동화
3. task 리스트 생성 및 의존성 검증 실행 | `.project/features/work/*/draft.yaml`, `src/*` | 구현 가능한 task 단위 확정
4. 병렬 build 실행 명령 처리(`run_parallel`) | `src/main`, `src/config`, `src/ui`, `src/assets/templates/prompts/tasks.txt` | tokio 기반 병렬 codex 실행 및 상태 표시
5. 3-패널 UI 포커스/활성 상태 전환 처리(arrow/enter/esc) | `src/ui`, `src/assets/style/*` | project/draft/task pane 상호작용 및 esc 2회 정책 보장
6. 실패/타임아웃 로깅 실행 | `.project/log.md`, `src/*` | 운영 추적성과 실패 원인 기록 확보
7. 언어/스택 기반 프로젝트 초기화 실행 | `src/main`, `src/config`, `Cargo.toml`, `src/main.rs` | rust + ratatui + tokio 초기 실행 가능한 워크스페이스 구성

## structure
- `src/main.rs` : CLI 엔트리포인트, 서브커맨드 라우팅, 실행 모드 분기
- `src/ui/*` : 3-패널 렌더링, 포커스/활성 전환, 키 이벤트 처리
- `src/config/*` : 설정 로딩/검증(`max_parallel`, `auto_yes`, keymap)
- `src/tmux/*` : tmux pane 대상 명령 문자열 전송(세션 수명 관리 제외)
- `src/assets/templates/project.md` : 프로젝트 설계 문서 템플릿
- `src/assets/templates/draft.yaml` : 기능 초안 템플릿
- `src/assets/templates/prompts/tasks.txt` : codex task 실행 프롬프트 템플릿
- `configs/project.yaml` : 등록 프로젝트 메타 목록 저장소
- `.project/project.md` : 선택 프로젝트의 설계 기준 문서
- `.project/features/work/<feature>/draft.yaml` : 기능 단위 draft 문서
- `.project/log.md` : 실패/타임아웃 운영 로그

# Domains

도메인은 더 이상 쪼개기 어려운 실행 책임 단위로 정의하며, 각 도메인은 입력/상태/출력을 명시한다.

### project
- **name**: `project`
- **Description**: 프로젝트 등록/선택/삭제 및 작업 루트 전환 책임
- **Inputs**: CLI 인수, `configs/project.yaml`, 사용자 선택 이벤트
- **States**: added, selected, removed, working, completed
- **Actions**: create, add, select, delete, load
- **Outputs**: 현재 작업 경로, project 메타정보, `.project/project.md` 초기 상태
- **rule**:
  - 프로젝트 경로와 메타정보를 단일 소스(`configs/project.yaml`)에서 관리한다.
  - 대상 경로에 `.project`가 있으면 신규 생성 대신 기존 프로젝트 로드로 처리한다.
- **variable**:
  - name
  - path
  - description
  - created_at
  - updated_at

### draft
- **name**: `draft`
- **Description**: 기능 명세(`draft.yaml`) 생성/추가/삭제 및 유효성 검증 책임
- **Inputs**: `plan-drafts-code` 결과, 템플릿, 사용자 편집값
- **States**: created, updated, ready, removed, completed
- **Actions**: create, update, append, delete, validate
- **Outputs**: `.project/features/work/<feature>/draft.yaml`, task 생성 가능한 구조화 데이터
- **rule**:
  - `src/assets/templates/draft.yaml` 스키마를 따른다.
  - 기능 추가 요구는 도메인 코드 확장보다 `draft-create`/`draft-add`로 먼저 반영한다.
- **variable**:
  - feature_name
  - draft_path
  - tasks
  - depends_on

### task_execution
- **name**: `task_execution`
- **Description**: task 생성, 의존성 점검, 병렬 실행, 실패 기록 책임
- **Inputs**: draft tasks, `project.md(info/spec)`, config(`max_parallel`, `auto_yes`)
- **States**: inactive, active, blocked, failed, clear
- **Actions**: generate_tasks, check_depends_on, run_parallel, mark_state, write_log
- **Outputs**: task 상태 맵, 실행 결과, `.project/log.md` 기록
- **rule**:
  - `depends_on` 미완료 task는 `blocked` 처리하고 실행하지 않는다.
  - tokio 세마포어로 `max_parallel` 동시 실행 수를 강제한다.
  - 실패/timeout은 즉시 로그 기록 후 재시도 없이 실패 종료한다.
  - codex 자동 확인 옵션은 config 기반으로 주입/미주입을 분기한다.
- **variable**:
  - max_parallel
  - auto_yes
  - queue
  - task_status_map
  - timeout_seconds

### workspace_ui
- **name**: `workspace_ui`
- **Description**: 3-패널 레이아웃 제어, 포커스 이동, 실행 상태 가시화 책임
- **Inputs**: 키 이벤트(arrow/enter/esc/p), task 상태 업데이트 이벤트
- **States**: inactive, active
- **Actions**: focus_move, activate, deactivate, open_modal, render_status
- **Outputs**: 현재 활성 pane, 모달 상태, 실시간 상태 렌더링
- **rule**:
  - 화살표 입력은 포커스만 이동시키고 enter 입력 시 활성화한다.
  - esc 입력은 활성 해제 우선, 리스트 선택 상태에서는 esc 2회 정책을 적용한다.
  - task 상태(inactive/active/blocked/failed/clear)를 패널에 반영한다.
- **variable**:
  - focused_pane
  - active_pane
  - modal_state
  - selected_index_map

# Flow

## stage list
1. project 관리 모드 진입 및 대상 프로젝트 선택/생성
2. draft 생성 또는 추가로 기능 명세 확정
3. task 생성 및 의존성 검증
4. 병렬 실행(run_parallel) 및 상태/로그 반영

## UI

### project menu
- description: 프로젝트 목록 메인 화면
- flow: project info, detail menu
- domain: project
- action: 프로젝트 생성, 프로젝트 추가, 프로젝트 삭제, detail 진입
- rule:
  1. `configs/project.yaml`에서 프로젝트 목록을 로드한다.
  2. 목록에 이름/생성일/최근수정일/설명을 표시한다.
  3. 선택 항목에서 enter 입력 시 detail menu로 이동한다.

### detail menu
- description: 선택 프로젝트 대시보드
- flow: project menu (esc)
- domain: project, draft, task_execution, workspace_ui
- action: draft 생성, task 생성, 병렬 처리 시작
- structure:
  - project info pane: 현재 프로젝트 경로, spec, 진행 상태 표시
  - draft pane: draft 목록 표시, enter 상세, `draft-create`/`draft-add` 실행 진입
  - task pane: task 상태 실시간 표시, `config.keymap.run_parallel` 입력 시 병렬 실행
- rule:
  1. draft가 없으면 task 생성/병렬 실행 진입을 제한하고 draft 생성 안내를 표시한다.
  2. 병렬 실행 중에는 중복 실행 입력을 차단하고 진행 모달만 갱신한다.

# Step

## 프로젝트
### 프로젝트 생성
1. Main menu에서 project 생성 선택
2. 프로젝트 위치 설정(기본값: 프로그램 실행 위치)
3. 대상 경로에 `.project` 존재 여부 확인
4. 없으면 `jj git init` 실행 후 `.project/project.md` 템플릿 생성
5. tmux pane로 codex `plan-project-code` 실행 명령 전달
6. 필요 시 codex `build-domain` 실행 명령 전달
7. `project.md`의 `info.spec`를 읽어 초기 스택(`cargo init`, `ratatui`, `tokio`) 반영
8. 폴더가 비어있을 때만 초기화 명령을 실행하고 기존 코드가 있으면 병합 모드로 전환

## draft
### draft 생성 (`draft-create`)
1. detail menu에서 draft 생성 액션 선택
2. tmux pane로 codex `plan-drafts-code` 실행 명령 전달
3. `.project/features/work/<feature>/draft.yaml` 생성 여부와 스키마 유효성 확인

### draft 추가 (`draft-add`)
1. 기존 draft 목록에서 추가 대상 기능 선택
2. tmux pane로 codex `plan-drafts-code` 기반 추가 명령 전달
3. 기존 draft와 충돌(중복 task id, depends_on 불일치) 여부 검증

### draft 삭제
1. draft list에서 항목 선택 후 `d`
2. `y/n` 확인 후 파일 삭제 및 UI 목록 갱신

## 병렬 처리
### 병렬 실행
1. task pane에서 `config.keymap.run_parallel`(기본 `p`) 입력
2. `run_parallel_build_code` 실행
3. `./.project/features/work/*/draft.yaml` 수집
4. 각 draft task + `./.project/project.md`의 info/spec를 합쳐 codex 입력 구성
5. `src/assets/templates/prompts/tasks.txt` 포맷으로 작업 프롬프트 생성
6. tokio로 `max_parallel` 동시 실행 제한 적용
7. 진행 모달에 task 상태(inactive/active/blocked/failed/clear) 실시간 반영
8. 실패/timeout 발생 시 `./.project/log.md` 기록 후 해당 task 실패 처리
9. 전체 task 종료 후 결과 요약(성공/실패 수) 표시

# Constraints

## ui
- 3개 pane(project/draft/task)의 focus/active 상태를 분리 관리한다.
- 모든 pane 포커스 이동은 화살표 키만 사용한다.
- list item 선택 상태에서 esc 2회 정책을 지켜 오동작(즉시 이탈)을 방지한다.
- 병렬 실행 상태는 최소 1초 이하 주기로 화면에 갱신 가능해야 한다.

## task
- 동시 실행 수는 `config.yaml`의 `max_parallel` 값을 사용한다(기본 10).
- `depends_on` 미완료 task는 실행하지 않고 `blocked`로 표시한다.
- 실패 task는 재시도하지 않고 로그 기록 후 종료한다.
- codex 실행 시 자동 y 옵션은 `auto_yes` 설정값으로만 제어한다.
- task 입력 데이터는 draft 스키마 검증 통과 항목만 사용한다.

## tmux
- tmux는 명령 전달 전용이며 세션 생성/삭제/복구 책임을 갖지 않는다.
- 병렬 스케줄링과 상태관리는 tokio 런타임에서 처리한다.

## log
- 위치: `프로그램 실행 위치/.project/log.md`
- 기록 조건: task 실행 실패, 실행 대기시간 초과, codex 프로세스 비정상 종료
- 기록 항목: task 이름, feature 이름, 실패 시각(ISO8601), 실패 사유, 재현 명령 요약
- 로그 쓰기 실패 시 stderr에 즉시 경고를 출력하고 task를 실패 처리한다.

# Verification
- [미완] [project]가 [생성]되었을 때 [`.project/project.md`가 템플릿 구조로 생성]된다.
- [미완] [project]가 [기존 `.project` 경로에서 선택]되었을 때 [신규 초기화 없이 기존 설정이 로드]된다.
- [미완] [draft-create]가 [실행]되었을 때 [`.project/features/work/<feature>/draft.yaml`이 생성]된다.
- [미완] [draft-add]가 [실행]되었을 때 [기존 draft에 task가 추가되고 depends_on 검증을 통과]한다.
- [미완] [run_parallel]이 [실행]되었을 때 [max_parallel 제한 하에서 task 상태가 실시간 표시]된다.
- [미완] [task 실패/timeout]이 [발생]했을 때 [`.project/log.md`에 필수 항목이 기록]된다.

# Gate Checklist
- 모호함 해소: 완료(사용자 입력 요구사항 1~4 및 출력 규칙 반영)
- `.project/project.md` 생성/최신화: 완료(현재 문서 최신화)
- 완료 기준 문서화: 완료(Verification 시나리오 6건 명시)
- domain-create 스킬 실행 여부: 미실행(현재 단계는 문서 상세화 편집 범위)
- QA 1회 왕복 여부: 완료(요구사항 입력 1회 기준)
- features 개수(3~7): 완료(기존 7개 유지, 불필요한 기능 추가 없음)
- Flow stage 정합성: 완료(stage 1~4와 Step/Domain 연결 확인)
- Constraints/Verification 미완 표기: 완료(Verification 전 항목 `[미완]` 유지)