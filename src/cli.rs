use std::env;
use std::path::Path;

pub fn calc_program_name(args: &[String]) -> &str {
    args.first()
        .and_then(|s| Path::new(s).file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("rust-orchestra")
}

pub fn calc_is_help_command(args: &[String]) -> bool {
    matches!(
        args.get(1).map(String::as_str),
        Some("help" | "-h" | "--help")
    )
}

pub fn print_usage(program: &str) {
    let mut commands = [
        "help | -h | --help",
        "plan-project [llm]",
        "detail-project [llm]",
        "detail-project -d <description> -s <spec> [--llm <bin>]",
        "init_code_project [-n <name>] [-p <path>] [-s <spec>] [-d <description>] [-a <message>]",
        "init_code_plan [-a]",
        "add_code_plan [-f] [-m <message>] [-a]",
        "create_code_draft",
        "add_code_draft [-f] [-m <message>]",
        "add_code_draft_item [-f] [-m <message>]",
        "impl_code_draft",
        "check_code_draft [-a]",
        "test",
        "check_task",
        "check_draft",
        "list-projects",
        "create-project [-n <name>] [-p <path>] [-s <spec>] [-d <description>]",
        "select-project <name>",
        "delete-project <name>",
        "validate-tasks <feature_name>",
        "add-function [request]",
        "activate-tui",
        "open-ui",
        "run-auto [project_name]",
        "auto <message>",
        "auto -d <description> -s <spec>",
        "auto-check",
        "auto-improve <request>",
        "draft-report",
        "send-tmux <pane_id> <msg...> [enter|raw]",
        "build-parallel-code",
        "run_parallel_test",
        "chat -n <name> [--background] [-m <message>] [-i <receiver_id>] [--data <data>]",
        "chat-wait -n <name> -a <true|false> [-c <count>]",
        "feedback",
        "press-key <key>",
    ];
    commands.sort_unstable();

    println!("usage:");
    for command in commands {
        println!("  {program} {command}");
    }
}

pub async fn execute_cli(args: &[String]) -> Result<String, String> {
    if args.len() < 2 {
        return Err("missing command".to_string());
    }

    match args[1].as_str() {
        "plan-project" => {
            let llm = args.get(2).map(String::as_str);
            super::plan_project(llm)
        }
        "init_code_project" => super::code::init_code_project(&args[2..]),
        "init_code_plan" => super::code::init_code_plan(&args[2..]),
        "add_code_plan" => super::code::add_code_plan(&args[2..]),
        "create_code_draft" => {
            if args.len() != 2 {
                return Err("create_code_draft does not accept arguments".to_string());
            }
            super::code::create_code_draft()
        }
        "add_code_draft" => super::code::add_code_draft(&args[2..]),
        "add_code_draft_item" => super::code::add_code_draft_item(&args[2..]),
        "impl_code_draft" => {
            if args.len() != 2 {
                return Err("impl_code_draft does not accept arguments".to_string());
            }
            super::code::impl_code_draft().await
        }
        "check_code_draft" => {
            let auto_yes = args.get(2).is_some_and(|v| v == "-a");
            super::code::check_code_draft(auto_yes)
        }
        "check_task" => {
            if args.len() != 2 {
                return Err("check_task does not accept arguments".to_string());
            }
            super::code::check_task()
        }
        "test" => {
            if args.len() != 2 {
                return Err("test does not accept arguments".to_string());
            }
            super::code::check_code_draft(false)
        }
        "check_draft" => {
            if args.len() != 2 {
                return Err("check_draft does not accept arguments".to_string());
            }
            super::code::check_draft()
        }
        "detail-project" => {
            let mut description: Option<&str> = None;
            let mut spec: Option<&str> = None;
            let mut llm: Option<&str> = None;
            let mut i = 2usize;
            while i < args.len() {
                match args[i].as_str() {
                    "-d" => {
                        i += 1;
                        description = args.get(i).map(String::as_str);
                    }
                    "-s" => {
                        i += 1;
                        spec = args.get(i).map(String::as_str);
                    }
                    "--llm" => {
                        i += 1;
                        llm = args.get(i).map(String::as_str);
                    }
                    other if !other.starts_with('-') && llm.is_none() => {
                        llm = Some(other);
                    }
                    _ => {}
                }
                i += 1;
            }
            if let (Some(d), Some(s)) = (description, spec) {
                super::detail_project_with_inputs(d, s, llm)
            } else {
                super::detail_project(llm)
            }
        }
        "list-projects" => super::list_projects(),
        "create-project" => {
            let mut name: Option<String> = None;
            let mut path: Option<String> = None;
            let mut description: Option<String> = None;
            let mut spec: Option<String> = None;
            let mut i = 2usize;
            while i < args.len() {
                match args[i].as_str() {
                    "-n" => {
                        i += 1;
                        if let Some(v) = args.get(i) {
                            name = Some(v.clone());
                        }
                    }
                    "-p" => {
                        i += 1;
                        if let Some(v) = args.get(i) {
                            path = Some(v.clone());
                        }
                    }
                    "-d" => {
                        i += 1;
                        if let Some(v) = args.get(i) {
                            description = Some(v.clone());
                        }
                    }
                    "-s" => {
                        i += 1;
                        if let Some(v) = args.get(i) {
                            spec = Some(v.clone());
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            let default_name = env::current_dir()
                .ok()
                .and_then(|p| p.file_name().map(|v| v.to_string_lossy().to_string()))
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| "project".to_string());
            let name = name.unwrap_or(default_name);
            let description = description
                .unwrap_or_else(|| "heolloworld를 출력하는 간단한 web app으로".to_string());
            let spec = spec.unwrap_or_else(|| "nextjs".to_string());
            super::create_project(
                name.as_str(),
                path.as_deref(),
                description.as_str(),
                spec.as_str(),
            )
        }
        "select-project" => {
            if args.len() < 3 {
                return Err("select-project requires <name>".to_string());
            }
            super::select_project(&args[2])
        }
        "delete-project" => {
            if args.len() < 3 {
                return Err("delete-project requires <name>".to_string());
            }
            super::delete_project(&args[2])
        }
        "validate-tasks" => {
            if args.len() < 3 {
                return Err("validate-tasks requires <feature_name>".to_string());
            }
            super::validate_tasks(&args[2])
        }
        "add-function" => {
            let request = if args.len() >= 3 {
                Some(args[2..].join(" "))
            } else {
                None
            };
            super::add_func(request).await
        }
        "activate-tui" => super::tui::activate_tui(),
        "open-ui" => super::tui::open_ui(),
        "run-auto" => {
            let project_name = args.get(2).map(String::as_str);
            super::auto_mode(project_name)
        }
        "auto" => {
            if args.len() >= 3 && !args[2].starts_with('-') {
                return super::code::auto_code_message(&args[2..].join(" "));
            }
            let mut description: Option<&str> = None;
            let mut spec: Option<&str> = None;
            let mut i = 2usize;
            while i < args.len() {
                match args[i].as_str() {
                    "-d" => {
                        i += 1;
                        description = args.get(i).map(String::as_str);
                    }
                    "-s" => {
                        i += 1;
                        spec = args.get(i).map(String::as_str);
                    }
                    _ => {}
                }
                i += 1;
            }
            if let (Some(d), Some(s)) = (description, spec) {
                super::auto_bootstrap(d, s)
            } else {
                let project_name = args.get(2).map(String::as_str);
                super::auto_mode(project_name)
            }
        }
        "auto-check" => {
            if args.len() != 2 {
                return Err("auto-check does not accept arguments".to_string());
            }
            super::auto_check()
        }
        "auto-improve" => {
            if args.len() < 3 {
                return Err("auto-improve requires <request>".to_string());
            }
            super::auto_improve(&args[2..].join(" "))
        }
        "draft-report" => {
            if args.len() != 2 {
                return Err("draft-report does not accept arguments".to_string());
            }
            super::draft_report()
        }
        "send-tmux" => {
            if args.len() < 4 {
                return Err("send-tmux requires <pane_id> <msg...> [enter|raw]".to_string());
            }
            let pane_id = &args[2];
            let (msg_slice, option) = match args.last().map(String::as_str) {
                Some("enter" | "raw") if args.len() >= 5 => (&args[3..args.len() - 1], args[args.len() - 1].as_str()),
                _ => (&args[3..], "enter"),
            };
            if msg_slice.is_empty() {
                return Err("send-tmux requires non-empty message".to_string());
            }
            let msg = msg_slice.join(" ");
            super::tmux::tsend(pane_id, &msg, option)
        }
        "build-parallel-code" => {
            super::parallel::run_parallel_build_code().await
        }
        "run_parallel_test" => {
            if args.len() != 2 {
                return Err("run_parallel_test does not accept arguments".to_string());
            }
            super::run_parallel_test().await
        }
        "chat" => {
            if args.len() < 4 {
                return Err(
                    "chat requires -n <name> (optional: --background | -m <message> -i <receiver_id> --data <data>)"
                        .to_string(),
                );
            }
            super::chat_command(&args[2..]).await
        }
        "chat-wait" => {
            if args.len() < 4 {
                return Err("chat-wait requires -n <name> -a <true|false> (optional: -c <count>)".to_string());
            }
            super::chat_wait_command(&args[2..]).await
        }
        "feedback" => {
            if args.len() != 2 {
                return Err("feedback does not accept arguments".to_string());
            }
            super::run_feedback()
        }
        "press-key" => {
            if args.len() < 3 {
                return Err("press-key requires <key>".to_string());
            }
            super::parallel::press_key(&args[2]).await
        }
        _ => Err(format!("unknown command: {}", args[1])),
    }
}
