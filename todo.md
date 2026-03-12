# problem
- 음성 입력 결과 배열이 누적 갱신될 때 겹치는 구간을 그대로 이어 붙여, 동일 어절이 반복 입력된다.
- `impl_code_draft`가 시작 전에 전체 planned item을 한 번에 `worked`로 옮겨서, 실제로 실패하거나 아직 시작하지 않은 item도 `worked` 상태로 남는다.
- `check_code_draft -a`는 LLM follow-up이 timeout 나면 즉시 실패를 반환해 `report.md`를 남기지 못한다.
- `orc clit test -p . -m "PageEditor build verification"`은 web runner 기본 절차가 로그인 셀렉터를 강제해 비로그인 앱에서도 잘못된 smoke path를 탄다.

# tasks
- 음성 입력 훅은 result index 기준으로 transcript를 재구성하고, 겹치는 세그먼트는 한 번만 반영하도록 보정한다.
- 반복 어절이 들어오는 음성 인식 mock 시나리오를 web 테스트에 추가한다.
- 관련 회귀 테스트와 로그 문서 `.../.agents/log.md`를 함께 갱신한다.
- `impl_code_draft`는 item 시작 시점에만 `worked`로 기록하고, item별 성공/실패에 따라 즉시 `complete`/`error`를 저장하도록 상태 전이를 고친다.
- `check_code_draft`는 LLM follow-up 실패나 timeout 시에도 fallback report를 생성하고 종료하도록 바꾼다.
- `rc test`의 web 기본 절차는 로그인 UI를 전제하지 않는 generic smoke selector를 기본값으로 쓰고, login 모드는 명시적 요청일 때만 타도록 수정한다.
- 관련 회귀 테스트와 로그 문서 `.../.agents/log.md`를 함께 갱신한다.
- 재시도 사유: `rc` 직접 실행 형식을 잘못 써서 `test` subcommand usage error가 발생했다.
- 실패 원인 해결: 실제 smoke 확인은 `orc clit test ...` 브리지 또는 `rc`의 허용 형식으로 다시 실행한다.

# check
- `cd assets/web && npm run test:e2e -- --grep "voice input"`
- `cargo test`
- `cargo install --path /home/tree/project/rust-orc`
- `cargo test`
- `cd /home/tree/home/apps/web/PageEditor && cargo run --quiet --manifest-path /home/tree/project/rust-orc/Cargo.toml --bin orc -- impl_code_draft`
- `cd /home/tree/home/apps/web/PageEditor && cargo run --quiet --manifest-path /home/tree/project/rust-orc/Cargo.toml --bin orc -- check_code_draft -a`
- `cd /home/tree/home/apps/web/PageEditor && /home/tree/project/rust-orc/target/debug/rc test -p . -m "PageEditor build verification"`
- `cargo install --path /home/tree/project/rust-orc`
