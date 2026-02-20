use anyhow::{Context, Result, bail};

pub fn open_split_and_send_message(message: &str) -> Result<()> {
    if std::env::var_os("TMUX").is_none() {
        bail!("tmux session not detected. run this command inside tmux.");
    }

    let pane_command = format!("orc show-ui --add-msg {}", quote_shell_single(message));
    let status = std::process::Command::new("tmux")
        .arg("split-window")
        .arg("-v")
        .arg("-c")
        .arg("#{pane_current_path}")
        .arg(pane_command)
        .status()
        .context("failed to execute tmux split-window")?;

    if !status.success() {
        bail!("tmux split-window failed with status: {status}");
    }
    Ok(())
}

fn quote_shell_single(raw: &str) -> String {
    let escaped = raw.replace('\'', r#"'"'"'"#);
    format!("'{escaped}'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_shell_single_escapes_single_quote() {
        assert_eq!(quote_shell_single("a'b"), "'a'\"'\"'b'");
    }
}
