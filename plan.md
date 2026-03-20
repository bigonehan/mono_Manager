## 문제
- `impl_orc_code` 실행 시 `drafts.yaml`/`job.md` 상태 전이가 오케스트레이터 함수로 강제되지 않아, 작업 상태 동기화가 불명확하다.
- LLM 구현 단계에서 상태 파일을 직접 수정하지 못하도록 역할 분리가 필요하다.

## 해결책
- `src/code.rs`에 상태 전이 전용 함수 2개를 둔다.
  - draft item state 변경 함수
  - job task list 이동 함수
- `impl_orc_code` 흐름을 `시작 전 work 전이 -> LLM 실행 -> 성공/실패 후 상태 전이`로 고정한다.
  - 성공: `draft.state=complete`, `job.task=check`
  - 실패: `draft.state=error`, `job.task=fail`
- `assets/prompts/build_parallel.md`에 `drafts.yaml`/`job.md` 직접 수정 금지 규칙을 명시한다.

## 검증
- `cargo test`
- `cargo install --path /home/tree/project/rust-orc`
