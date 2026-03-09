# problem
- detail 화면에서 episode pane과 draft pane 배치가 요구사항과 다르고, draft 상단 액션 버튼 위치도 잘못되어 있다.
- episode 선택 상세 정보에 전체화면 Markdown 읽기 흐름이 없어 긴 내용을 읽기 어렵다.
- episode가 없을 때 drafts add/modify 버튼이 활성 상태이며, draft build 분량도 요구한 1500자 내외 규칙과 맞지 않다.
- tmux worker 경로의 `orc init_code_project -a`가 타임아웃되어 기존 프로젝트 기준 증분 workflow로 전환해야 한다.

# tasks
- 강제 실행: 초기화 단계는 건너뛰고 `orc add_code_plan -a`로 현재 요구사항을 기존 plan에 병합한다.
- 강제 실행: `orc create_input_md`와 `orc add_code_draft -f`로 plan/draft 산출물을 재생성한다.
- 강제 실행: 재시도에서는 초기화 단계 대신 증분 plan/draft 경로만 사용한다.
- web UI detail 레이아웃을 수정해 episode pane을 draft pane 위로 옮기고 add/modify/build 버튼을 draft pane 아래로 이동한다.
- episode 선택 정보 패널에 `읽기` 버튼과 전체화면 Markdown 뷰어를 추가한다.
- episode 미존재 시 drafts add/modify 비활성화를 연결하고, draft item build 텍스트 길이를 1500자 내외로 제한한다.
- 검증에서 CLI 산출물 갱신, UI 트리거, 내부 핸들러, 파일/상태 반영 경로를 함께 확인한다.

# check
- `orc add_code_plan -a`
- `orc create_input_md`
- `orc add_code_draft -f`
- `cargo test --manifest-path /home/tree/project/rust-orc/Cargo.toml`
- `cargo run --bin orc -- --help`
