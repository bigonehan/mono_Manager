# 문제
- `cli_rust_orchestra`가 workspace가 ready이면 성공을 반환하지만, `job.md`에 `cli_rust_orchestra` requirement가 없어도 성공할 수 있다.
- 이 경우 "requirement 기반 draft item 생성" 제약을 만족하지 못한 채 완료 메시지가 나올 수 있다.

# 해결책
- `flow_rust_orchestra` 경로에 요구 feature(`cli_rust_orchestra`) draft 생성 보장 검증을 추가한다.
- 먼저 테스트를 추가해 requirement 부재 시 실패를 기대하도록 만들고, 이후 구현을 보완한다.

# 검증
- 단위 테스트: requirement 부재 시 `flow_rust_orchestra`가 에러를 반환한다.
- 단위 테스트: requirement 존재 시 기존처럼 draft를 생성하고 성공 메시지를 반환한다.
