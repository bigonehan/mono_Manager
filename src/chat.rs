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
) -> Result<Output, String> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|e| format!("failed to spawn {}: {}", timeout_label, e))?;
    let started = Instant::now();
    let mut next_report_sec = LONG_WAIT_REPORT_SEC;
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
                if elapsed_sec >= next_report_sec {
                    let status = format!(
                        "[orc-status] {} | elapsed={}s | {}",
                        timeout_label, elapsed_sec, wait_reason
                    );
                    emit_owner_wait_status(project_root, None, "LLM_WAIT", &status);
                    next_report_sec += LONG_WAIT_REPORT_SEC;
                }
                if started.elapsed() >= Duration::from_secs(timeout_sec) {
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
    while !code_path.exists() {
        let elapsed_sec = started.elapsed().as_secs();
        if elapsed_sec >= next_report_sec {
            let latest = read_last_non_empty_line(&stderr_path)
                .or_else(|| read_last_non_empty_line(&stdout_path))
                .unwrap_or_else(|| "tmux worker still waiting for llm response".to_string());
            let status = format!(
                "[orc-status] {} | elapsed={}s | {}",
                timeout_label, elapsed_sec, latest
            );
            emit_owner_wait_status(dir, parent_pane.as_deref(), "LLM_WAIT", &status);
            next_report_sec += LONG_WAIT_REPORT_SEC;
        }
        if started.elapsed() >= Duration::from_secs(timeout_sec) {
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
    append_chat_log(&cwd, "LLM_PROMPT", &prompt);
    let model_bin = crate::default_model_bin();
    let dangerous = crate::model_supports_dangerous_flag(&model_bin);
    let total_attempts = llm_retry_count();
    let mut last_error = "unknown llm error".to_string();
    for attempt in 1..=total_attempts {
        if should_use_tmux_for_llm() {
            match run_llm_via_tmux(
                &cwd,
                &model_bin,
                &prompt,
                timeout_sec,
                false,
                dangerous,
                &format!("{} exec", model_bin),
            ) {
                Ok(result) => {
                    if result.success {
                        append_chat_log(&cwd, "LLM_RESPONSE", &result.stdout);
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
            command.arg("exec");
            if dangerous {
                command.arg(CODEX_DANGEROUS_FLAG);
            }
            command.arg(&prompt);
            match run_command_with_timeout(
                command,
                timeout_sec,
                &format!("{} exec", model_bin),
                &cwd,
                "waiting for llm response",
            ) {
                Ok(output) if output.status.success() => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    append_chat_log(&cwd, "LLM_RESPONSE", &stdout);
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

pub(crate) fn run_codex_exec_capture(prompt: &str) -> Result<String, String> {
    run_codex_exec_capture_with_timeout(prompt, codex_exec_timeout_sec())
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
    let prompt = normalize_exec_prompt(prompt);
    append_chat_log(dir, "LLM_PROMPT", &prompt);
    let model_bin = crate::default_model_bin();
    let dangerous = crate::model_supports_dangerous_flag(&model_bin);
    let total_attempts = total_attempts.max(1);
    let mut last_error = "unknown llm error".to_string();
    for attempt in 1..=total_attempts {
        if should_use_tmux_for_llm() {
            match run_llm_via_tmux(
                dir,
                &model_bin,
                &prompt,
                timeout_sec,
                false,
                dangerous,
                &format!("{} exec in {}", model_bin, dir.display()),
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
                &format!("{} exec in {}", model_bin, dir.display()),
                dir,
                "waiting for llm response",
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
    let mut last_error = "unknown llm error".to_string();
    for attempt in 1..=total_attempts {
        if should_use_tmux_for_llm() {
            match run_llm_via_tmux(
                &cwd,
                llm,
                &prompt,
                timeout_sec,
                true,
                use_dangerous,
                &format!("{} exec -y", llm),
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
                        &format!("{} exec", llm),
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
                &format!("{} exec -y", llm),
                &cwd,
                "waiting for llm response",
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
                            &format!("{} exec", llm),
                            &cwd,
                            "waiting for llm response",
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
