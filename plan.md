## 문제
- `story` 관련 코드/프로파일이 남아 있어 legacy 경로(`assets/story/...`)와 story 명령 진입점이 계속 노출된다.
- 요청사항은 story 관련 부분 전체 제거다.

## 해결책
- `src/main.rs`에서 `mod story;` 제거.
- `src/profile/mod.rs`에서 Story 전용 타입/구현/매핑(`is_known_profile_name`, `resolve_profile`) 제거.
- `src/cli.rs` usage의 profile 목록에서 `story` 제거.
- `src/web_api/mod.rs`의 `ProjectType`에서 `story` alias 제거.
- `src/story.rs` 파일 삭제.

## 검증
- `rg -n "\\bstory\\b|Story|assets/story" src README.md`
- `cargo test`
- `cargo run --bin orc -- --help`
- `cargo install --path /home/tree/project/rust-orc`

## 재시도 반영
- `cargo test` 실패 원인은 story 제거와 독립적인 기존 unresolved symbol/시그니처 불일치 오류로 확인됨.
- 이번 턴 강제 실행 항목: story 제거 완료 여부를 `rg`로 먼저 확정하고, `cargo test`를 동일 조건으로 1회 재시도한다.
- `cargo install`도 동일 오류로 실패하여, install 재시도는 컴파일 깨짐 복구 이후 단계로 이관한다.
