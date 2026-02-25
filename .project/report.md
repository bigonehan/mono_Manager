# Code Review Report

## Findings

1. [High] 기존 프로젝트 호환 경로 제거로 draft 목록 소실 위험
- 위치: /home/tree/project/rust-orc/src/main.rs:147
- 위치: /home/tree/project/rust-orc/src/ui/mod.rs:3006
- 내용: `tasks_list.yaml` fallback/migration 경로가 제거되어, 기존에 `drafts_list.yaml`가 없는 프로젝트는 planned/features를 읽지 못하고 빈 상태로 동작할 수 있다.
- 영향: 기존 프로젝트에서 Draft/Parallel 진입 시 planned가 비어 보이거나 후속 단계가 실패할 가능성.

2. [Medium] 도메인 추출 파서가 포맷 편차에 취약
- 위치: /home/tree/project/rust-orc/src/main.rs:661
- 내용: `calc_extract_project_md_domain_names`가 `- **name**:` 완전 일치 라인만 허용한다. 공백 변형(예: `- **name** :`)이나 대소문자/마크다운 편차가 있으면 도메인 목록이 비어버린다.
- 영향: planned 네이밍에서 도메인 기반 접두어/토큰 선택 품질 저하.

3. [Medium] `.project/scenario.md` 형식 불일치
- 위치: /home/tree/project/rust-orc/.project/scenario.md
- 내용: check-code 규칙의 요구 형식(`명령 | 실행/변경 파일 | 파생 결과` 1줄)과 달리 `->` 중심 문장들이 다수다.
- 영향: 시나리오 기반 검증 자동화/추적 신뢰도 저하.

## Check-Code Phases

### PHASE 1
- Q1 전역 선언: 있음 (const 경로/설정 상수들).
- Q2 변경 가능 전역: 확인된 mutable global 없음.

### PHASE 2
- Q3~Q5 분류 결과 요약:
  - ACTION: 파일 I/O, LLM 호출, CLI 실행 함수들(`action_*`).
  - CALC: 문자열/키/파싱 보조 함수(`calc_*`).

### PHASE 3
- Q6 계산 함수 인자 mutation: 이번 점검 범위에서 직접 mutation 패턴 미발견.
- Q7 중첩 복사: 주요 상태 갱신은 새 Vec/Map 구성 후 대입 패턴.

### PHASE 4
- Q8/Q9 외부 경계 deep copy: 명시 deep copy 정책은 별도 미구현(즉시 오류는 아님, 리스크는 낮음).

### PHASE 5
- Q10~Q12 혼재 여부:
  - 일부 action 함수가 프롬프트 구성 + 정책 판단 + 저장까지 함께 수행(복합도 높음).
  - 즉시 버그는 아니나 유지보수 비용 상승 요인.

### PHASE 6
- Q13 중복 함수: 명백한 중복 구현 다수는 이번 범위에서 미발견.
- Q14 COW 반복: 정상 범위.

### PHASE 7
- Q15 수정 항목: 코드 수정 없음(리뷰 문서 작성만 수행).
- Q16 요구사항 만족 여부: 불만족(위 Findings 1~3 해결 필요).
- Q17 scenario 일치 여부: 불일치(형식 미준수 항목 존재).

## Verification
- 실행 검증: `cargo test` (최근 실행 기준 21 passed, 0 failed).
