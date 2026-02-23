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

pub fn flow_print_usage(program: &str) {
    println!("usage:");
    println!("  {program} plan-project [llm]");
    println!("  {program} plan-init [-n name] [-d description] [-s spec] [--llm llm]");
    println!("  {program} detail-project [llm]");
    println!("  {program} list-projects");
    println!("  {program} create-project <name> [path] [description]");
    println!("  {program} add-project <name> <path> [description]");
    println!("  {program} select-project <name>");
    println!("  {program} delete-project <name>");
    println!("  {program} validate-tasks <feature_name>");
    println!("  {program} create-draft");
    println!("  {program} add-draft <feature_name> [request]");
    println!("  {program} delete-draft <feature_name>");
    println!("  {program} add-function");
    println!("  {program} open-ui");
    println!("  {program} run-auto [project_name]");
    println!("  {program} send-tmux <pane_id> <msg...> [enter|raw]");
    println!("  {program} build-parallel-code");
    println!("  {program} press-key <key>");
}

pub async fn flow_execute_cli(args: &[String]) -> Result<String, String> {
    if args.len() < 2 {
        return Err("missing command".to_string());
    }

    match args[1].as_str() {
        "plan-project" => {
            let llm = args.get(2).map(String::as_str);
            super::flow_plan_project(llm)
        }
        "plan-init" => {
            let mut name: Option<&str> = None;
            let mut description: Option<&str> = None;
            let mut spec: Option<&str> = None;
            let mut llm: Option<&str> = None;
            let mut i = 2usize;
            while i < args.len() {
                match args[i].as_str() {
                    "-n" | "--name" => {
                        i += 1;
                        if i >= args.len() {
                            return Err("plan-init: -n/--name requires value".to_string());
                        }
                        name = Some(args[i].as_str());
                    }
                    "-d" | "--description" => {
                        i += 1;
                        if i >= args.len() {
                            return Err("plan-init: -d/--description requires value".to_string());
                        }
                        description = Some(args[i].as_str());
                    }
                    "-s" | "--spec" => {
                        i += 1;
                        if i >= args.len() {
                            return Err("plan-init: -s/--spec requires value".to_string());
                        }
                        spec = Some(args[i].as_str());
                    }
                    "--llm" => {
                        i += 1;
                        if i >= args.len() {
                            return Err("plan-init: --llm requires value".to_string());
                        }
                        llm = Some(args[i].as_str());
                    }
                    unknown => {
                        return Err(format!("plan-init: unknown arg `{}`", unknown));
                    }
                }
                i += 1;
            }
            super::flow_plan_init(name, description, spec, llm)
        }
        "detail-project" => {
            let llm = args.get(2).map(String::as_str);
            super::flow_detail_project(llm)
        }
        "list-projects" | "list" => super::flow_list_projects(),
        "create-project" | "create" => {
            if args.len() < 3 {
                return Err("create-project requires <name>".to_string());
            }
            let name = &args[2];
            let path = args.get(3).map(|s| s.as_str());
            let description = args.get(4).map_or("", String::as_str);
            super::flow_create_project(name, path, description)
        }
        "add-project" | "add" => {
            if args.len() < 4 {
                return Err("add-project requires <name> <path>".to_string());
            }
            let description = args.get(4).map_or("", String::as_str);
            super::flow_add_project(&args[2], &args[3], description)
        }
        "select-project" | "select" => {
            if args.len() < 3 {
                return Err("select-project requires <name>".to_string());
            }
            super::flow_select_project(&args[2])
        }
        "delete-project" | "delete" => {
            if args.len() < 3 {
                return Err("delete-project requires <name>".to_string());
            }
            super::flow_delete_project(&args[2])
        }
        "validate-tasks" => {
            if args.len() < 3 {
                return Err("validate-tasks requires <feature_name>".to_string());
            }
            super::flow_validate_tasks(&args[2])
        }
        "create-draft" | "draft-create" => {
            if args.len() != 2 {
                return Err("create-draft does not accept arguments".to_string());
            }
            super::flow_draft_create()
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
            super::flow_draft_add(&args[2], request)
        }
        "delete-draft" | "draft-delete" => {
            if args.len() < 3 {
                return Err("delete-draft requires <feature_name>".to_string());
            }
            super::flow_draft_delete(&args[2])
        }
        "add-function" | "add-func" => {
            let request = if args.len() >= 3 {
                Some(args[2..].join(" "))
            } else {
                None
            };
            super::flow_add_func(request)
        }
        "open-ui" | "ui" => super::flow_ui(),
        "run-auto" | "auto" => {
            let project_name = args.get(2).map(String::as_str);
            super::flow_auto_mode(project_name)
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
            super::flow_tsend(pane_id, &msg, option)
        }
        "build-parallel-code" => {
            super::flow_run_parallel_build_code().await
        }
        "press-key" => {
            if args.len() < 3 {
                return Err("press-key requires <key>".to_string());
            }
            super::flow_press_key(&args[2]).await
        }
        _ => Err(format!("unknown command: {}", args[1])),
    }
}
