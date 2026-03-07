# AGENTS Override

- 2026-03-06: profile 레이어 리팩토링 작업 시 별도 브랜치에서 진행한다.
- 2026-03-06: profile 종속 범위는 prompt/template 로딩(project.md, plan.yaml, drafts.yaml, parallel run) 및 해당 호출 프롬프트로 한정한다.
- 2026-03-06: 공통 인터페이스는 project/plan/draft/feedback 흐름을 우선 제공하고, 기본 구현체는 code profile로 유지한다.
- 2026-03-07: 모든 작업 완료 시 `nf -m "<task-name> complete"` 실행을 강제한다. 별도 요청이 없어도 필수로 실행하며 `notify.fish` 직접 호출은 금지한다.
- 2026-03-07: 사용자가 `/temp` 검증 루프를 요청하면 `/home/tree/temp`를 삭제/재생성 후 `orc auto`를 실행하고, 실패 시 `/home/tree/temp/todo.md`와 `/home/tree/temp/feedback.md`를 작성한 뒤 plan 갱신 후 재시도한다.
- 2026-03-07: 사용자가 web UI 확장(`open-ui -w`, assets 기반 frontend, playwright 검증 루프)을 요청한 경우, TUI 기능 목록을 먼저 추출해 `input.md`에 반영한 뒤 구현/검증을 반복한다.
- 2026-03-07: web UI는 `project`/`detail` 탭 분리 구조를 유지하고, detail 편집은 pane 선택 시 우상단 gear 아이콘으로 진입하는 읽기전용 기본 화면으로 제공한다.
- 2026-03-07: web UI 상태는 로컬 useState보다 `zustand` 스토어를 우선 사용해 탭/선택/편집/로그를 중앙 관리한다.
- 2026-03-07: project pane의 선택 item 우상단에 수정/삭제 아이콘 버튼(SVG)을 노출하고, 해당 item 단위 edit/delete 동작을 제공한다.
- 2026-03-07: UI 스타일 변경 요청 시 `current.png`를 기준으로 project info 시각톤을 맞추고, 모든 pane 컨테이너에 rounded border를 강제 적용한다.
- 2026-03-07: project 탭은 grid 카드 + 생성 모달 구조를 기본으로 하며, 카드에 type 라벨/상태 태그/폴더 아이콘/큰 제목을 표시하고 `project_type(story|movie|code|mono)` 필드를 기본 `code`로 유지한다.
- 2026-03-07: web navbar는 좌측에 현재 선택 프로젝트 표시, 우측에 project/detail 탭 버튼을 두고, 카드형 border 대신 하단 underline(border-b)만 사용한다.
