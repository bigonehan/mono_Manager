## 문제
- `rc clit test`의 step 실행은 장시간 무응답 구간 동안 heartbeat가 없어 hang처럼 보인다.
- `orc clit ...` forwarding 경로도 `cargo run --bin rc`를 기다리는 동안 중간 상태를 보여주지 않는다.

## 해결책
- `src/bin/rc.rs`의 step 실행을 polling 기반으로 바꿔 장시간 step마다 주기적인 heartbeat를 stdout/stderr와 execution record에 남긴다.
- `src/main.rs`의 `run_rc_forward`도 장시간 대기 중 heartbeat를 출력해 forwarding 경로의 무응답 구간을 줄인다.
- 회귀를 막기 위해 heartbeat 메시지 포맷을 검증하는 단위 테스트를 추가한다.

## 검증
- `cargo test`
- `env PATH="/tmp/orc-fake-bin-...:$PATH" timeout 20 target/debug/rc clit test -p <slow-target> -m smoke`
- `env PATH="/tmp/orc-fake-bin-...:$PATH" timeout 20 target/debug/orc clit test -p <slow-target> -m smoke`
