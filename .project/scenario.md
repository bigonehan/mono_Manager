orc auto "react todo" | src/cli.rs | execute_cli("auto") 분기 진입
execute_cli("auto") | src/code.rs | auto_code_message("react todo") 호출
auto_code_message | src/code.rs | run_code_subcommand_in_new_session("init_code_project", ["-a", "react todo"]) 실행
init_code_project | src/code.rs | infer_from_message로 name/description/spec 추론
infer_from_message 결과 | .project/project.md | create_project_md_from_template 또는 load_code_project 실행
project.md 준비 완료 | src/code.rs | detail_code_project 실행
detail_code_project 완료 | src/main.rs | create_code_domain 실행
create_code_domain 완료 | src/main.rs | bootstrap_code_project 실행
bootstrap_code_project 완료 | workspace artifacts | auto 메시지 흐름 종료
