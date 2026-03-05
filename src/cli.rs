use std::path::Path;

pub fn program_name(args: &[String]) -> &str {
    args.first()
        .and_then(|s| Path::new(s).file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("rust-orchestra")
}

pub fn is_help_command(args: &[String]) -> bool {
    matches!(
        args.get(1).map(String::as_str),
        Some("help" | "-h" | "--help")
    )
}

pub fn print_usage(program: &str) {
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
        "open-ui",
        "auto <message> | auto -f",
        "send-tmux <pane_id> <msg...> [enter|raw]",
        "chat -n <name> [--background] [-m <message>] [-i <receiver_id>] [--data <data>]",
        "chat-wait -n <name> -a <true|false> [-c <count>]",
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
        "init_code_project" => super::code::init_code_project(&args[2..]),
        "init_code_plan" => super::code::init_code_plan(&args[2..]),
        "add_code_plan" => super::code::add_code_plan(&args[2..]),
        "create_input_md" => {
            if args.len() != 2 {
                return Err("create_input_md does not accept arguments".to_string());
            }
            super::code::create_input_md()
        }
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
        "open-ui" => super::tui::open_ui(),
        "auto" => {
            if args.len() >= 3 && args[2] == "-f" {
                if args.len() != 3 {
                    return Err("auto -f does not accept extra arguments".to_string());
                }
                return super::code::auto_code_from_input_file();
            }
            if args.len() < 3 || args[2].starts_with('-') {
                return Err("auto requires <message>".to_string());
            }
            super::code::auto_code_message(&args[2..].join(" "))
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
        _ => Err(format!("unknown command: {}", args[1])),
    }
}
