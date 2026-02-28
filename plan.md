# plan

## 문제
- draft 생성 시 YAML 스키마 오류(duplicate key, contracts 형식 불일치)가 반복된다.
- 원인 후보: draft 생성 프롬프트의 형식 제약이 충분히 강하지 않다.

## 해결책
1. draft 프롬프트/템플릿/보정 프롬프트를 전수 점검한다.
2. YAML 스키마 제약(금지 키, 필수 키, contracts 형식, duplicate key 금지)을 명시적으로 강화한다.
3. 수정 후 `cargo test`로 회귀 확인한다.

## 검증
- 프롬프트 본문에 스키마 제약이 명시되어야 한다.
- 동일 오류 재현 케이스에서 duplicate rule/비구조 contracts 발생률이 낮아져야 한다.
