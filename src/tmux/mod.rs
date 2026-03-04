use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendOption {
    Enter,
    Raw,
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
        ]);
    }
    run_tmux(&["split-window", "-h", "-P", "-F", "#{pane_id}"])
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
            "bash",
            "-lc",
            command,
        ]);
    }
    run_tmux(&["split-window", "-h", "-P", "-F", "#{pane_id}", "bash", "-lc", command])
}

pub fn send_keys(pane_id: &str, msg: &str, option: SendOption) -> Result<(), String> {
    match option {
        SendOption::Enter => {
            run_tmux(&["send-keys", "-t", pane_id, msg, "C-m"])?;
        }
        SendOption::Raw => {
            run_tmux(&["send-keys", "-t", pane_id, msg])?;
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

pub fn display_message(pane_id: &str, msg: &str) -> Result<(), String> {
    if pane_id.trim().is_empty() {
        return Ok(());
    }
    run_tmux(&["display-message", "-t", pane_id, msg])?;
    Ok(())
}

pub fn tsend(pane_id: &str, msg: &str, option: &str) -> Result<String, String> {
    let send_option = match option {
        "raw" => SendOption::Raw,
        "enter" => SendOption::Enter,
        _ => return Err("tsend option must be `enter` or `raw`".to_string()),
    };
    send_keys(pane_id, msg, send_option)?;
    Ok(format!(
        "tsend done: pane={} option={} msg={}",
        pane_id, option, msg
    ))
}
