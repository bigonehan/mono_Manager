# plan
- add_orc_drafts 파서 실패 원인을 자연어 출력 경로로 고정한다.
- draft_item 생성 프롬프트를 YAML 단일 item 출력으로 제한한다.
- add_orc_drafts 단계에서 단일 item/list 파싱과 1회 정규화 재시도를 적용한다.
- ORC 체인(add_orc_drafts -> impl_orc_code -> check_orc_code -> clit test)을 timeout 180s, 단계별 최대 2회 재시도로 검증한다.

# requirement
## rust_cli_workspace
## project_documentation
## cli_rust_orchestra
## cli_help
## cli_create_input_md
## cli_create_code_draft
## cli_impl_code_draft
## cli_test
## cli_check_task
## cli_check_draft
## cli_code
## cli_init_code_project
## cli_init_code_plan
## cli_add_code_plan
## cli_add_code_draft
## cli_add_code_draft_item
## cli_check_code_draft
## cli_open_ui
## cli_serve_web_api
## cli_auto
## cli_auto_add_function
## cli_send_tmux
## cli_enter
## cli_chat
## cli_chat_wait
## cli_clit
## cli_report_long_running_status
## cli_track_check_process
## web_detail_check_pane
## web_manual_rc_check
## web_screenshot_feedback
## web_voice_input_fields
## cli_reset_task_artifacts
## web_project_feedback_path
## web_rc_binary_resolver
## web_test_voice_helper

# task
## planned
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
- cli_clit
- cli_report_long_running_status
- cli_track_check_process
- web_detail_check_pane
- web_manual_rc_check
- web_screenshot_feedback
- web_voice_input_fields
- cli_reset_task_artifacts
- web_project_feedback_path
- web_rc_binary_resolver
- web_test_voice_helper
## work
## check
## completed
## fail

# problems
- add_orc_drafts rust_cli_workspace : codex exec [너는 ORC draft_item 생성기다.] attempt 2/2 timed out after 10s
- add_orc_drafts project_documentation : codex exec [너는 ORC draft_item 생성기다.] attempt 2/2 timed out after 10s
- add_orc_drafts cli_rust_orchestra : codex exec [너는 ORC draft_item 생성기다.] attempt 2/2 timed out after 10s
- add_orc_drafts cli_help : codex exec [너는 ORC draft_item 생성기다.] attempt 2/2 timed out after 10s
- add_orc_drafts cli_create_input_md : codex exec [너는 ORC draft_item 생성기다.] attempt 2/2 timed out after 10s
- add_orc_drafts cli_create_code_draft : codex exec [너는 ORC draft_item 생성기다.] attempt 2/2 timed out after 10s
- add_orc_drafts cli_impl_code_draft : codex exec [너는 ORC draft_item 생성기다.] attempt 2/2 timed out after 10s
- add_orc_drafts cli_test : codex exec [너는 ORC draft_item 생성기다.] attempt 2/2 timed out after 10s
