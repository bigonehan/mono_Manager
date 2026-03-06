## 문제
- `assets/code/templates/plan.yaml`이 루트 `planned/worked/complete`를 포함해 목표 스키마와 다르다.
- 일부 함수/프롬프트가 루트 상태 필드를 암묵적으로 허용하거나 참조해 스키마 일관성이 깨진다.

## 해결책
- `assets/code/templates/plan.yaml`에서 루트 `planned/worked/complete`를 제거한다.
- `src/code.rs`의 `CodePlanDoc`에서 루트 호환 필드를 제거하고 `drafts.*`만 사용하도록 정리한다.
- `assets/code/prompts`에서 `plan.yaml` 상태 필드 설명을 `drafts.planned/worked/complete` 기준으로 수정한다.
- 작업 완료 후 `./.agents/log.md`에 완료 기록을 추가한다.

## 검증
- `rg -n "^planned:|^worked:|^complete:" assets/code/templates/plan.yaml` 결과가 없어야 한다.
- `rg -n "plan\.yaml.*planned/worked/complete|plan\.yaml.*planned" assets/code/prompts src/code.rs`로 참조 문구를 점검한다.
- `cargo test -q` 실행으로 회귀 검증한다.
- `cargo install --path /home/tree/project/rust-orc`를 실행한다.
