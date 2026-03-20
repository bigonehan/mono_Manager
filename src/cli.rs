use std::path::Path;

pub fn program_name(args: &[String]) -> &str {
    args.first()
        .and_then(|s| Path::new(s).file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("rust-orchestra")
}

pub fn is_help_command(args: &[String]) -> bool {
    if matches!(
        args.get(1).map(String::as_str),
        Some("help" | "-h" | "--help")
    ) {
        return true;
    }
    if args.len() >= 3
        && super::profile::is_known_profile_name(args[1].as_str())
        && matches!(
            args.get(2).map(String::as_str),
            Some("help" | "-h" | "--help")
        )
    {
        return true;
    }
    false
}

pub fn print_usage(program: &str) {
    println!("profiles: code (default)");
    println!("usage:");
    println!("  {program} [profile] <command> [args...]");
    let mut commands = [
        "help | -h | --help",
        "clit <args...>  (forward to rc CLI)",
        "init_orc_project [-n <name>] [-s <spec>] [-d <description>] [-a]",
        "build_orc_domains",
        "init_orc_job",
        "add_orc_drafts",
        "impl_orc_code",
        "check_orc_code",
        "open-ui [-w|--web|-b|--build]",
        "serve-web-api [--addr <host:port>]",
        "send-tmux <pane_id> <msg...> [enter|enter-exit|raw|display]",
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
        "init_orc_project" => profile.project_service().init_project(tail),
        "build_orc_domains" => profile.project_service().build_domains(),
        "init_orc_job" => profile.project_service().init_job(),
        "add_orc_drafts" => profile.draft_service().add_drafts(),
        "impl_orc_code" => {
            if !tail.is_empty() {
                return Err("impl_orc_code does not accept arguments".to_string());
            }
            profile.draft_service().run_parallel().await
        }
        "check_orc_code" => {
            profile.feedback_service().check()
        }
        "open-ui" => {
            if tail.is_empty() {
                super::tui::TuiRuntime::new().run_ui_entry()
            } else if tail.len() == 1 && matches!(tail[0].as_str(), "-w" | "--web") {
                super::web::open_web_ui()
            } else if tail.len() == 1 && matches!(tail[0].as_str(), "-b" | "--build") {
                super::web::open_web_ui_build()
            } else {
                Err("open-ui accepts no args or one of: -w, --web, -b, --build".to_string())
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
        "send-tmux" => {
            if tail.len() < 2 {
                return Err(
                    "send-tmux requires <pane_id> <msg...> [enter|enter-exit|raw|display]"
                        .to_string(),
                );
            }
            let pane_id = &tail[0];
            let (msg_slice, option) = match tail.last().map(String::as_str) {
                Some("enter" | "enter-exit" | "raw" | "display") if tail.len() >= 3 => {
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
            crate::chat_command(tail).await
        }
        "chat-wait" => {
            if tail.len() < 2 {
                return Err(
                    "chat-wait requires -n <name> -a <true|false> (optional: -c <count>)"
                        .to_string(),
                );
            }
            crate::chat_wait_command(tail).await
        }
        "clit" => {
            if tail.is_empty() {
                return Err(
                    "clit requires rc arguments (example: clit test -p <path> -m <mode>)"
                        .to_string(),
                );
            }
            crate::run_rc_forward(tail)
        }
        _ => Err(format!("unknown command: {}", command)),
    }
}
