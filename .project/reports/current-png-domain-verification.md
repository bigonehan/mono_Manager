# ORC Worker Report

## task
- current.png 기준 mono detail domains 미표시 원인 점검
- 대상 프로젝트: app/web

## plan
1. current.png 대상 프로젝트와 실제 registry 선택 상태를 고정한다.
2. project detail API, project.md, packages/domains 실데이터를 비교한다.
3. 화면 실패 조건과 데이터 부족/연결 실패 중 어느 쪽인지 판정한다.
4. 후속 개선이 필요하면 개선 후보를 적는다.

## findings
- registry project_type: missing
- registry path: missing
- project.md has # domains: False
- packages/domains entries: 0
- packages/domains names: []

## conclusion
- 현재 app/web에 표시할 domain 데이터가 없다.
- current.png의 '(none)'은 현재 데이터와 일치한다.
- 이전 테스트 주입 기반 확인은 현재 워크스페이스 판정 근거로 사용할 수 없다.

## improvement_candidates
- current.png 기반 검증에서는 테스트 주입 데이터와 실제 workspace 데이터를 분리해 체크리스트를 먼저 고정한다.
- mono domains pane 완료 조건에 '실제 project.md #domains 또는 packages/domains 엔트리 존재 확인'을 추가한다.
