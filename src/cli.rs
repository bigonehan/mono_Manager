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
        "list-projects (alias: list)",
        "create-project <name> [path] [description]",
        "select-project <name> (alias: select)",
        "delete-project <name> (alias: delete)",
        "validate-tasks <feature_name>",
        "create-draft (alias: draft-create)",
        "add-plan [hint]",
        "add-draft <feature_name> [request] (alias: draft-add)",
        "delete-draft <feature_name> (alias: draft-delete)",
        "add-function [request] (alias: add-func)",
        "open-ui (alias: ui)",
        "run-auto [project_name]",
        "auto -d <description> -s <spec>",
        "auto-check",
        "auto-improve <request>",
        "draft-report",
        "send-tmux <pane_id> <msg...> [enter|raw] (alias: tsend)",
        "build-parallel-code",
        "build-parallel-todo",
        "run_parallel_test",
        "chat -n <name> [--background] [-m <message>] [-i <receiver_id>] [--data <data>]",
        "chat-wait -n <name> -a <true|false> [-c <count>]",
        "feedback",
        "build-function-auto (alias: build-todo-auto, build-functon-auto)",
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
        "list-projects" | "list" => super::list_projects(),
        "create-project" => {
            if args.len() < 3 {
                return Err("create-project requires <name> [path] [description]".to_string());
            }
            super::create_project(
                args[2].as_str(),
                args.get(3).map(String::as_str),
                args.get(4).map_or("", String::as_str),
            )
        }
        "select-project" | "select" => {
            if args.len() < 3 {
                return Err("select-project requires <name>".to_string());
            }
            super::select_project(&args[2])
        }
        "delete-project" | "delete" => {
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
        "create-draft" | "draft-create" => {
            if args.len() != 2 {
                return Err("create-draft does not accept arguments".to_string());
            }
            super::draft_create()
        }
        "add-plan" => {
            let request = if args.len() >= 3 {
                Some(args[2..].join(" "))
            } else {
                None
            };
            super::add_plan(request)
        }
        "add-draft" | "draft-add" => {
            if args.len() < 3 {
                return Err("add-draft requires <feature_name> [request]".to_string());
            }
            let request = if args.len() >= 4 {
                Some(args[3..].join(" "))
            } else {
                None
            };
            super::draft_add(&args[2], request)
        }
        "delete-draft" | "draft-delete" => {
            if args.len() < 3 {
                return Err("delete-draft requires <feature_name>".to_string());
            }
            super::draft_delete(&args[2])
        }
        "add-function" | "add-func" => {
            let request = if args.len() >= 3 {
                Some(args[2..].join(" "))
            } else {
                None
            };
            super::add_func(request)
        }
        "open-ui" | "ui" => super::ui(),
        "run-auto" => {
            let project_name = args.get(2).map(String::as_str);
            super::auto_mode(project_name)
        }
        "auto" => {
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
        "send-tmux" | "tsend" => {
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
        "build-parallel-todo" => {
            super::parallel::run_parallel_todo().await
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
        "build-function-auto" | "build-todo-auto" | "build-functon-auto" => {
            if args.len() != 2 {
                return Err("build-function-auto does not accept arguments".to_string());
            }
            super::build_function_auto().await
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
