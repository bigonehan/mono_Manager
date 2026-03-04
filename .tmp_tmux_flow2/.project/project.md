# info
name : tmux_flow_verification
description : tmux flow verification
spec : tmux, bash, shell script
path : /home/tree/project/rust-orc/.tmp_tmux_flow2

# features
- workspace_bootstrap

# rules
- 모든 실행 스크립트는 `bash` 기준으로 동작해야 한다.
- tmux 세션/윈도우/패인 이름은 충돌 없이 재실행 가능한 규칙으로 관리한다.
- 스크립트는 동일 입력에 대해 멱등하게 동작해야 한다.
- 실패 시 즉시 비정상 종료 코드와 원인 메시지를 출력한다.
- 경로 기준은 프로젝트 루트(`/home/tree/project/rust-orc/.tmp_tmux_flow2`)로 고정한다.

# constraints
- `tmux`와 `bash`가 설치된 환경에서만 동작한다.
- 비대화형 실행을 기본으로 하며 사용자 입력 대기를 요구하지 않는다.
- 프로젝트 루트 외부 파일/세션 상태를 임의로 변경하지 않는다.
- 검증 가능한 명령 로그를 남겨야 한다.

# domains
## workspace_bootstrap
### states
- requested
- session_prepared
- panes_configured
- bootstrap_verified
- failed
### action
- tmux 세션 생성 또는 기존 세션 재사용 여부 판별
- 윈도우/패인 레이아웃 구성
- 각 패인에 초기 명령 주입
- 실행 결과 및 상태 검증
### rules
- 상태 전이는 `requested -> session_prepared -> panes_configured -> bootstrap_verified` 순서를 따른다.
- 중간 단계 실패 시 상태는 즉시 `failed`로 전환한다.
- 이미 준비된 세션이 있으면 중복 생성하지 않고 검증 단계로 진행한다.
- 모든 동작은 재실행 시 동일 결과를 보장해야 한다.
### constraints
- 세션 이름은 고정 규칙을 사용해 중복 충돌을 방지해야 한다.
- 패인 구성은 최소 1개 이상이며 검증 가능한 명령이 포함되어야 한다.
- 실패 상태에서는 추가 동작을 진행하지 않는다.
