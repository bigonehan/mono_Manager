# info
name : rust-orc
description : hello world 출력
spec : next js
path : /home/tree/project/rust-orc

# features
- rust_cli_workspace
- project_documentation
- cli_rust_orchestra
- cli_help
- cli_create_code_draft
- cli_impl_code_draft
- cli_test
- cli_check_task
- cli_check_draft
- cli_open_ui
- cli_init_code_project
- cli_init_code_plan
- cli_add_code_plan
- cli_add_code_draft
- cli_add_code_draft_item
- cli_check_code_draft
- cli_auto
- cli_send_tmux
- cli_enter
- cli_chat
- cli_chat_wait

# rules
- 모든 기능은 `/home/tree/project/rust-orc` 경로 기준으로 동작한다.
- CLI 명령은 단일 진입점에서 일관된 인자 파싱/에러 메시지 규칙을 따른다.
- `hello world 출력` 요구사항은 기본 실행 경로에서 항상 재현 가능해야 한다.
- 문서화는 실제 CLI 동작과 불일치가 없도록 유지한다.
- draft/plan/task 검증 계열 명령은 상태 기반으로 선행 조건을 확인한 뒤 실행한다.
- 자동화 명령(`cli_auto`)은 내부 단계 실패 시 즉시 중단하고 실패 원인을 반환한다.
- tmux 연동 명령은 대상 세션/패널 식별자 검증 후 전송한다.
- UI 오픈 명령은 CLI 결과와 동일한 상태를 조회/표시해야 한다.

# constraints
- Rust 워크스페이스 구조를 유지하고 바이너리/라이브러리 경계를 임의로 변경하지 않는다.
- Next.js 스펙은 UI 진입점 용도로 유지하며 CLI 핵심 로직을 UI 전용 로직으로 대체하지 않는다.
- 신규 기능 추가 시 기존 명령 이름/행동 계약을 깨는 변경을 금지한다.
- 명령 실행 결과는 성공/실패 코드와 사람이 읽을 수 있는 메시지를 함께 제공해야 한다.
- 테스트/체크 명령은 네트워크 비의존 기본 경로에서 재현 가능해야 한다.
- 문서 파일은 구현보다 앞서 과장된 기능 설명을 포함하지 않는다.

# domains
## workspace
### states
- uninitialized
- initialized
- configured
- ready
### action
- 워크스페이스 초기화
- 기본 설정 파일 로드
- 명령 실행 환경 점검
### rules
- 워크스페이스 루트는 `path` 정보와 일치해야 한다.
- 초기화 이전 상태에서 실행 불가능한 명령은 차단한다.
- 상태 전이는 `uninitialized -> initialized -> configured -> ready` 순서를 따른다.
### constraints
- 루트 경로 외부를 기준으로 상대 경로 연산을 수행하지 않는다.
- 필수 설정 누락 시 `ready` 상태로 전이하지 않는다.

## documentation
### states
- missing
- draft
- synced
### action
- 프로젝트 문서 생성
- 기능 목록 동기화
- 규칙/제약 검증
### rules
- 문서의 기능 목록은 `# features`와 동일한 식별자를 사용한다.
- 문서 상태는 구현 상태보다 앞서 확정되지 않는다.
- 변경 이력은 추적 가능한 단위로 반영한다.
### constraints
- 문서에 플레이스홀더 텍스트를 남기지 않는다.
- 구현되지 않은 명령을 완료된 기능으로 표기하지 않는다.

## draft_management
### states
- empty
- created
- item_added
- validated
- implemented
### action
- 코드 draft 생성
- draft 항목 추가
- draft 검증
- draft 기반 구현
### rules
- draft 항목은 실행 가능한 작업 단위로 작성한다.
- 검증 실패 draft는 구현 단계로 진행하지 않는다.
- 구현 완료 후 draft 상태를 `implemented`로 갱신한다.
### constraints
- draft 항목 간 중복 책임을 허용하지 않는다.
- 필수 입력 누락 상태에서 draft 생성/추가를 진행하지 않는다.

## task_validation
### states
- pending
- checking
- passed
- failed
### action
- task 점검 실행
- draft 점검 실행
- 코드 draft 점검 실행
### rules
- 점검 명령은 동일 입력에서 결정론적 결과를 제공해야 한다.
- 실패 시 실패 이유와 대상 항목을 함께 반환한다.
- `passed` 상태는 모든 필수 체크 통과 시에만 부여한다.
### constraints
- 점검 과정에서 사용자 데이터/소스 파일을 임의 수정하지 않는다.
- 체크 우회를 위한 강제 성공 플래그를 기본 동작에 포함하지 않는다.

## execution_orchestration
### states
- idle
- running
- waiting
- completed
- error
### action
- 오케스트레이션 실행
- 자동 실행
- 명령 체인 중단/재개
### rules
- 오케스트레이션은 단계별 시작/종료 로그를 남긴다.
- 단계 실패 시 후속 단계 실행을 중단한다.
- `cli_chat_wait` 상태는 명시적 완료 신호로만 해제된다.
### constraints
- 무한 대기 상태를 방치하지 않도록 타임아웃 또는 중단 경로를 제공한다.
- 동시 실행 시 동일 리소스에 대한 충돌을 허용하지 않는다.

## tmux_chat
### states
- disconnected
- connected
- sending
- waiting
### action
- tmux 세션 진입
- 메시지 전송
- 채팅 명령 실행
- 응답 대기
### rules
- 전송 전 세션/패널 존재 여부를 확인한다.
- 전송 실패는 재시도 가능 상태로 보고한다.
- 채팅 명령은 요청-응답 단위를 보존한다.
### constraints
- 유효하지 않은 세션 식별자로 전송을 시도하지 않는다.
- 대기 상태 전환 시 취소 경로를 제공해야 한다.

## ui_bridge
### states
- closed
- opening
- opened
- synced
### action
- UI 열기
- CLI 상태 조회
- UI 상태 동기화
### rules
- UI에 노출되는 상태는 CLI의 최신 결과와 일치해야 한다.
- UI 오픈 실패 시 원인과 복구 방법을 반환한다.
- UI 동기화는 읽기 전용 조회를 우선한다.
### constraints
- UI 레이어가 CLI 핵심 도메인 규칙을 우회하지 않는다.
- UI 전용 상태를 단독 진실 원천으로 사용하지 않는다.
