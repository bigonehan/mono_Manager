# 구현 확인
- targets: set -euo pipefail 적용, 실패 시 즉시 중단, tmux list-sessions/list-windows/list-panes 기반 상태 판별 구현, tmux 세션 존재 확인 및 필요 시 생성(중복 생성 방지), 대상 윈도우/패인 분할 구성 적용, 성공/실패 기준 로그 출력 및 종료 코드 반환, 패인 수와 레이아웃 일치 여부 검증, 프로젝트 루트 작업 디렉터리 유효성 확인 로직 구현
- check_followup: check-code follow-up: CHECK_CODE_RESULT
- test: test skipped: Cargo.toml not found
- debug_pane: %23

# 발견된 문제
- 없음
