use serde::{Deserialize, Serialize};
use std::env;
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
        "init_orc_project [-n <name>] [-p <path>] [-s <spec>] [-d <description>] [-m <message>] [-a]",
        "build_orc_domains",
        "auto_add_function <message>",
        "init_orc_job",
        "add_orc_drafts",
        "create_job_md",
        "create_input_md",
        "cli_rust_orchestra",
        "impl_orc_code",
        "check_orc_code",
        "open-ui [-w|--web|-b|--build]",
        "serve-web-api [--addr <host:port>]",
        "worker-create [name]",
        "worker-send <worker_ref|pane_id> <msg...>|--stdin [enter|enter-exit|raw|display]",
        "worker-wait <worker_ref|pane_id> <pattern> [timeout_ms] [lines]",
        "worker-close <worker_ref|pane_id>",
        "worker-dev-url <worker_ref|pane_id> [lines]",
        "manager-trace <stage> [detail...]",
        "check-manager-trace [preflight|impl|check|final]",
        "check-manager-completion [job.md]",
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

fn resolve_worker_for_send(target: &str) -> Result<super::tmux::WorkerPaneRef, String> {
    match super::tmux::resolve_worker_ref(target) {
        Ok(worker) => Ok(worker),
        Err(err) => {
            let Some(pane_id) = super::tmux::legacy_worker_ref_pane_id(target) else {
                return Err(err);
            };
            Ok(super::tmux::register_worker_pane(None, pane_id))
        }
    }
}

fn canonical_command_for_match(command: &str) -> &str {
    match command {
        "cli_create_input_md" => "create_input_md",
        "orc_manager_trace" => "manager-trace",
        "check_orc_manager_trace" => "check-manager-trace",
        other => other,
    }
}

const ORC_CANONICAL_STATE_FILE: &str = ".project/log.md";
const CHECK_PROCESS_FILE: &str = ".project/check-process.md";
const TASK_SESSION_KEY_ENV: &str = "ORC_TASK_SESSION_KEY";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ManagerTraceEvent {
    ts: u64,
    kind: String,
    stage: String,
    status: String,
    detail: String,
    job_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    worker_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    artifact: Option<String>,
}

fn supported_manager_trace_stage(stage: &str) -> bool {
    matches!(
        stage,
        "stage_global_override_read"
            | "stage_job_md_locked"
            | "stage_plan_done"
            | "stage_input_locked"
            | "stage_output_locked"
            | "stage_keep_locked"
            | "stage_add_locked"
            | "stage_forbid_locked"
            | "stage_symptom_locked"
            | "stage_success_locked"
            | "stage_impl_session_started"
            | "stage_impl_done"
            | "stage_check_session_started"
            | "stage_check_done"
            | "stage_restart_path_verified"
            | "stage_negative_check_passed"
            | "stage_manager_reverified"
    )
}

fn read_orc_manager_trace_events() -> Result<Vec<ManagerTraceEvent>, String> {
    let raw = fs::read_to_string(ORC_CANONICAL_STATE_FILE)
        .map_err(|_| format!("ERROR: canonical state file missing: {ORC_CANONICAL_STATE_FILE}"))?;
    let mut events = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if !(trimmed.starts_with('{') && trimmed.ends_with('}')) {
            continue;
        }
        let Ok(event) = serde_json::from_str::<ManagerTraceEvent>(trimmed) else {
            continue;
        };
        if event.kind == "manager_trace" {
            events.push(event);
        }
    }
    if events.is_empty() {
        return Err(format!(
            "ERROR: no manager trace events in canonical state: {ORC_CANONICAL_STATE_FILE}"
        ));
    }
    Ok(events)
}

fn active_task_key() -> Option<String> {
    env::var(TASK_SESSION_KEY_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn latest_trace_run(events: &[ManagerTraceEvent]) -> Vec<ManagerTraceEvent> {
    if let Some(task_key) = active_task_key() {
        let keyed: Vec<ManagerTraceEvent> = events
            .iter()
            .filter(|event| event.task_key.as_deref() == Some(task_key.as_str()))
            .cloned()
            .collect();
        if !keyed.is_empty() {
            return keyed;
        }
    }

    let start = events
        .iter()
        .rposition(|event| event.stage == "stage_global_override_read")
        .unwrap_or(0);
    events[start..].to_vec()
}

fn find_stage_line(events: &[ManagerTraceEvent], stage: &str) -> Result<usize, String> {
    events
        .iter()
        .position(|event| event.stage == stage)
        .map(|idx| idx + 1)
        .ok_or_else(|| format!("ERROR: missing required orc_manager trace: {stage}"))
}

fn find_stage_event<'a>(
    events: &'a [ManagerTraceEvent],
    stage: &str,
) -> Result<(usize, &'a ManagerTraceEvent), String> {
    events
        .iter()
        .enumerate()
        .find(|(_, event)| event.stage == stage)
        .map(|(idx, event)| (idx + 1, event))
        .ok_or_else(|| format!("ERROR: missing required orc_manager trace: {stage}"))
}

fn assert_trace_lt(events: &[ManagerTraceEvent], left: &str, right: &str) -> Result<(), String> {
    let left_line = find_stage_line(events, left)?;
    let right_line = find_stage_line(events, right)?;
    if left_line >= right_line {
        return Err(format!(
            "ERROR: invalid orc_manager trace order: {left} should precede {right}"
        ));
    }
    Ok(())
}

fn infer_trace_status(detail_text: &str) -> &'static str {
    let lower = detail_text.to_ascii_lowercase();
    if lower.contains("status=0") || lower.contains("code=0") {
        return "ok";
    }
    if lower.contains("status=") || lower.contains("code=") || lower.contains(":error") {
        return "error";
    }
    "ok"
}

fn extract_tag_value(detail_text: &str, key: &str) -> Option<String> {
    for token in detail_text.split(|ch: char| ch.is_whitespace() || ch == '|' || ch == ';') {
        let normalized = token.trim_matches(|ch| matches!(ch, ',' | ')' | '('));
        if let Some(rest) = normalized.strip_prefix(&format!("{key}=")) {
            let value = rest.trim_matches(|ch| matches!(ch, '"' | '\'' | ',' | ';'));
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn extract_artifact(detail_text: &str) -> Option<String> {
    if let Some(url) = detail_text
        .split(|ch: char| ch.is_whitespace() || ch == '|' || ch == ';')
        .find(|token| token.starts_with("http://") || token.starts_with("https://"))
    {
        return Some(url.trim_matches(|ch| matches!(ch, ',' | ';')).to_string());
    }
    extract_tag_value(detail_text, "artifact").or_else(|| extract_tag_value(detail_text, "impl"))
}

fn ensure_stage_ok(events: &[ManagerTraceEvent], stage: &str) -> Result<(), String> {
    let (_, event) = find_stage_event(events, stage)?;
    if event.status != "ok" {
        return Err(format!(
            "ERROR: manager trace stage is not ok: {stage} status={} detail={}",
            event.status, event.detail
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
    let event = ManagerTraceEvent {
        ts,
        kind: "manager_trace".to_string(),
        stage: stage.to_string(),
        status: infer_trace_status(&detail_text).to_string(),
        detail: detail_text.clone(),
        job_path: "job.md".to_string(),
        task_key: active_task_key(),
        worker_ref: extract_tag_value(&detail_text, "worker")
            .or_else(|| extract_tag_value(&detail_text, "worker_ref")),
        artifact: extract_artifact(&detail_text),
    };
    let line = serde_json::to_string(&event)
        .map_err(|e| format!("failed to encode manager trace event: {e}"))?;

    let mut trace = OpenOptions::new()
        .create(true)
        .append(true)
        .open(ORC_CANONICAL_STATE_FILE)
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
    let events = read_orc_manager_trace_events()?;
    let run_lines = latest_trace_run(&events);
    match mode {
        "preflight" | "impl" => {
            find_stage_line(&run_lines, "stage_global_override_read")?;
            find_stage_line(&run_lines, "stage_job_md_locked")?;
            find_stage_line(&run_lines, "stage_plan_done")?;
            find_stage_line(&run_lines, "stage_input_locked")?;
            find_stage_line(&run_lines, "stage_output_locked")?;
            find_stage_line(&run_lines, "stage_keep_locked")?;
            find_stage_line(&run_lines, "stage_add_locked")?;
            find_stage_line(&run_lines, "stage_forbid_locked")?;
            find_stage_line(&run_lines, "stage_symptom_locked")?;
            find_stage_line(&run_lines, "stage_success_locked")?;
            assert_trace_lt(&run_lines, "stage_global_override_read", "stage_job_md_locked")?;
            assert_trace_lt(&run_lines, "stage_job_md_locked", "stage_plan_done")?;
            assert_trace_lt(&run_lines, "stage_plan_done", "stage_input_locked")?;
            assert_trace_lt(&run_lines, "stage_input_locked", "stage_output_locked")?;
            assert_trace_lt(&run_lines, "stage_output_locked", "stage_keep_locked")?;
            assert_trace_lt(&run_lines, "stage_keep_locked", "stage_add_locked")?;
            assert_trace_lt(&run_lines, "stage_add_locked", "stage_forbid_locked")?;
            assert_trace_lt(&run_lines, "stage_forbid_locked", "stage_symptom_locked")?;
            assert_trace_lt(&run_lines, "stage_symptom_locked", "stage_success_locked")?;
            ensure_stage_ok(&run_lines, "stage_global_override_read")?;
            ensure_stage_ok(&run_lines, "stage_job_md_locked")?;
            ensure_stage_ok(&run_lines, "stage_plan_done")?;
        }
        "check" => {
            find_stage_line(&run_lines, "stage_impl_session_started")?;
            find_stage_line(&run_lines, "stage_impl_done")?;
            assert_trace_lt(&run_lines, "stage_global_override_read", "stage_job_md_locked")?;
            assert_trace_lt(&run_lines, "stage_job_md_locked", "stage_plan_done")?;
            assert_trace_lt(&run_lines, "stage_plan_done", "stage_impl_session_started")?;
            assert_trace_lt(&run_lines, "stage_impl_session_started", "stage_impl_done")?;
            ensure_stage_ok(&run_lines, "stage_impl_session_started")?;
            ensure_stage_ok(&run_lines, "stage_impl_done")?;
        }
        "final" => {
            find_stage_line(&run_lines, "stage_impl_session_started")?;
            find_stage_line(&run_lines, "stage_impl_done")?;
            find_stage_line(&run_lines, "stage_check_session_started")?;
            find_stage_line(&run_lines, "stage_check_done")?;
            find_stage_line(&run_lines, "stage_restart_path_verified")?;
            find_stage_line(&run_lines, "stage_negative_check_passed")?;
            find_stage_line(&run_lines, "stage_manager_reverified")?;
            assert_trace_lt(&run_lines, "stage_global_override_read", "stage_job_md_locked")?;
            assert_trace_lt(&run_lines, "stage_job_md_locked", "stage_plan_done")?;
            assert_trace_lt(&run_lines, "stage_plan_done", "stage_impl_session_started")?;
            assert_trace_lt(&run_lines, "stage_impl_session_started", "stage_impl_done")?;
            assert_trace_lt(&run_lines, "stage_impl_done", "stage_check_session_started")?;
            assert_trace_lt(&run_lines, "stage_check_session_started", "stage_check_done")?;
            assert_trace_lt(&run_lines, "stage_check_done", "stage_manager_reverified")?;
            assert_trace_lt(
                &run_lines,
                "stage_restart_path_verified",
                "stage_negative_check_passed",
            )?;
            assert_trace_lt(&run_lines, "stage_negative_check_passed", "stage_manager_reverified")?;
            ensure_stage_ok(&run_lines, "stage_impl_session_started")?;
            ensure_stage_ok(&run_lines, "stage_impl_done")?;
            ensure_stage_ok(&run_lines, "stage_check_session_started")?;
            ensure_stage_ok(&run_lines, "stage_check_done")?;
            ensure_stage_ok(&run_lines, "stage_restart_path_verified")?;
            ensure_stage_ok(&run_lines, "stage_negative_check_passed")?;
            ensure_stage_ok(&run_lines, "stage_manager_reverified")?;
        }
        _ => return Err(format!("ERROR: unsupported mode: {mode}")),
    }
    Ok(format!("PASS: orc_manager trace verified ({mode})"))
}

fn collect_bullet_lines_in_section(raw: &str, header: &str) -> Vec<String> {
    let mut in_section = false;
    let mut items = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case(header) {
            in_section = true;
            continue;
        }
        if in_section && trimmed.starts_with('#') {
            break;
        }
        if in_section && trimmed.starts_with("- ") {
            items.push(trimmed.to_string());
        }
    }
    items
}

fn collect_unchecked_verify_lines(raw: &str) -> Vec<String> {
    let mut in_section = false;
    let mut items = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("## verify") {
            in_section = true;
            continue;
        }
        if in_section && trimmed.starts_with("## ") {
            break;
        }
        if in_section && trimmed.starts_with('#') && !trimmed.starts_with("## ") {
            break;
        }
        if in_section && trimmed.starts_with("- [ ]") {
            items.push(trimmed.to_string());
        }
    }
    items
}

fn check_manager_completion_from_raw(raw: &str) -> Result<String, String> {
    let top_level_problems = collect_bullet_lines_in_section(raw, "# problems");
    if !top_level_problems.is_empty() {
        return Err(format!(
            "ERROR: unresolved blocking items remain in # problems\n{}",
            top_level_problems.join("\n")
        ));
    }
    let nested_problems = collect_bullet_lines_in_section(raw, "## problems");
    if !nested_problems.is_empty() {
        return Err(format!(
            "ERROR: unresolved blocking items remain in ## problems\n{}",
            nested_problems.join("\n")
        ));
    }
    let unchecked_verify = collect_unchecked_verify_lines(raw);
    if !unchecked_verify.is_empty() {
        return Err(format!(
            "ERROR: unchecked verify items remain in ## verify\n{}",
            unchecked_verify.join("\n")
        ));
    }
    Ok("PASS: orc manager completion guard verified".to_string())
}

fn check_manager_completion(job_file: &str) -> Result<String, String> {
    let raw = fs::read_to_string(job_file)
        .map_err(|_| format!("ERROR: job file not found: {}", job_file))?;
    check_manager_completion_from_raw(&raw)?;
    check_orc_manager_trace("final")?;
    Ok("PASS: orc manager completion guard verified".to_string())
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
            if tail.len() > 1 {
                return Err(format!("{raw_command} accepts at most one optional [name]"));
            }
            super::tmux::worker_create(tail.first().map(String::as_str)).map(|worker| worker.encode())
        }
        "worker-send" => {
            if tail.len() < 2 {
                return Err(
                    "worker-send requires <worker_ref|pane_id> <msg...>|--stdin [enter|enter-exit|raw|display]"
                        .to_string(),
                );
            }
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
            let worker = resolve_worker_for_send(&tail[0])?;
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
                "worker-send done: pane={} option={} msg_len={}",
                worker.pane_id,
                option,
                msg.len()
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
        "check-manager-completion" => {
            if tail.len() > 1 {
                return Err("check-manager-completion accepts zero args or one optional [job.md]".to_string());
            }
            let job_file = tail.first().map(String::as_str).unwrap_or("job.md");
            check_manager_completion(job_file)
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
        _ => Err(format!("unknown command: {}", command)),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        assert_trace_lt, canonical_command_for_match, is_help_command, latest_trace_run,
        ManagerTraceEvent,
    };

    fn trace_event(ts: u64, stage: &str, detail: &str) -> ManagerTraceEvent {
        ManagerTraceEvent {
            ts,
            kind: "manager_trace".to_string(),
            stage: stage.to_string(),
            status: "ok".to_string(),
            detail: detail.to_string(),
            job_path: "job.md".to_string(),
            task_key: None,
            worker_ref: None,
            artifact: None,
        }
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

    #[test]
    fn worker_send_legacy_ref_extracts_same_pane_for_retry() {
        let pane_id = "%17";
        let legacy_ref = format!("worker-123::{}::1000", pane_id);
        let worker = super::resolve_worker_for_send(&legacy_ref).expect("legacy worker retry");
        assert_eq!(worker.pane_id, pane_id);
    }

    #[test]
    fn check_manager_completion_rejects_problem_and_verify_remnants() {
        let raw = "# problems\n- blocker\n\n## verify\n- [ ] unresolved\n";
        let err = super::check_manager_completion_from_raw(raw).expect_err("must fail");
        assert!(err.contains("# problems"));
    }

    #[test]
    fn check_manager_completion_accepts_clean_job_md() {
        let raw = "# problems\n\n## verify\n- [x] done\n";
        let result = super::check_manager_completion_from_raw(raw).expect("must pass");
        assert!(result.contains("PASS"));
    }

    #[test]
    fn latest_trace_run_ignores_older_trace_blocks() {
        let lines = vec![
            trace_event(1, "stage_global_override_read", "old"),
            trace_event(1, "stage_job_md_locked", "old"),
            trace_event(1, "stage_plan_done", "old"),
            trace_event(1, "stage_impl_session_started", "old"),
            trace_event(1, "stage_impl_done", "old"),
            trace_event(2, "stage_global_override_read", "new"),
            trace_event(2, "stage_job_md_locked", "new"),
            trace_event(2, "stage_plan_done", "new"),
        ];

        let run = latest_trace_run(&lines);

        assert_eq!(run.len(), 3);
        assert_eq!(run[0].stage, "stage_global_override_read");
        assert_eq!(run[0].detail, "new");
    }

    #[test]
    fn final_trace_allows_restart_checks_before_check_done() {
        let lines = vec![
            trace_event(1, "stage_global_override_read", ""),
            trace_event(1, "stage_job_md_locked", ""),
            trace_event(1, "stage_plan_done", ""),
            trace_event(1, "stage_impl_session_started", ""),
            trace_event(1, "stage_impl_done", ""),
            trace_event(1, "stage_check_session_started", ""),
            trace_event(1, "stage_restart_path_verified", ""),
            trace_event(1, "stage_negative_check_passed", ""),
            trace_event(1, "stage_check_done", ""),
            trace_event(1, "stage_manager_reverified", ""),
        ];

        let run = latest_trace_run(&lines);

        assert!(assert_trace_lt(&run, "stage_check_done", "stage_manager_reverified").is_ok());
        assert!(assert_trace_lt(&run, "stage_restart_path_verified", "stage_negative_check_passed").is_ok());
        assert!(assert_trace_lt(&run, "stage_negative_check_passed", "stage_manager_reverified").is_ok());
    }
}
