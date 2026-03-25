## 문제
- `cli_impl_code_draft` 요구사항에서 생성되는 draft item의 `rule` 값에 공백/빈 문자열이 포함되면 그대로 저장된다.
- requirement 기반 draft item 생성에서 빈 rule을 제거하지 않으면 후속 검증/비교 시 노이즈가 생긴다.

## 해결책
- `build_draft_item_from_requirement` 시작부에서 `req.rules`를 trim + 빈 값 제거로 정규화한다.
- 정규화된 rule 배열을 draft item의 `rule` 필드에만 반영하고 기존 생성 포맷(scope/step/tasks/constraints/check)은 유지한다.
- 단위테스트를 먼저 추가해 빈 rule 제거 동작을 고정한다.

## 검증
- `cargo test build_draft_item_from_requirement_filters_blank_rules -- --nocapture`
- `cargo test build_draft_item_from_requirement -- --nocapture`
