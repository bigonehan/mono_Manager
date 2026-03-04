# tmux
- 모든 자동화 실행 전에 `tmux` 세션 존재 여부를 먼저 검증한다.
- 이미 존재하는 세션은 재사용하고 중복 생성하지 않는다.
> `tmux list-sessions` 결과로 대상 세션 존재 여부를 판별한다.
> 세션이 없을 때만 생성하고, 명령 실패 시 1회 이내 재시도 후 실패 로그와 종료 코드를 반환한다.

# tmux
- 상태 판별은 `tmux list-sessions`, `tmux list-windows`, `tmux list-panes` 결과만 사용한다.
- 검증 대상 세션/윈도우 이름은 입력값 또는 고정 규칙 중 하나로 일관 처리한다.
> 세션/윈도우/패인 정보를 조회해 `session_checked`부터 `split_configured`까지 순차 상태를 기록한다.
> 조회 결과가 기준과 다르면 `verification_failed`로 전이하고 종료한다.

