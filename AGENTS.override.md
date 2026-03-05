# AGENTS Override

- 2026-03-05: `draft_item` 생성은 주석이 포함된 템플릿(`assets/code/templates/draft_item.yaml`)을 LLM 입력으로 사용해 의미를 추론한 뒤 값 채우기로 수행한다.
- 최종 산출물(`.project/drafts.yaml` item)에는 템플릿 주석/예시/placeholder를 포함하지 않는다.
- `draft_item` 관련 프롬프트는 "주석 읽기 -> 값 채우기 -> 주석 제거" 순서를 명시해야 한다.
- 2026-03-05: `if)` 가상 시나리오 출력은 줄 단위 `a -> b` 포맷만 사용한다. 각 단계는 반드시 다음 줄에 분리해서 작성한다.
- 2026-03-05: 사용자가 `~~~을 만들어줘` 형태로 요청하면 매니저 pane이 워커 pane을 단계별로 열고(`tmux split-window`), `orc send-tmux`로 `auto -> plan -> drafts -> impl -> check_draft`를 순차 위임/완료 회수/재시도 판단하는 흐름을 우선 적용한다.
- 2026-03-05: 트리거 문구는 `~~~을 만들어줘`, `~~~을 추가해줘`, `~을 읽고 처리해줘` 3가지를 동일 계열로 인식한다. 단, `읽고 처리해줘`는 기존 `input.md`를 읽는 명령 경로(`add_code_plan -f`, `add_code_draft -f`)를 사용하고 `create_input_md`를 호출하지 않는다.
