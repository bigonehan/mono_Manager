use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendOption {
    Enter,
    Raw,
}

fn action_run_tmux(args: &[&str]) -> Result<String, String> {
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

pub fn action_split_window_pane() -> Result<String, String> {
    action_run_tmux(&["split-window", "-P", "-F", "#{pane_id}"])
}

pub fn action_send_keys(pane_id: &str, msg: &str, option: SendOption) -> Result<(), String> {
    match option {
        SendOption::Enter => {
            action_run_tmux(&["send-keys", "-t", pane_id, msg, "C-m"])?;
        }
        SendOption::Raw => {
            action_run_tmux(&["send-keys", "-t", pane_id, msg])?;
        }
    }
    Ok(())
}

pub fn action_current_pane_id() -> Result<String, String> {
    action_run_tmux(&["display-message", "-p", "#{pane_id}"])
}

pub fn action_rename_pane(pane_id: &str, name: &str) -> Result<(), String> {
    action_run_tmux(&["rename-pane", "-t", pane_id, name])?;
    Ok(())
}

pub fn tsend(pane_id: &str, msg: &str, option: &str) -> Result<String, String> {
    let send_option = match option {
        "raw" => SendOption::Raw,
        "enter" => SendOption::Enter,
        _ => return Err("tsend option must be `enter` or `raw`".to_string()),
    };
    action_send_keys(pane_id, msg, send_option)?;
    Ok(format!(
        "tsend done: pane={} option={} msg={}",
        pane_id, option, msg
    ))
}
