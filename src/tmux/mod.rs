use std::process::Command;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendOption {
    Enter,
    EnterExit,
    Raw,
    Display,
}

#[derive(Debug, Clone)]
pub struct WorkerPaneRef {
    pub worker_id: String,
    pub pane_id: String,
    pub pane_pid: Option<String>,
}

impl WorkerPaneRef {
    pub fn short_id(&self) -> &str {
        self.worker_id.split('-').next().unwrap_or(self.worker_id.as_str())
    }
}

fn run_tmux(args: &[&str]) -> Result<String, String> {
    let output = Command::new("tmux")
        .args(args)
        .output()
        .map_err(|e| format!("failed to execute tmux: {}", e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

const TMUX_SHELL: &str = "fish";
const TMUX_EXEC_FLAG: &str = "-ic";

pub fn split_window_pane() -> Result<String, String> {
    if let Ok(target) = current_pane_id() {
        return run_tmux(&[
            "split-window",
            "-h",
            "-t",
            target.as_str(),
            "-c",
            "#{pane_current_path}",
            "-P",
            "-F",
            "#{pane_id}",
            TMUX_SHELL,
            "-i",
        ]);
    }
    run_tmux(&[
        "split-window",
        "-h",
        "-P",
        "-F",
        "#{pane_id}",
        TMUX_SHELL,
        "-i",
    ])
}

pub fn split_window_run(command: &str) -> Result<String, String> {
    if let Ok(target) = current_pane_id() {
        return run_tmux(&[
            "split-window",
            "-h",
            "-t",
            target.as_str(),
            "-c",
            "#{pane_current_path}",
            "-P",
            "-F",
            "#{pane_id}",
            TMUX_SHELL,
            TMUX_EXEC_FLAG,
            command,
        ]);
    }
    run_tmux(&[
        "split-window",
        "-h",
        "-P",
        "-F",
        "#{pane_id}",
        TMUX_SHELL,
        TMUX_EXEC_FLAG,
        command,
    ])
}

pub fn send_keys(pane_id: &str, msg: &str, option: SendOption) -> Result<(), String> {
    match option {
        SendOption::Enter => {
            run_tmux(&["send-keys", "-t", pane_id, msg, "C-m"])?;
        }
        SendOption::EnterExit => {
            let wrapped = format!("{}; exit", msg);
            run_tmux(&["send-keys", "-t", pane_id, wrapped.as_str(), "C-m"])?;
        }
        SendOption::Raw => {
            run_tmux(&["send-keys", "-t", pane_id, msg])?;
        }
        SendOption::Display => {
            display_message(pane_id, msg)?;
        }
    }
    Ok(())
}

pub fn current_pane_id() -> Result<String, String> {
    run_tmux(&["display-message", "-p", "#{pane_id}"])
}

pub fn rename_pane(pane_id: &str, name: &str) -> Result<(), String> {
    run_tmux(&["rename-pane", "-t", pane_id, name])?;
    Ok(())
}

pub fn kill_pane(pane_id: &str) -> Result<(), String> {
    run_tmux(&["kill-pane", "-t", pane_id])?;
    Ok(())
}

pub fn pane_pid(pane_id: &str) -> Result<String, String> {
    run_tmux(&["display-message", "-p", "-t", pane_id, "#{pane_pid}"])
}

pub fn kill_pane_if_pid_matches(pane_id: &str, expected_pid: Option<&str>) -> Result<(), String> {
    if pane_id.trim().is_empty() {
        return Ok(());
    }
    if let Some(expected) = expected_pid {
        let expected = expected.trim();
        if expected.is_empty() {
            return Ok(());
        }
        let current = match pane_pid(pane_id) {
            Ok(pid) => pid,
            Err(_) => return Ok(()),
        };
        if current.trim() != expected {
            return Ok(());
        }
    }
    let _ = run_tmux(&["kill-pane", "-t", pane_id]);
    Ok(())
}

pub fn register_worker_pane(pane_id: &str) -> WorkerPaneRef {
    let pid = pane_pid(pane_id).ok().filter(|v| !v.trim().is_empty());
    WorkerPaneRef {
        worker_id: Uuid::new_v4().to_string(),
        pane_id: pane_id.to_string(),
        pane_pid: pid,
    }
}

pub fn kill_worker_pane(worker: &WorkerPaneRef) -> Result<(), String> {
    kill_pane_if_pid_matches(&worker.pane_id, worker.pane_pid.as_deref())
}

pub fn display_message(pane_id: &str, msg: &str) -> Result<(), String> {
    if pane_id.trim().is_empty() {
        return Ok(());
    }
    run_tmux(&["display-message", "-t", pane_id, msg])?;
    Ok(())
}

pub fn tsend(pane_id: &str, msg: &str, option: &str) -> Result<String, String> {
    let send_option = match option {
        "display" => SendOption::Display,
        "enter-exit" => SendOption::EnterExit,
        "raw" => SendOption::Raw,
        "enter" => SendOption::Enter,
        _ => return Err("tsend option must be `enter`, `enter-exit`, `raw`, or `display`".to_string()),
    };
    send_keys(pane_id, msg, send_option)?;
    Ok(format!(
        "tsend done: pane={} option={} msg={}",
        pane_id, option, msg
    ))
}
