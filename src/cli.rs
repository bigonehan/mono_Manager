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
        Some("help" | "cli_help" | "-h" | "--help")
    ) {
        return true;
    }
    if args.len() >= 3
        && super::profile::is_known_profile_name(args[1].as_str())
        && matches!(
            args.get(2).map(String::as_str),
            Some("help" | "cli_help" | "-h" | "--help")
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
        "help | cli_help | -h | --help",
        "clit <args...>  (deprecated; use check_orc_code and helper commands directly)",
        "init_orc_project|init_code_project [-n <name>] [-p <path>] [-s <spec>] [-d <description>] [-m <message>] [-a]",
        "build_orc_domains",
        "auto_add_function <message>",
        "init_orc_job",
        "add_orc_drafts",
        "create_job_md",
        "create_input_md",
        "cli_rust_orchestra",
        "impl_code_draft | cli_impl_code_draft",
        "check_orc_code",
        "open-ui [-w|--web|-b|--build]",
        "serve-web-api [--addr <host:port>]",
        "send-tmux <pane_id> <msg...> [enter|enter-exit|raw|display]",
        "capture-pane <pane_id> [lines]",
        "wait-ready <pane_id> <pattern> [timeout_ms] [lines]",
        "http-healthcheck <url> [timeout_ms]",
        "chat -n <name> [--background] [-m <message>] [-i <receiver_id>] [--data <data>]",
        "chat-wait -n <name> -a <true|false> [-c <count>]",
    ];
    commands.sort_unstable();

    for command in commands {
        println!("  {program} {command}");
    }
}

fn normalize_clit_args(args: &[String]) -> Vec<String> {
    if args.is_empty() {
        return Vec::new();
    }
    if matches!(args.first().map(String::as_str), Some("clit")) {
        return args.to_vec();
    }
    let mut normalized = Vec::with_capacity(args.len() + 1);
    normalized.push("clit".to_string());
    normalized.extend(args.iter().cloned());
    normalized
}

fn canonical_command_for_match(command: &str) -> &str {
    match command {
        "init_code_project" => "init_orc_project",
        "impl_code_draft" | "cli_impl_code_draft" => "impl_orc_code",
        "cli_create_input_md" => "create_input_md",
        other => other,
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
    let raw_command = args[command_idx].as_str();
    let command = canonical_command_for_match(raw_command);
    let tail = &args[(command_idx + 1)..];

    match command {
        "init_orc_project" => profile.project_service().init_project(tail),
        "build_orc_domains" => profile.project_service().build_domains(),
        "auto_add_function" => {
            if tail.is_empty() {
                return Err("auto_add_function requires <message>".to_string());
            }
            crate::code::auto_add_function(&tail.join(" ")).await
        }
        "init_orc_job" => profile.project_service().init_job(),
        "add_orc_drafts" => profile.draft_service().add_drafts(),
        "cli_rust_orchestra" => crate::code::flow_rust_orchestra(Path::new("."), tail),
        "impl_orc_code" => {
            if !tail.is_empty() {
                return Err(format!("{raw_command} does not accept arguments"));
            }
            profile.draft_service().run_parallel().await
        }
        "create_job_md" => {
            if !tail.is_empty() {
                return Err(format!("{raw_command} does not accept arguments"));
            }
            crate::code::create_job_md()
        }
        "create_input_md" => {
            if !tail.is_empty() {
                return Err(format!("{raw_command} does not accept arguments"));
            }
            crate::code::create_input_md()
        }
        "check_orc_code" => profile.feedback_service().check(),
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
        "capture-pane" => {
            if tail.is_empty() || tail.len() > 2 {
                return Err("capture-pane requires <pane_id> [lines]".to_string());
            }
            let pane_id = &tail[0];
            let lines = if tail.len() == 2 {
                tail[1]
                    .parse::<usize>()
                    .map_err(|_| "capture-pane: lines must be a positive integer".to_string())?
            } else {
                80
            };
            super::tmux::capture_pane_tail(pane_id, lines)
        }
        "wait-ready" => {
            if tail.len() < 2 || tail.len() > 4 {
                return Err(
                    "wait-ready requires <pane_id> <pattern> [timeout_ms] [lines]".to_string(),
                );
            }
            let pane_id = &tail[0];
            let pattern = &tail[1];
            let timeout_ms = if tail.len() >= 3 {
                tail[2]
                    .parse::<u64>()
                    .map_err(|_| "wait-ready: timeout_ms must be an integer".to_string())?
            } else {
                30_000
            };
            let lines = if tail.len() >= 4 {
                tail[3]
                    .parse::<usize>()
                    .map_err(|_| "wait-ready: lines must be a positive integer".to_string())?
            } else {
                120
            };
            super::tmux::wait_for_ready(pane_id, pattern, timeout_ms, lines)
        }
        "http-healthcheck" => {
            if tail.is_empty() || tail.len() > 2 {
                return Err("http-healthcheck requires <url> [timeout_ms]".to_string());
            }
            let timeout_ms = if tail.len() == 2 {
                tail[1]
                    .parse::<u64>()
                    .map_err(|_| "http-healthcheck: timeout_ms must be an integer".to_string())?
            } else {
                10_000
            };
            super::tmux::http_healthcheck(&tail[0], timeout_ms)
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
                return Err("clit is deprecated; use check_orc_code and helper commands directly".to_string());
            }
            let _normalized = normalize_clit_args(tail);
            Err("clit test was removed; use check_orc_code plus capture-pane/wait-ready/http-healthcheck helpers".to_string())
        }
        _ => Err(format!("unknown command: {}", command)),
    }
}

#[cfg(test)]
mod tests {
    use super::{canonical_command_for_match, is_help_command, normalize_clit_args};

    #[test]
    fn canonical_command_maps_impl_code_draft_alias() {
        assert_eq!(
            canonical_command_for_match("impl_code_draft"),
            "impl_orc_code"
        );
    }

    #[test]
    fn canonical_command_maps_init_code_project_alias() {
        assert_eq!(
            canonical_command_for_match("init_code_project"),
            "init_orc_project"
        );
    }

    #[test]
    fn canonical_command_maps_cli_impl_code_draft_alias() {
        assert_eq!(
            canonical_command_for_match("cli_impl_code_draft"),
            "impl_orc_code"
        );
    }

    #[test]
    fn canonical_command_keeps_other_commands() {
        assert_eq!(canonical_command_for_match("open-ui"), "open-ui");
    }

    #[test]
    fn canonical_command_keeps_auto_add_function() {
        assert_eq!(
            canonical_command_for_match("auto_add_function"),
            "auto_add_function"
        );
    }

    #[test]
    fn canonical_command_keeps_create_job_md() {
        assert_eq!(
            canonical_command_for_match("create_job_md"),
            "create_job_md"
        );
    }

    #[test]
    fn canonical_command_maps_cli_create_input_md_alias() {
        assert_eq!(
            canonical_command_for_match("cli_create_input_md"),
            "create_input_md"
        );
    }

    #[test]
    fn is_help_command_accepts_cli_help_alias() {
        let args = vec!["orc".to_string(), "cli_help".to_string()];
        assert!(is_help_command(&args));
    }

    #[test]
    fn normalize_clit_args_inserts_rc_subcommand_when_omitted() {
        let args = vec!["test".to_string(), "-p".to_string(), ".".to_string()];
        assert_eq!(
            normalize_clit_args(&args),
            vec![
                "clit".to_string(),
                "test".to_string(),
                "-p".to_string(),
                ".".to_string()
            ]
        );
    }

    #[test]
    fn normalize_clit_args_keeps_existing_clit_prefix() {
        let args = vec!["clit".to_string(), "test".to_string()];
        assert_eq!(normalize_clit_args(&args), args);
    }
}
