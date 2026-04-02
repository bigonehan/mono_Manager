use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{
    fs,
    fs::OpenOptions,
    io::{self, IsTerminal, Read, Write},
};

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
        "worker-create",
        "worker-send <worker_ref|pane_id> <msg...>|--stdin [enter|enter-exit|raw|display]",
        "worker-wait <worker_ref|pane_id> <pattern> [timeout_ms] [lines]",
        "worker-close <worker_ref|pane_id>",
        "worker-dev-url <worker_ref|pane_id> [lines]",
        "manager-trace <stage> [detail...]",
        "check-manager-trace [preflight|impl|check|final]",
        "send-tmux <pane_id> <msg...>|--stdin [enter|enter-exit|raw|display]",
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

fn normalize_stdin_send_message(command_name: &str, mut buffer: String) -> Result<String, String> {
    if let Some(stripped) = buffer.strip_suffix("\r\n") {
        buffer = stripped.to_string();
    } else if let Some(stripped) = buffer.strip_suffix('\n') {
        buffer = stripped.to_string();
    }
    if buffer.is_empty() {
        return Err(format!("{command_name} requires non-empty message"));
    }
    Ok(buffer)
}

fn read_send_message_from_stdin(command_name: &str) -> Result<String, String> {
    let mut stdin = io::stdin();
    if stdin.is_terminal() {
        return Err(format!("{command_name} --stdin requires piped stdin"));
    }
    let mut buffer = String::new();
    stdin
        .read_to_string(&mut buffer)
        .map_err(|e| format!("{command_name} failed to read stdin: {e}"))?;
    normalize_stdin_send_message(command_name, buffer)
}

fn canonical_command_for_match(command: &str) -> &str {
    match command {
        "init_code_project" => "init_orc_project",
        "impl_code_draft" | "cli_impl_code_draft" => "impl_orc_code",
        "cli_create_input_md" => "create_input_md",
        "orc_manager_trace" => "manager-trace",
        "check_orc_manager_trace" => "check-manager-trace",
        other => other,
    }
}

const ORC_MANAGER_TRACE_FILE: &str = ".project/orc_manager_trace.log";
const CHECK_PROCESS_FILE: &str = ".project/check-process.md";

fn supported_manager_trace_stage(stage: &str) -> bool {
    matches!(
        stage,
        "stage_global_override_read"
            | "stage_job_md_locked"
            | "stage_plan_done"
            | "stage_impl_session_started"
            | "stage_impl_done"
            | "stage_check_session_started"
            | "stage_check_done"
            | "stage_manager_reverified"
    )
}

fn read_orc_manager_trace_lines() -> Result<Vec<String>, String> {
    fs::read_to_string(ORC_MANAGER_TRACE_FILE)
        .map(|content| content.lines().map(|line| line.to_string()).collect())
        .map_err(|_| format!("ERROR: orc_manager trace file missing: {ORC_MANAGER_TRACE_FILE}"))
}

fn find_stage_line(lines: &[String], stage: &str) -> Result<usize, String> {
    lines.iter()
        .position(|line| line.contains(stage))
        .map(|idx| idx + 1)
        .ok_or_else(|| format!("ERROR: missing required orc_manager trace: {stage}"))
}

fn assert_trace_lt(lines: &[String], left: &str, right: &str) -> Result<(), String> {
    let left_line = find_stage_line(lines, left)?;
    let right_line = find_stage_line(lines, right)?;
    if left_line >= right_line {
        return Err(format!(
            "ERROR: invalid orc_manager trace order: {left} should precede {right}"
        ));
    }
    Ok(())
}

fn append_orc_manager_trace(stage: &str, detail: &[String]) -> Result<String, String> {
    if !supported_manager_trace_stage(stage) {
        return Err(format!("ERROR: unsupported orc_manager stage: {stage}"));
    }
    fs::create_dir_all(".project").map_err(|e| e.to_string())?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let detail_text = detail.join(" ");
    let mut line = format!("[{ts}] {stage}");
    if !detail_text.is_empty() {
        line.push_str(" | ");
        line.push_str(&detail_text);
    }

    let mut trace = OpenOptions::new()
        .create(true)
        .append(true)
        .open(ORC_MANAGER_TRACE_FILE)
        .map_err(|e| e.to_string())?;
    writeln!(trace, "{line}").map_err(|e| e.to_string())?;

    let mut check_process = OpenOptions::new()
        .create(true)
        .append(true)
        .open(CHECK_PROCESS_FILE)
        .map_err(|e| e.to_string())?;
    let check_line = if detail_text.is_empty() {
        format!("- [{ts}] {stage}")
    } else {
        format!("- [{ts}] {stage} | {detail_text}")
    };
    writeln!(check_process, "{check_line}").map_err(|e| e.to_string())?;
    Ok(line)
}

fn check_orc_manager_trace(mode: &str) -> Result<String, String> {
    let lines = read_orc_manager_trace_lines()?;
    match mode {
        "preflight" | "impl" => {
            find_stage_line(&lines, "stage_global_override_read")?;
            find_stage_line(&lines, "stage_job_md_locked")?;
            find_stage_line(&lines, "stage_plan_done")?;
            assert_trace_lt(&lines, "stage_global_override_read", "stage_job_md_locked")?;
            assert_trace_lt(&lines, "stage_job_md_locked", "stage_plan_done")?;
        }
        "check" => {
            find_stage_line(&lines, "stage_impl_session_started")?;
            find_stage_line(&lines, "stage_impl_done")?;
            assert_trace_lt(&lines, "stage_global_override_read", "stage_job_md_locked")?;
            assert_trace_lt(&lines, "stage_job_md_locked", "stage_plan_done")?;
            assert_trace_lt(&lines, "stage_plan_done", "stage_impl_session_started")?;
            assert_trace_lt(&lines, "stage_impl_session_started", "stage_impl_done")?;
        }
        "final" => {
            find_stage_line(&lines, "stage_impl_session_started")?;
            find_stage_line(&lines, "stage_impl_done")?;
            find_stage_line(&lines, "stage_check_session_started")?;
            find_stage_line(&lines, "stage_check_done")?;
            find_stage_line(&lines, "stage_manager_reverified")?;
            assert_trace_lt(&lines, "stage_global_override_read", "stage_job_md_locked")?;
            assert_trace_lt(&lines, "stage_job_md_locked", "stage_plan_done")?;
            assert_trace_lt(&lines, "stage_plan_done", "stage_impl_session_started")?;
            assert_trace_lt(&lines, "stage_impl_session_started", "stage_impl_done")?;
            assert_trace_lt(&lines, "stage_impl_done", "stage_check_session_started")?;
            assert_trace_lt(&lines, "stage_check_session_started", "stage_check_done")?;
            assert_trace_lt(&lines, "stage_check_done", "stage_manager_reverified")?;
        }
        _ => return Err(format!("ERROR: unsupported mode: {mode}")),
    }
    Ok(format!("PASS: orc_manager trace verified ({mode})"))
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
        "worker-create" => {
            if !tail.is_empty() {
                return Err(format!("{raw_command} does not accept arguments"));
            }
            super::tmux::worker_create().map(|worker| worker.encode())
        }
        "worker-send" => {
            if tail.len() < 2 {
                return Err(
                    "worker-send requires <worker_ref|pane_id> <msg...>|--stdin [enter|enter-exit|raw|display]"
                        .to_string(),
                );
            }
            let worker = super::tmux::resolve_worker_ref(&tail[0])?;
            let (msg, option) = if tail[1] == "--stdin" {
                let option = match tail.get(2).map(String::as_str) {
                    Some("enter" | "enter-exit" | "raw" | "display") => tail[2].as_str(),
                    Some(other) => {
                        return Err(format!(
                            "worker-send --stdin accepts only one optional send option, got {other}"
                        ));
                    }
                    None => "enter",
                };
                (read_send_message_from_stdin("worker-send")?, option)
            } else {
                let (msg_slice, option) = match tail.last().map(String::as_str) {
                    Some("enter" | "enter-exit" | "raw" | "display") if tail.len() >= 3 => {
                        (&tail[1..tail.len() - 1], tail[tail.len() - 1].as_str())
                    }
                    _ => (&tail[1..], "enter"),
                };
                if msg_slice.is_empty() {
                    return Err("worker-send requires non-empty message".to_string());
                }
                (msg_slice.join(" "), option)
            };
            super::tmux::worker_send(
                &worker,
                &msg,
                match option {
                    "display" => super::tmux::SendOption::Display,
                    "enter-exit" => super::tmux::SendOption::EnterExit,
                    "raw" => super::tmux::SendOption::Raw,
                    _ => super::tmux::SendOption::Enter,
                },
            )?;
            Ok(format!(
                "worker-send done: pane={} option={} msg={}",
                worker.pane_id,
                option,
                msg
            ))
        }
        "worker-wait" => {
            if tail.len() < 2 || tail.len() > 4 {
                return Err(
                    "worker-wait requires <worker_ref|pane_id> <pattern> [timeout_ms] [lines]"
                        .to_string(),
                );
            }
            let worker = super::tmux::resolve_worker_ref(&tail[0])?;
            let timeout_ms = if tail.len() >= 3 {
                tail[2]
                    .parse::<u64>()
                    .map_err(|_| "worker-wait: timeout_ms must be an integer".to_string())?
            } else {
                30_000
            };
            let lines = if tail.len() >= 4 {
                tail[3]
                    .parse::<usize>()
                    .map_err(|_| "worker-wait: lines must be a positive integer".to_string())?
            } else {
                120
            };
            super::tmux::worker_wait(&worker, &tail[1], timeout_ms, lines)
        }
        "worker-close" => {
            if tail.len() != 1 {
                return Err("worker-close requires <worker_ref|pane_id>".to_string());
            }
            let worker = super::tmux::resolve_worker_ref(&tail[0])?;
            super::tmux::worker_close(&worker)?;
            Ok(format!("worker-close done: pane={}", worker.pane_id))
        }
        "worker-dev-url" => {
            if tail.is_empty() || tail.len() > 2 {
                return Err("worker-dev-url requires <worker_ref|pane_id> [lines]".to_string());
            }
            let worker = super::tmux::resolve_worker_ref(&tail[0])?;
            let lines = if tail.len() == 2 {
                tail[1]
                    .parse::<usize>()
                    .map_err(|_| "worker-dev-url: lines must be a positive integer".to_string())?
            } else {
                120
            };
            super::tmux::worker_dev_url(&worker, lines)
        }
        "manager-trace" => {
            if tail.is_empty() {
                return Err("manager-trace requires <stage> [detail...]".to_string());
            }
            append_orc_manager_trace(&tail[0], &tail[1..])
        }
        "check-manager-trace" => {
            if tail.len() > 1 {
                return Err(
                    "check-manager-trace accepts zero args or one of: preflight, impl, check, final"
                        .to_string(),
                );
            }
            let mode = tail.first().map(String::as_str).unwrap_or("preflight");
            check_orc_manager_trace(mode)
        }
        "send-tmux" => {
            if tail.len() < 2 {
                return Err(
                    "send-tmux requires <pane_id> <msg...>|--stdin [enter|enter-exit|raw|display]"
                        .to_string(),
                );
            }
            let pane_id = &tail[0];
            let (msg, option) = if tail[1] == "--stdin" {
                let option = match tail.get(2).map(String::as_str) {
                    Some("enter" | "enter-exit" | "raw" | "display") => tail[2].as_str(),
                    Some(other) => {
                        return Err(format!(
                            "send-tmux --stdin accepts only one optional send option, got {other}"
                        ));
                    }
                    None => "enter",
                };
                (read_send_message_from_stdin("send-tmux")?, option)
            } else {
                let (msg_slice, option) = match tail.last().map(String::as_str) {
                    Some("enter" | "enter-exit" | "raw" | "display") if tail.len() >= 3 => {
                        (&tail[1..tail.len() - 1], tail[tail.len() - 1].as_str())
                    }
                    _ => (&tail[1..], "enter"),
                };
                if msg_slice.is_empty() {
                    return Err("send-tmux requires non-empty message".to_string());
                }
                (msg_slice.join(" "), option)
            };
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
                return Err(
                    "clit is deprecated; use check_orc_code and helper commands directly"
                        .to_string(),
                );
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

    #[test]
    fn canonical_command_keeps_worker_create() {
        assert_eq!(canonical_command_for_match("worker-create"), "worker-create");
    }

    #[test]
    fn canonical_command_keeps_worker_dev_url() {
        assert_eq!(canonical_command_for_match("worker-dev-url"), "worker-dev-url");
    }

    #[test]
    fn canonical_command_maps_orc_manager_trace_alias() {
        assert_eq!(
            canonical_command_for_match("orc_manager_trace"),
            "manager-trace"
        );
    }

    #[test]
    fn canonical_command_maps_check_orc_manager_trace_alias() {
        assert_eq!(
            canonical_command_for_match("check_orc_manager_trace"),
            "check-manager-trace"
        );
    }

    #[test]
    fn normalize_stdin_send_message_drops_single_trailing_newline() {
        assert_eq!(
            super::normalize_stdin_send_message("worker-send", "echo hi\n".to_string())
                .expect("message"),
            "echo hi"
        );
    }

    #[test]
    fn normalize_stdin_send_message_rejects_empty_body() {
        assert!(super::normalize_stdin_send_message("worker-send", "\n".to_string()).is_err());
    }
}
