# 결과
- rust-orc 템플릿 경로 수정 완료

# 미해결
- 없음

# 보완
- 설치된 orc 바이너리 재설치 후 워크플로 재검증 진행

## 2026-03-10 retry
### 문제
- `orc init_code_plan -a` 실행 시 `.project/plan.yaml` 기존 파일 때문에 "init_code_plan can run only once" 오류가 발생했다.

### 미해결점
- 기존 프로젝트 상태를 유지하면서 새 요구사항을 plan/draft에 반영해야 한다.

### 재시도 전략
- `orc add_code_plan -a`로 기존 plan에 기능 요구사항을 추가하고 다음 단계를 계속 수행한다.

## 2026-03-10 retry-2
### 문제
- `orc add_code_draft -a`가 제한 시간 내 완료되지 않았고, `.project/drafts.yaml`은 `draft: []` 상태로 남았다.

### 미해결점
- 자동 경로에서 draft 산출물이 확정되지 않아 구현 단계(`impl_code_draft`)로 진행할 수 없다.

### 재시도 전략
- 자동 경로 대신 파일 기반 `orc add_code_draft -f`를 실행해 `input.md`를 직접 사용한다.
- 필요 시 `orc create_code_draft`로 단일 draft 생성 경로를 시도한다.

## 2026-03-10 retry-3
### 문제
- `orc create_code_draft`도 무출력 상태로 타임아웃되어 draft 생성이 진행되지 않았다.

### 미해결점
- orc draft 생성 단계가 현재 환경에서 블로킹되어 workflow 완주가 불가능하다.

### 재시도 전략
- 동일 명령 반복 대신 수동 코드 변경으로 기능을 구현하고, 검증 단계(`cargo test`, `orc check_code_draft -a`)만 orc 명령으로 수행한다.

## 2026-03-10 retry-4
### 문제
- tmux worker 경로로 실행한 `orc init_code_project -a "episode pane 위치 조정, 읽기 뷰어 추가, draft 버튼 비활성화, build 분량 1500자"`가 2분 이상 종료되지 않아 workflow 시작 단계가 멈췄다.

### 미해결점
- 기존 `.project/project.md`를 유지한 채 새 UI 요구사항만 plan/draft 단계에 반영해야 한다.

### 재시도 전략
- 초기화 단계는 건너뛰고 기존 프로젝트 상태를 사용한다.
- `orc add_code_plan -a`로 새 요구사항을 plan에 병합한 뒤 `orc create_input_md`와 `orc add_code_draft -f`를 순서대로 재실행한다.
