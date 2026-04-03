# plan
- ORC 세션이 `plan -> job.md -> drafts.yaml -> 병렬 draft_item -> check` 체인을 건너뛰고 유닛테스트만으로 검증 완료 처리하는 원인을 고정한다.
- `check_orc_code`가 실행 증거 없이 verify를 complete로 옮기지 못하게 막는다.
- 구현/검증 프롬프트와 문서에서 `유닛테스트 pass = 완료`로 읽히는 흐름을 제거한다.
- ORC 체인을 다시 실행해 `job.md`, `drafts.yaml`, 병렬 구현, 검증 결과를 남긴다.

# requirement
## orc_check_requires_execution_evidence
1. verify 단계는 `job.md#check evidence`를 먼저 읽고 unresolved를 문제로 승격한다.
2. unresolved가 없을 때만 verify를 complete로 이동한다.
- `check_orc_code`는 `job.md#check evidence` 실행 근거가 없으면 성공 완료를 만들지 않는다.
- `job.md#check evidence`에 `[ ]` 항목이 있으면 `job.md#problems`에 남기고 verify 상태를 유지한다.
## orc_prompt_rejects_unit_test_only_completion
1. build/check 안내 문구를 실행 근거 중심으로 교체한다.
2. 문서와 프롬프트를 같은 기준으로 맞춘다.
- 구현 프롬프트와 검증 프롬프트는 유닛테스트 통과만으로 완료라고 쓰지 않는다.
- README에도 실행 증거 기반 완료 규칙을 반영한다.

# task
## planned
## work
## verify
## complete
- orc_check_requires_execution_evidence
- orc_prompt_rejects_unit_test_only_completion
## fail

# problems

# check

# check evidence
- [x] orc_check_requires_execution_evidence -> execution evidence required : `cargo test check_orc_code -- --nocapture`
- [x] orc_prompt_rejects_unit_test_only_completion -> prompt and docs updated : `rg -n 'job.md#check evidence|유닛테스트 통과만으로 완료 처리하지 않는다|유닛 테스트는 보조 검증일 뿐 완료 판정이 아니다' README.md assets/prompts/build_parallel.md assets/prompts/check_code.md src/code.rs`
- [x] clit deprecation -> legacy check path blocked : `orc clit test -p . -m "orc process regression"`
