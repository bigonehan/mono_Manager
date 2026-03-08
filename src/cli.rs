use std::path::Path;

pub fn program_name(args: &[String]) -> &str {
    args.first()
        .and_then(|s| Path::new(s).file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("rust-orchestra")
}

pub fn is_help_command(args: &[String]) -> bool {
    if matches!(args.get(1).map(String::as_str), Some("help" | "-h" | "--help")) {
        return true;
    }
    if args.len() >= 3
        && super::profile::is_known_profile_name(args[1].as_str())
        && matches!(args.get(2).map(String::as_str), Some("help" | "-h" | "--help"))
    {
        return true;
    }
    false
}

pub fn print_usage(program: &str) {
    println!("profiles: code (default), story, write (planned), movie (planned)");
    println!("usage:");
    println!("  {program} [profile] <command> [args...]");
    let mut commands = [
        "help | -h | --help",
        "init_code_project [-n <name>] [-p <path>] [-s <spec>] [-d <description>] [-a <message>]",
        "init_code_plan [-a]",
        "add_code_plan [-f] [-m <message>] [-a]",
        "create_input_md",
        "create_code_draft",
        "add_code_draft [-f] [-m <message>] [-a]",
        "add_code_draft_item [-f] [-m <message>] [-a]",
        "impl_code_draft",
        "check_code_draft [-a]",
        "test",
        "check_task",
        "check_draft",
        "open-ui [-w|--web]",
        "serve-web-api [--addr <host:port>]",
        "auto <message> | auto -f",
        "send-tmux <pane_id> <msg...> [enter|raw]",
        "chat -n <name> [--background] [-m <message>] [-i <receiver_id>] [--data <data>]",
        "chat-wait -n <name> -a <true|false> [-c <count>]",
    ];
    commands.sort_unstable();

    for command in commands {
        println!("  {program} {command}");
    }
}

fn resolve_default_profile_name() -> String {
    super::load_app_config()
        .as_ref()
        .map_or("code".to_string(), |cfg| {
            cfg.default_profile_name().to_string()
        })
}

fn resolve_profile_and_command_index(args: &[String]) -> (String, usize) {
    if args.len() >= 3 && super::profile::is_known_profile_name(args[1].as_str()) {
        return (args[1].clone(), 2);
    }
    (resolve_default_profile_name(), 1)
}

pub async fn execute_cli(args: &[String]) -> Result<String, String> {
    if args.len() < 2 {
        return Err("missing command".to_string());
    }
    let (profile_name, command_idx) = resolve_profile_and_command_index(args);
    if args.len() <= command_idx {
        return Err("missing command".to_string());
    }
    let profile = super::profile::resolve_profile(&profile_name)?;
    let command = args[command_idx].as_str();
    let tail = &args[(command_idx + 1)..];

    match command {
        "init_code_project" => profile.project_service().create(tail),
        "init_code_plan" => profile.plan_service().create(tail),
        "add_code_plan" => profile.plan_service().add_feature(tail),
        "create_input_md" => {
            if !tail.is_empty() {
                return Err("create_input_md does not accept arguments".to_string());
            }
            profile.plan_service().create_input()
        }
        "create_code_draft" => {
            if !tail.is_empty() {
                return Err("create_code_draft does not accept arguments".to_string());
            }
            profile.plan_service().create_draft()
        }
        "add_code_draft" => profile.draft_service().add(tail),
        "add_code_draft_item" => profile.draft_service().move_item_to_drafts_yaml(tail),
        "impl_code_draft" => {
            if !tail.is_empty() {
                return Err("impl_code_draft does not accept arguments".to_string());
            }
            profile.draft_service().run_parallel().await
        }
        "check_code_draft" => {
            let auto_yes = tail.first().is_some_and(|v| v == "-a");
            profile.feedback_service().check(auto_yes)
        }
        "check_task" => {
            if !tail.is_empty() {
                return Err("check_task does not accept arguments".to_string());
            }
            profile.feedback_service().decide_policy()
        }
        "test" => {
            if !tail.is_empty() {
                return Err("test does not accept arguments".to_string());
            }
            profile.feedback_service().check(false)
        }
        "check_draft" => {
            if !tail.is_empty() {
                return Err("check_draft does not accept arguments".to_string());
            }
            profile.feedback_service().check_draft()
        }
        "open-ui" => {
            if tail.is_empty() {
                super::tui::open_ui()
            } else if tail.len() == 1 && matches!(tail[0].as_str(), "-w" | "--web") {
                super::web::open_web_ui()
            } else {
                Err("open-ui accepts no args or one of: -w, --web".to_string())
            }
        }
        "serve-web-api" => {
            let mut addr = "127.0.0.1:7788".to_string();
            let mut i = 0usize;
            while i < tail.len() {
                match tail[i].as_str() {
                    "--addr" => {
                        if i + 1 >= tail.len() {
                            return Err("serve-web-api: --addr requires value".to_string());
                        }
                        addr = tail[i + 1].clone();
                        i += 2;
                    }
                    other => {
                        return Err(format!("serve-web-api: unknown arg {}", other));
                    }
                }
            }
            super::web_api::serve_web_api(&addr).await
        }
        "auto" => {
            if tail.first().is_some_and(|v| v == "-f") {
                if tail.len() != 1 {
                    return Err("auto -f does not accept extra arguments".to_string());
                }
                return profile.project_service().auto_from_input();
            }
            if tail.is_empty() || tail[0].starts_with('-') {
                return Err("auto requires <message>".to_string());
            }
            profile.project_service().auto_message(&tail.join(" "))
        }
        "send-tmux" => {
            if tail.len() < 2 {
                return Err("send-tmux requires <pane_id> <msg...> [enter|raw]".to_string());
            }
            let pane_id = &tail[0];
            let (msg_slice, option) = match tail.last().map(String::as_str) {
                Some("enter" | "raw") if tail.len() >= 3 => {
                    (&tail[1..tail.len() - 1], tail[tail.len() - 1].as_str())
                }
                _ => (&tail[1..], "enter"),
            };
            if msg_slice.is_empty() {
                return Err("send-tmux requires non-empty message".to_string());
            }
            let msg = msg_slice.join(" ");
            super::tmux::tsend(pane_id, &msg, option)
        }
        "chat" => {
            if tail.len() < 2 {
                return Err(
                    "chat requires -n <name> (optional: --background | -m <message> -i <receiver_id> --data <data>)"
                        .to_string(),
                );
            }
            super::chat_command(tail).await
        }
        "chat-wait" => {
            if tail.len() < 2 {
                return Err("chat-wait requires -n <name> -a <true|false> (optional: -c <count>)".to_string());
            }
            super::chat_wait_command(tail).await
        }
        _ => Err(format!("unknown command: {}", command)),
    }
}
