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
    println!("usage:");
    println!("  {program} plan-project [llm]");
    println!("  {program} detail-project [llm]");
    println!("  {program} detail-project -d <description> -s <spec> [--llm <bin>]");
    println!("  {program} list-projects");
    println!("  {program} create-project <name> [path] [description]");
    println!("  {program} select-project <name>");
    println!("  {program} delete-project <name>");
    println!("  {program} validate-tasks <feature_name>");
    println!("  {program} create-draft");
    println!("  {program} add-plan [hint]");
    println!("  {program} add-draft <feature_name> [request]");
    println!("  {program} delete-draft <feature_name>");
    println!("  {program} add-function");
    println!("  {program} open-ui");
    println!("  {program} run-auto [project_name]");
    println!("  {program} auto -d <description> -s <spec>");
    println!("  {program} auto-check");
    println!("  {program} auto-improve <request>");
    println!("  {program} draft-report");
    println!("  {program} send-tmux <pane_id> <msg...> [enter|raw]");
    println!("  {program} build-parallel-code");
    println!("  {program} press-key <key>");
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
        "press-key" => {
            if args.len() < 3 {
                return Err("press-key requires <key>".to_string());
            }
            super::parallel::press_key(&args[2]).await
        }
        _ => Err(format!("unknown command: {}", args[1])),
    }
}
