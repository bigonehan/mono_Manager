use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const CODEX_DANGEROUS_FLAG: &str = "--dangerously-bypass-approvals-and-sandbox";
const LONG_WAIT_REPORT_SEC: u64 = 60;

#[derive(Debug, Clone, Copy)]
pub(crate) struct LlmProgressWatch {
    pub soft_timeout_sec: u64,
    pub stall_timeout_sec: u64,
    pub hard_timeout_sec: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProgressWatchDecision {
    Continue,
    StartMonitoring,
    Stalled,
    HardTimedOut,
}

fn append_chat_log(project_root: &Path, role: &str, message: &str) {
    let debug_enabled = crate::load_app_config()
        .as_ref()
        .is_none_or(crate::config::AppConfig::debug_enabled);
    if !debug_enabled {
        return;
    }
    let path = project_root.join(".project").join("chat.log");
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "[{}] {}", ts, role);
        let _ = writeln!(file, "{}", message);
        let _ = writeln!(file);
    }
}

fn trim_wait_detail(detail: &str) -> String {
    let normalized = detail.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.len() <= 160 {
        return normalized;
    }
    format!("{}...", &normalized[..157])
}

fn prompt_trace_label(prompt: &str) -> String {
    prompt
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(trim_wait_detail)
        .filter(|line| !line.is_empty())
        .unwrap_or_else(|| "llm prompt".to_string())
}

fn read_last_non_empty_line(path: &Path) -> Option<String> {
    let raw = fs::read_to_string(path).ok()?;
    raw.lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(trim_wait_detail)
}

fn emit_owner_wait_status(project_root: &Path, owner_pane: Option<&str>, role: &str, status: &str) {
    eprintln!("{}", status);
    append_chat_log(project_root, role, status);
    let _ = crate::append_check_process_status(role, status);
    if let Some(pane_id) = owner_pane {
        let _ = crate::tmux::display_message(pane_id, status);
    }
}

fn should_skip_activity_dir(name: &str) -> bool {
    matches!(name, ".git" | ".project" | "target" | "node_modules")
}

fn latest_workspace_activity_time(root: &Path) -> Option<SystemTime> {
    fn visit(path: &Path, latest: &mut Option<SystemTime>) {
        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let entry_path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };
            if file_type.is_dir() {
                if should_skip_activity_dir(&entry.file_name().to_string_lossy()) {
                    continue;
                }
                visit(&entry_path, latest);
                continue;
            }
            let modified = match entry.metadata().and_then(|meta| meta.modified()) {
                Ok(modified) => modified,
                Err(_) => continue,
            };
            if latest.is_none_or(|current| modified > current) {
                *latest = Some(modified);
            }
        }
    }

    let mut latest = None;
    visit(root, &mut latest);
    latest
}

fn detect_workspace_activity(root: &Path, last_seen: &mut Option<SystemTime>) -> bool {
    let current = latest_workspace_activity_time(root);
    match (current, *last_seen) {
        (Some(current), Some(previous)) if current > previous => {
            *last_seen = Some(current);
            true
        }
        (Some(current), None) => {
            *last_seen = Some(current);
            false
        }
        _ => false,
    }
}

fn progress_watch_decision(
    watch: LlmProgressWatch,
    elapsed_sec: u64,
    monitor_elapsed_sec: Option<u64>,
    progress_elapsed_sec: Option<u64>,
) -> ProgressWatchDecision {
    if elapsed_sec < watch.soft_timeout_sec {
        return ProgressWatchDecision::Continue;
    }
    if elapsed_sec >= watch.hard_timeout_sec {
        return ProgressWatchDecision::HardTimedOut;
    }
    if monitor_elapsed_sec.is_none() {
        return ProgressWatchDecision::StartMonitoring;
    }
    let idle_sec = progress_elapsed_sec.or(monitor_elapsed_sec).unwrap_or(0);
    if idle_sec >= watch.stall_timeout_sec {
        ProgressWatchDecision::Stalled
    } else {
        ProgressWatchDecision::Continue
    }
}

fn codex_exec_timeout_sec() -> u64 {
    crate::load_app_config()
        .as_ref()
        .map_or(300, crate::config::AppConfig::default_timeout_sec)
        .max(1)
}

fn run_command_with_timeout(
    mut command: Command,
    timeout_sec: u64,
    timeout_label: &str,
    project_root: &Path,
    wait_reason: &str,
    progress_watch: Option<LlmProgressWatch>,
) -> Result<Output, String> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|e| format!("failed to spawn {}: {}", timeout_label, e))?;
    let started = Instant::now();
    let mut next_report_sec = LONG_WAIT_REPORT_SEC;
    let mut last_workspace_activity = latest_workspace_activity_time(project_root);
    let mut monitor_started_at: Option<Instant> = None;
    let mut last_progress_after_monitor: Option<Instant> = None;
    loop {
        match child
            .try_wait()
            .map_err(|e| format!("failed while waiting {}: {}", timeout_label, e))?
        {
            Some(_) => {
                return child
                    .wait_with_output()
                    .map_err(|e| format!("failed to collect output for {}: {}", timeout_label, e));
            }
            None => {
                let elapsed_sec = started.elapsed().as_secs();
                if detect_workspace_activity(project_root, &mut last_workspace_activity)
                    && monitor_started_at.is_some()
                {
                    last_progress_after_monitor = Some(Instant::now());
                }
                if elapsed_sec >= next_report_sec {
                    let wait_detail = if let Some(watch) = progress_watch {
                        if elapsed_sec >= watch.soft_timeout_sec {
                            if last_progress_after_monitor.is_some() {
                                "implementation still progressing after soft timeout"
                            } else {
                                "soft timeout reached, monitoring for implementation progress"
                            }
                        } else {
                            wait_reason
                        }
                    } else {
                        wait_reason
                    };
                    let status = format!(
                        "[orc-status] {} | elapsed={}s | {}",
                        timeout_label, elapsed_sec, wait_detail
                    );
                    let progress_status = if let Some(watch) = progress_watch {
                        if elapsed_sec >= watch.soft_timeout_sec && last_progress_after_monitor.is_some() {
                            "slow_progress"
                        } else if elapsed_sec >= watch.soft_timeout_sec {
                            "soft_timeout_monitoring"
                        } else {
                            "running"
                        }
                    } else {
                        "running"
                    };
                    crate::code::update_impl_draft_progress_from_watch(
                        project_root,
                        timeout_label,
                        progress_status,
                        elapsed_sec,
                        wait_detail,
                    );
                    emit_owner_wait_status(project_root, None, "LLM_WAIT", &status);
                    next_report_sec += LONG_WAIT_REPORT_SEC;
                }
                if let Some(watch) = progress_watch {
                    let monitor_elapsed_sec =
                        monitor_started_at.map(|instant| instant.elapsed().as_secs());
                    let progress_elapsed_sec = last_progress_after_monitor
                        .map(|instant| instant.elapsed().as_secs());
                    match progress_watch_decision(
                        watch,
                        elapsed_sec,
                        monitor_elapsed_sec,
                        progress_elapsed_sec,
                    ) {
                        ProgressWatchDecision::Continue => {}
                        ProgressWatchDecision::StartMonitoring => {
                            monitor_started_at = Some(Instant::now());
                            let status = format!(
                                "[orc-status] {} | elapsed={}s | soft timeout reached, checking whether implementation is still progressing",
                                timeout_label, elapsed_sec
                            );
                            crate::code::update_impl_draft_progress_from_watch(
                                project_root,
                                timeout_label,
                                "soft_timeout_monitoring",
                                elapsed_sec,
                                "soft timeout reached, checking whether implementation is still progressing",
                            );
                            emit_owner_wait_status(project_root, None, "LLM_WAIT", &status);
                        }
                        ProgressWatchDecision::Stalled => {
                            let _ = child.kill();
                            let _ = child.wait();
                            crate::code::update_impl_draft_progress_from_watch(
                                project_root,
                                timeout_label,
                                "suspected_stall",
                                elapsed_sec,
                                &format!(
                                    "stalled after soft timeout {}s with no progress for {}s",
                                    watch.soft_timeout_sec, watch.stall_timeout_sec
                                ),
                            );
                            return Err(format!(
                                "{} stalled after soft timeout {}s with no progress for {}s",
                                timeout_label, watch.soft_timeout_sec, watch.stall_timeout_sec
                            ));
                        }
                        ProgressWatchDecision::HardTimedOut => {
                            let _ = child.kill();
                            let _ = child.wait();
                            crate::code::update_impl_draft_progress_from_watch(
                                project_root,
                                timeout_label,
                                "hard_timeout",
                                elapsed_sec,
                                &format!("timed out after hard timeout {}s", watch.hard_timeout_sec),
                            );
                            return Err(format!(
                                "{} timed out after hard timeout {}s",
                                timeout_label, watch.hard_timeout_sec
                            ));
                        }
                    }
                } else if started.elapsed() >= Duration::from_secs(timeout_sec) {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "{} timed out after {}s",
                        timeout_label, timeout_sec
                    ));
                }
                thread::sleep(Duration::from_millis(200));
            }
        }
    }
}

#[derive(Debug, Clone)]
struct LlmExecResult {
    success: bool,
    stdout: String,
    stderr: String,
}

fn should_use_tmux_for_llm() -> bool {
    if !env_flag_true("ORC_USE_TMUX_PANES") {
        return false;
    }
    let debug_enabled = crate::load_app_config()
        .as_ref()
        .is_none_or(crate::config::AppConfig::debug_enabled);
    debug_enabled
        && env::var("TMUX")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
}

fn env_flag_true(name: &str) -> bool {
    match env::var(name) {
        Ok(v) => {
            let lowered = v.trim().to_ascii_lowercase();
            lowered == "1" || lowered == "true" || lowered == "yes" || lowered == "on"
        }
        Err(_) => false,
    }
}

fn llm_retry_count() -> u32 {
    crate::load_app_config()
        .as_ref()
        .map_or(2, crate::config::AppConfig::llm_retry_count)
        .max(1)
}

fn quote_sh(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn normalize_exec_prompt(prompt: &str) -> String {
    if prompt.contains("쳐아서") {
        prompt.replace("쳐아서", "찾아서")
    } else {
        prompt.to_string()
    }
}

fn run_llm_via_tmux(
    dir: &Path,
    llm_bin: &str,
    prompt: &str,
    timeout_sec: u64,
    add_yes_flag: bool,
    add_dangerous_flag: bool,
    timeout_label: &str,
    progress_watch: Option<LlmProgressWatch>,
) -> Result<LlmExecResult, String> {
    let prompt = normalize_exec_prompt(prompt);
    let runtime = dir.join(".project").join("runtime");
    fs::create_dir_all(&runtime)
        .map_err(|e| format!("failed to create runtime dir {}: {}", runtime.display(), e))?;
    let stamp = crate::now_unix();
    let token = format!("{}_{}", stamp, std::process::id());
    let prompt_path = runtime.join(format!("tmux-llm-{}.prompt.txt", token));
    let script_path = runtime.join(format!("tmux-llm-{}.sh", token));
    let stdout_path = runtime.join(format!("tmux-llm-{}.stdout.log", token));
    let stderr_path = runtime.join(format!("tmux-llm-{}.stderr.log", token));
    let code_path = runtime.join(format!("tmux-llm-{}.code", token));
    fs::write(&prompt_path, prompt.as_str())
        .map_err(|e| format!("failed to write {}: {}", prompt_path.display(), e))?;

    let mut flags = Vec::new();
    if add_yes_flag {
        flags.push("-y".to_string());
    }
    if add_dangerous_flag {
        flags.push(CODEX_DANGEROUS_FLAG.to_string());
    }
    let flags_joined = if flags.is_empty() {
        String::new()
    } else {
        format!(" {}", flags.join(" "))
    };
    let script = format!(
        "#!/usr/bin/env bash\n\
cd {dir}\n\
echo \"[orc-llm] start: {llm} exec\"\n\
echo \"[orc-llm] cwd: {dir_display}\"\n\
{llm} exec{flags} \"$(cat {prompt})\" > >(tee {stdout}) 2> >(tee {stderr} >&2)\n\
status=$?\n\
printf \"%s\" \"$status\" > {code}\n",
        dir = quote_sh(&dir.display().to_string()),
        dir_display = dir.display(),
        llm = quote_sh(llm_bin),
        flags = flags_joined,
        prompt = quote_sh(&prompt_path.display().to_string()),
        stdout = quote_sh(&stdout_path.display().to_string()),
        stderr = quote_sh(&stderr_path.display().to_string()),
        code = quote_sh(&code_path.display().to_string()),
    );
    fs::write(&script_path, script)
        .map_err(|e| format!("failed to write {}: {}", script_path.display(), e))?;

    let script_cmd = format!("bash {}", quote_sh(&script_path.display().to_string()));
    let pane_id = crate::tmux::split_window_run(&script_cmd)
        .map_err(|e| format!("{} (tmux split/run failed: {})", timeout_label, e))?;
    let worker = crate::tmux::register_worker_pane(&pane_id);
    let _ = crate::tmux::rename_pane(&worker.pane_id, &format!("llm-{}", worker.short_id()));

    let started = Instant::now();
    let parent_pane = crate::tmux::current_pane_id().ok();
    let mut next_report_sec = LONG_WAIT_REPORT_SEC;
    let mut last_workspace_activity = latest_workspace_activity_time(dir);
    let mut monitor_started_at: Option<Instant> = None;
    let mut last_progress_after_monitor: Option<Instant> = None;
    let mut last_status_line = String::new();
    while !code_path.exists() {
        let elapsed_sec = started.elapsed().as_secs();
        let latest = read_last_non_empty_line(&stderr_path)
            .or_else(|| read_last_non_empty_line(&stdout_path))
            .unwrap_or_else(|| "tmux worker still waiting for llm response".to_string());
        if latest != last_status_line {
            last_status_line = latest.clone();
            if monitor_started_at.is_some() {
                last_progress_after_monitor = Some(Instant::now());
            }
        }
        if detect_workspace_activity(dir, &mut last_workspace_activity) && monitor_started_at.is_some()
        {
            last_progress_after_monitor = Some(Instant::now());
        }
        if elapsed_sec >= next_report_sec {
            let latest = if let Some(watch) = progress_watch {
                if elapsed_sec >= watch.soft_timeout_sec && last_progress_after_monitor.is_some() {
                    format!("implementation still progressing | {}", latest)
                } else if elapsed_sec >= watch.soft_timeout_sec {
                    format!("soft timeout reached, monitoring progress | {}", latest)
                } else {
                    latest
                }
            } else {
                latest
            };
            let status = format!(
                "[orc-status] {} | elapsed={}s | {}",
                timeout_label, elapsed_sec, latest
            );
            let progress_status = if let Some(watch) = progress_watch {
                if elapsed_sec >= watch.soft_timeout_sec && last_progress_after_monitor.is_some() {
                    "slow_progress"
                } else if elapsed_sec >= watch.soft_timeout_sec {
                    "soft_timeout_monitoring"
                } else {
                    "running"
                }
            } else {
                "running"
            };
            crate::code::update_impl_draft_progress_from_watch(
                dir,
                timeout_label,
                progress_status,
                elapsed_sec,
                &latest,
            );
            emit_owner_wait_status(dir, parent_pane.as_deref(), "LLM_WAIT", &status);
            next_report_sec += LONG_WAIT_REPORT_SEC;
        }
        if let Some(watch) = progress_watch {
            let monitor_elapsed_sec = monitor_started_at.map(|instant| instant.elapsed().as_secs());
            let progress_elapsed_sec =
                last_progress_after_monitor.map(|instant| instant.elapsed().as_secs());
            match progress_watch_decision(watch, elapsed_sec, monitor_elapsed_sec, progress_elapsed_sec)
            {
                ProgressWatchDecision::Continue => {}
                ProgressWatchDecision::StartMonitoring => {
                    monitor_started_at = Some(Instant::now());
                    let status = format!(
                        "[orc-status] {} | elapsed={}s | soft timeout reached, checking whether implementation is still progressing",
                        timeout_label, elapsed_sec
                    );
                    crate::code::update_impl_draft_progress_from_watch(
                        dir,
                        timeout_label,
                        "soft_timeout_monitoring",
                        elapsed_sec,
                        "soft timeout reached, checking whether implementation is still progressing",
                    );
                    emit_owner_wait_status(dir, parent_pane.as_deref(), "LLM_WAIT", &status);
                }
                ProgressWatchDecision::Stalled => {
                    let _ = crate::tmux::kill_worker_pane(&worker);
                    crate::code::update_impl_draft_progress_from_watch(
                        dir,
                        timeout_label,
                        "suspected_stall",
                        elapsed_sec,
                        &format!(
                            "stalled after soft timeout {}s with no progress for {}s",
                            watch.soft_timeout_sec, watch.stall_timeout_sec
                        ),
                    );
                    return Err(format!(
                        "{} stalled after soft timeout {}s with no progress for {}s",
                        timeout_label, watch.soft_timeout_sec, watch.stall_timeout_sec
                    ));
                }
                ProgressWatchDecision::HardTimedOut => {
                    let _ = crate::tmux::kill_worker_pane(&worker);
                    crate::code::update_impl_draft_progress_from_watch(
                        dir,
                        timeout_label,
                        "hard_timeout",
                        elapsed_sec,
                        &format!("timed out after hard timeout {}s", watch.hard_timeout_sec),
                    );
                    return Err(format!(
                        "{} timed out after hard timeout {}s",
                        timeout_label, watch.hard_timeout_sec
                    ));
                }
            }
        } else if started.elapsed() >= Duration::from_secs(timeout_sec) {
            let _ = crate::tmux::kill_worker_pane(&worker);
            return Err(format!(
                "{} timed out after {}s",
                timeout_label, timeout_sec
            ));
        }
        thread::sleep(Duration::from_millis(200));
    }
    let _ = crate::tmux::kill_worker_pane(&worker);

    let code_raw = fs::read_to_string(&code_path)
        .map_err(|e| format!("failed to read {}: {}", code_path.display(), e))?;
    let code = code_raw.trim().parse::<i32>().unwrap_or(1);
    let stdout = fs::read_to_string(&stdout_path).unwrap_or_default();
    let stderr = fs::read_to_string(&stderr_path).unwrap_or_default();
    Ok(LlmExecResult {
        success: code == 0,
        stdout,
        stderr: stderr.trim().to_string(),
    })
}

pub(crate) fn run_codex_exec_capture_with_timeout(
    prompt: &str,
    timeout_sec: u64,
) -> Result<String, String> {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let prompt = normalize_exec_prompt(prompt);
    let model_bin = crate::default_model_bin();
    let trace_label = format!("{} exec [{}]", model_bin, prompt_trace_label(&prompt));
    run_codex_exec_capture_in_dir_with_attempts_labeled(
        &cwd,
        &prompt,
        timeout_sec,
        llm_retry_count(),
        Some(&trace_label),
    )
}

pub(crate) fn run_codex_exec_capture(prompt: &str) -> Result<String, String> {
    run_codex_exec_capture_with_timeout(prompt, codex_exec_timeout_sec())
}

pub(crate) fn run_codex_exec_capture_in_dir_with_progress_watch(
    dir: &Path,
    prompt: &str,
    timeout_sec: u64,
    progress_watch: LlmProgressWatch,
    trace_label: &str,
    total_attempts: u32,
) -> Result<String, String> {
    run_codex_exec_capture_in_dir_with_attempts_labeled_and_progress_watch(
        dir,
        prompt,
        timeout_sec,
        total_attempts,
        Some(trace_label),
        Some(progress_watch),
    )
}

pub(crate) fn run_codex_exec_capture_in_dir(dir: &Path, prompt: &str) -> Result<String, String> {
    run_codex_exec_capture_in_dir_with_timeout(dir, prompt, codex_exec_timeout_sec())
}

fn run_codex_exec_capture_in_dir_with_attempts(
    dir: &Path,
    prompt: &str,
    timeout_sec: u64,
    total_attempts: u32,
) -> Result<String, String> {
    run_codex_exec_capture_in_dir_with_attempts_labeled(
        dir,
        prompt,
        timeout_sec,
        total_attempts,
        None,
    )
}

fn run_codex_exec_capture_in_dir_with_attempts_labeled(
    dir: &Path,
    prompt: &str,
    timeout_sec: u64,
    total_attempts: u32,
    trace_label: Option<&str>,
) -> Result<String, String> {
    run_codex_exec_capture_in_dir_with_attempts_labeled_and_progress_watch(
        dir,
        prompt,
        timeout_sec,
        total_attempts,
        trace_label,
        None,
    )
}

fn run_codex_exec_capture_in_dir_with_attempts_labeled_and_progress_watch(
    dir: &Path,
    prompt: &str,
    timeout_sec: u64,
    total_attempts: u32,
    trace_label: Option<&str>,
    progress_watch: Option<LlmProgressWatch>,
) -> Result<String, String> {
    let prompt = normalize_exec_prompt(prompt);
    append_chat_log(dir, "LLM_PROMPT", &prompt);
    let model_bin = crate::default_model_bin();
    let dangerous = crate::model_supports_dangerous_flag(&model_bin);
    let total_attempts = total_attempts.max(1);
    let mut last_error = "unknown llm error".to_string();
    for attempt in 1..=total_attempts {
        let base_label = trace_label
            .map(str::trim)
            .filter(|label| !label.is_empty())
            .map(|label| label.to_string())
            .unwrap_or_else(|| format!("{} exec in {}", model_bin, dir.display()));
        let attempt_label = if total_attempts > 1 {
            format!("{} attempt {}/{}", base_label, attempt, total_attempts)
        } else {
            base_label.clone()
        };
        let start_detail = format!(
            "{} | cwd={} | timeout={}s",
            attempt_label,
            dir.display(),
            timeout_sec
        );
        append_chat_log(dir, "LLM_START", &start_detail);
        let _ = crate::append_check_process_status("LLM_START", &start_detail);
        if should_use_tmux_for_llm() {
            match run_llm_via_tmux(
                dir,
                &model_bin,
                &prompt,
                timeout_sec,
                false,
                dangerous,
                &attempt_label,
                progress_watch,
            ) {
                Ok(result) => {
                    if result.success {
                        append_chat_log(dir, "LLM_RESPONSE", &result.stdout);
                        return Ok(result.stdout);
                    }
                    last_error = result.stderr;
                }
                Err(e) => {
                    last_error = e;
                }
            }
        } else {
            let mut command = Command::new(&model_bin);
            command.current_dir(dir).arg("exec");
            if dangerous {
                command.arg(CODEX_DANGEROUS_FLAG);
            }
            command.arg(&prompt);
            match run_command_with_timeout(
                command,
                timeout_sec,
                &attempt_label,
                dir,
                "waiting for llm response",
                progress_watch,
            ) {
                Ok(output) if output.status.success() => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    append_chat_log(dir, "LLM_RESPONSE", &stdout);
                    return Ok(stdout);
                }
                Ok(output) => {
                    last_error = String::from_utf8_lossy(&output.stderr).trim().to_string();
                }
                Err(e) => {
                    last_error = e;
                }
            }
        }
        append_chat_log(
            dir,
            "LLM_RETRY",
            &format!(
                "attempt {}/{} failed: {}",
                attempt, total_attempts, last_error
            ),
        );
    }
    append_chat_log(dir, "LLM_ERROR", &last_error);
    Err(last_error)
}

pub(crate) fn run_codex_exec_capture_in_dir_with_timeout(
    dir: &Path,
    prompt: &str,
    timeout_sec: u64,
) -> Result<String, String> {
    run_codex_exec_capture_in_dir_with_attempts(dir, prompt, timeout_sec, llm_retry_count())
}

pub(crate) fn run_codex_exec_capture_in_dir_with_timeout_labeled(
    dir: &Path,
    prompt: &str,
    timeout_sec: u64,
    trace_label: &str,
    total_attempts: u32,
) -> Result<String, String> {
    run_codex_exec_capture_in_dir_with_attempts_labeled(
        dir,
        prompt,
        timeout_sec,
        total_attempts,
        Some(trace_label),
    )
}

pub(crate) fn run_codex_exec_capture_in_dir_once_with_timeout(
    dir: &Path,
    prompt: &str,
    timeout_sec: u64,
) -> Result<String, String> {
    run_codex_exec_capture_in_dir_with_attempts(dir, prompt, timeout_sec, 1)
}

pub(crate) fn run_llm_exec_capture(llm: &str, prompt: &str) -> Result<String, String> {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let prompt = normalize_exec_prompt(prompt);
    append_chat_log(&cwd, "LLM_PROMPT", &prompt);
    let timeout_sec = codex_exec_timeout_sec().max(30);
    let use_dangerous = crate::model_supports_dangerous_flag(llm);
    let total_attempts = llm_retry_count();
    let trace_label = prompt_trace_label(&prompt);
    let mut last_error = "unknown llm error".to_string();
    for attempt in 1..=total_attempts {
        let base_label = format!("{} exec [{}]", llm, trace_label);
        let attempt_label = if total_attempts > 1 {
            format!("{} attempt {}/{}", base_label, attempt, total_attempts)
        } else {
            base_label.clone()
        };
        let fallback_label = format!("{} fallback(no -y)", attempt_label);
        let start_detail = format!(
            "{} | cwd={} | timeout={}s",
            attempt_label,
            cwd.display(),
            timeout_sec
        );
        append_chat_log(&cwd, "LLM_START", &start_detail);
        let _ = crate::append_check_process_status("LLM_START", &start_detail);
        if should_use_tmux_for_llm() {
            match run_llm_via_tmux(
                &cwd,
                llm,
                &prompt,
                timeout_sec,
                true,
                use_dangerous,
                &attempt_label,
                None,
            ) {
                Ok(result) if result.success => {
                    append_chat_log(&cwd, "LLM_RESPONSE", &result.stdout);
                    return Ok(result.stdout);
                }
                Ok(result) if result.stderr.contains("unexpected argument '-y'") => {
                    match run_llm_via_tmux(
                        &cwd,
                        llm,
                        &prompt,
                        timeout_sec,
                        false,
                        use_dangerous,
                        &fallback_label,
                        None,
                    ) {
                        Ok(retry) if retry.success => {
                            append_chat_log(&cwd, "LLM_RESPONSE", &retry.stdout);
                            return Ok(retry.stdout);
                        }
                        Ok(retry) => {
                            last_error = retry.stderr;
                        }
                        Err(e) => {
                            last_error = e;
                        }
                    }
                }
                Ok(result) => {
                    last_error = result.stderr;
                }
                Err(e) => {
                    last_error = e;
                }
            }
        } else {
            let mut command = Command::new(llm);
            command.arg("exec").arg("-y");
            if use_dangerous {
                command.arg(CODEX_DANGEROUS_FLAG);
            }
            command.arg(&prompt);
            match run_command_with_timeout(
                command,
                timeout_sec,
                &attempt_label,
                &cwd,
                "waiting for llm response",
                None,
            ) {
                Ok(output) if output.status.success() => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    append_chat_log(&cwd, "LLM_RESPONSE", &stdout);
                    return Ok(stdout);
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    if stderr.contains("unexpected argument '-y'") {
                        let mut retry_command = Command::new(llm);
                        retry_command.arg("exec");
                        if use_dangerous {
                            retry_command.arg(CODEX_DANGEROUS_FLAG);
                        }
                        retry_command.arg(&prompt);
                        match run_command_with_timeout(
                            retry_command,
                            timeout_sec,
                            &fallback_label,
                            &cwd,
                            "waiting for llm response",
                            None,
                        ) {
                            Ok(retry) if retry.status.success() => {
                                let stdout = String::from_utf8_lossy(&retry.stdout).to_string();
                                append_chat_log(&cwd, "LLM_RESPONSE", &stdout);
                                return Ok(stdout);
                            }
                            Ok(retry) => {
                                last_error =
                                    String::from_utf8_lossy(&retry.stderr).trim().to_string();
                            }
                            Err(e) => {
                                last_error = e;
                            }
                        }
                    } else {
                        last_error = stderr;
                    }
                }
                Err(e) => {
                    last_error = e;
                }
            }
        }
        append_chat_log(
            &cwd,
            "LLM_RETRY",
            &format!(
                "attempt {}/{} failed: {}",
                attempt, total_attempts, last_error
            ),
        );
    }
    append_chat_log(&cwd, "LLM_ERROR", &last_error);
    Err(last_error)
}

pub(crate) async fn chat_command(_args: &[String]) -> Result<String, String> {
    Ok("chat_command placeholder".to_string())
}

pub(crate) async fn chat_wait_command(_args: &[String]) -> Result<String, String> {
    Ok("chat_wait_command placeholder".to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        latest_workspace_activity_time, progress_watch_decision, prompt_trace_label,
        trim_wait_detail, LlmProgressWatch, ProgressWatchDecision,
    };
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("mono_manager_chat_{}_{}", name, ts));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn prompt_trace_label_uses_first_non_empty_line() {
        let prompt = "\n\nadd_detail_project_code prompt\n- rule";
        assert_eq!(prompt_trace_label(prompt), "add_detail_project_code prompt");
    }

    #[test]
    fn prompt_trace_label_trims_long_first_line() {
        let long_line = format!("infer_code_spec prompt {}", "x".repeat(220));
        let expected = trim_wait_detail(&long_line);
        assert_eq!(prompt_trace_label(&long_line), expected);
    }

    #[test]
    fn latest_workspace_activity_time_skips_runtime_dirs() {
        let dir = temp_dir("activity_skip");
        fs::create_dir_all(dir.join(".project")).expect("create .project");
        fs::create_dir_all(dir.join("target")).expect("create target");
        fs::write(dir.join(".project").join("log.txt"), "ignored").expect("write .project file");
        fs::write(dir.join("target").join("build.txt"), "ignored").expect("write target file");

        assert!(latest_workspace_activity_time(&dir).is_none());

        fs::write(dir.join("src.txt"), "tracked").expect("write tracked file");
        assert!(latest_workspace_activity_time(&dir).is_some());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn progress_watch_enters_monitoring_after_soft_timeout() {
        let watch = LlmProgressWatch {
            soft_timeout_sec: 180,
            stall_timeout_sec: 120,
            hard_timeout_sec: 900,
        };
        assert_eq!(
            progress_watch_decision(watch, 180, None, None),
            ProgressWatchDecision::StartMonitoring
        );
    }

    #[test]
    fn progress_watch_detects_stall_after_monitoring() {
        let watch = LlmProgressWatch {
            soft_timeout_sec: 180,
            stall_timeout_sec: 120,
            hard_timeout_sec: 900,
        };
        assert_eq!(
            progress_watch_decision(watch, 301, Some(121), None),
            ProgressWatchDecision::Stalled
        );
    }
}
