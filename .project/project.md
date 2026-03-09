# info
name : rust-orc
description : hello world 출력
spec : next js
path : /home/tree/project/rust-orc

# features
- rust_cli_workspace
- project_documentation
- cli_rust_orchestra
- cli_help
- cli_create_input_md
- cli_create_code_draft
- cli_impl_code_draft
- cli_test
- cli_check_task
- cli_check_draft
- cli_code
- cli_init_code_project
- cli_init_code_plan
- cli_add_code_plan
- cli_add_code_draft
- cli_add_code_draft_item
- cli_check_code_draft
- cli_open_ui
- cli_serve_web_api
- cli_auto
- cli_auto_add_function
- cli_send_tmux
- cli_enter
- cli_chat
- cli_chat_wait
# rules
- 프로젝트 내부의 공통 규칙

# constraints
- 프로젝트 내부의 공통 제약

# domains
## name
### states
- 가능한 상태
### action
- 가능한 동작
### rules
- 프로젝트 내부의 규칙 `##도메인 이름` 아래에 `-` 리스트 형식으로
  표현
### constraints
- 도메인이 지켜야 하는 제약