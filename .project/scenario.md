orc auto "react todo" -> src/cli.rs의 execute_cli("auto") 분기 진입
execute_cli("auto") -> auto_code_message("react todo") 호출
auto_code_message -> run_code_subcommand_in_new_session("init_code_project", ["-a", "react todo"])
init_code_project -> infer_from_message로 name/description/spec 추론
infer_from_message 결과 -> create_project_md_from_template 또는 load_code_project 실행
project.md 준비 완료 -> detail_code_project 실행
detail_code_project 완료 -> create_code_domain 실행
create_code_domain 완료 -> bootstrap_code_project 실행
bootstrap_code_project 완료 -> auto 메시지 흐름 종료
