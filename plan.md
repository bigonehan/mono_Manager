# plan

## 문제
- `orc auto`는 현재 메시지 기반 초기화만 수행하고, 기존 `input.md`를 활용한 구현 자동 진행 경로가 없다.
- 사용자 요구는 `orc auto -f`에서 `input.md` 생성을 생략하고 기존 파일을 사용해 최종 구현(`impl_code_draft`)까지 수행하는 것이다.

## 해결책
1. `src/cli.rs`의 `auto` 파싱을 확장해 `auto -f`를 허용한다.
2. `src/code.rs`에 `auto_code_from_input_file()`를 추가해 `input.md` 존재 확인 후 `init_code_project -> init_code_plan(없을 때만) -> add_code_plan -f -> add_code_draft -f -> impl_code_draft` 순으로 실행한다.
3. `add_code_plan -f` 경로에서 불필요한 대화형 프롬프트를 띄우지 않도록 조건을 조정한다.
4. `README.md`와 CLI usage를 업데이트한다.

## 검증
- `orc --help`에 `auto [-f] <message>`가 반영된다.
- `cargo test -q`가 통과한다.
