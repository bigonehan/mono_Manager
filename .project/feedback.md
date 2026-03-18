# 문제
- `cargo test`가 다수의 기존 unresolved symbol 오류(E0425)와 시그니처 불일치(E0061)로 실패했다.
- 실패 지점은 `chat/cli/draft/tui/ui/main` 전반이며, story 제거 파일과 직접 연관되지 않은 참조 깨짐이 포함된다.

# 미해결점
- 전체 테스트 통과 상태를 확인하지 못했다.
- story 제거 변경분의 회귀 검증은 정적 검색(`rg`) 중심으로만 확인된 상태다.

# 재시도 전략
- story 제거 범위 검증(`rg -n "\\bstory\\b|Story|assets/story" src README.md`)을 우선 통과시킨다.
- 동일 빌드 검증(`cargo test`)을 재실행해 실패가 동일한지 확인한다.
- 실패가 동일하면 현재 턴에서는 story 제거 반영 결과만 보고하고, 컴파일 깨짐은 별도 수정 범위로 분리한다.

## 추가 실패 기록
- `cargo install --path /home/tree/project/rust-orc`도 동일한 unresolved symbol/시그니처 오류로 실패했다.
- 재시도 전략: 현재 턴 범위(story 제거) 밖의 컴파일 깨짐을 먼저 복구한 뒤 install을 재실행한다.
