## 문제
- `orc add_orc_drafts` 단계에서 LLM이 자연어 설명/머리말을 함께 반환해 YAML 파서가 반복 실패한다.
- 현재 구현은 `assets/prompts/init_project.md`를 재사용해 draft item 생성 프롬프트가 목적에 맞지 않는다.

## 해결책
- `add_orc_drafts`에서 draft-item 전용 프롬프트를 사용하도록 교체하고, 출력 형식을 YAML 단일 item으로 강제한다.
- 응답 파싱은 `YAML code fence 추출 -> 전체 본문 정리 -> 단일 item 파싱` 순서로 처리하고, 파싱 실패 시 1회 정규화(reformat) 재시도를 수행한다.
- 실패 메시지에 requirement 이름을 포함해 재시도/원인 추적 가능성을 높인다.

## 검증
- `timeout 180s orc add_orc_drafts` (실패 시 동일 단계 최대 2회 재시도)
- `timeout 180s orc impl_orc_code`
- `timeout 180s orc check_orc_code`
- `timeout 180s orc clit test -p . -m "add_orc_drafts parser output guard"`
