use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant};
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
        self.worker_id
            .split('-')
            .next()
            .unwrap_or(self.worker_id.as_str())
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

fn run_tmux_optional(args: &[&str]) -> Result<Option<String>, String> {
    let output = Command::new("tmux")
        .args(args)
        .output()
        .map_err(|e| format!("failed to execute tmux: {}", e))?;
    if output.status.success() {
        return Ok(Some(String::from_utf8_lossy(&output.stdout).trim().to_string()));
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.contains("can't find pane") || stderr.contains("can't find window") {
        return Ok(None);
    }
    Err(stderr)
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

pub fn capture_pane_tail(pane_id: &str, lines: usize) -> Result<String, String> {
    let line_count = lines.max(1).to_string();
    match run_tmux_optional(&["capture-pane", "-p", "-t", pane_id, "-S", &format!("-{}", line_count)])? {
        Some(output) => Ok(output),
        None => Err(format!("pane not found: {}", pane_id)),
    }
}

pub fn wait_for_ready(
    pane_id: &str,
    pattern: &str,
    timeout_ms: u64,
    lines: usize,
) -> Result<String, String> {
    if pattern.trim().is_empty() {
        return Err("wait_for_ready requires non-empty pattern".to_string());
    }
    let deadline = Instant::now() + Duration::from_millis(timeout_ms.max(1));
    let capture_lines = lines.max(20);
    loop {
        let tail = capture_pane_tail(pane_id, capture_lines)?;
        if tail.contains(pattern) {
            return Ok(format!("wait-ready matched: pane={} pattern={}", pane_id, pattern));
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "wait-ready timeout: pane={} pattern={} tail={}",
                pane_id,
                pattern,
                tail.lines().rev().take(10).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\\n")
            ));
        }
        sleep(Duration::from_millis(250));
    }
}

pub fn http_healthcheck(url: &str, timeout_ms: u64) -> Result<String, String> {
    let timeout_secs = format!("{:.3}", (timeout_ms.max(1) as f64) / 1000.0);
    let output = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--location",
            "--max-time",
            timeout_secs.as_str(),
            "--write-out",
            " HTTP_STATUS=%{http_code}",
            url,
        ])
        .output()
        .map_err(|e| format!("failed to execute curl: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        return Err(format!("http-healthcheck failed: {} {}", url, stderr).trim().to_string());
    }
    if !stdout.contains("HTTP_STATUS=") {
        return Err(format!("http-healthcheck missing status marker: {}", url));
    }
    let status = stdout
        .rsplit_once("HTTP_STATUS=")
        .map(|(_, code)| code.trim())
        .unwrap_or_default();
    if !matches!(status.parse::<u16>(), Ok(code) if (200..400).contains(&code)) {
        return Err(format!("http-healthcheck bad status: {} {}", url, status));
    }
    Ok(format!("http-healthcheck ok: {} {}", url, status))
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
        _ => {
            return Err(
                "tsend option must be `enter`, `enter-exit`, `raw`, or `display`".to_string(),
            )
        }
    };
    send_keys(pane_id, msg, send_option)?;
    Ok(format!(
        "tsend done: pane={} option={} msg={}",
        pane_id, option, msg
    ))
}

#[cfg(test)]
mod tests {
    use super::http_healthcheck;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn http_healthcheck_accepts_200_response() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        let addr = listener.local_addr().expect("listener addr");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0_u8; 1024];
            let _ = stream.read(&mut buf);
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
                .expect("write response");
        });

        let result = http_healthcheck(&format!("http://{}", addr), 1_000);
        handle.join().expect("server thread join");
        assert!(result.is_ok(), "expected success, got {:?}", result);
    }

    #[test]
    fn http_healthcheck_rejects_500_response() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        let addr = listener.local_addr().expect("listener addr");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0_u8; 1024];
            let _ = stream.read(&mut buf);
            stream
                .write_all(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 5\r\n\r\nerror")
                .expect("write response");
        });

        let result = http_healthcheck(&format!("http://{}", addr), 1_000);
        handle.join().expect("server thread join");
        assert!(result.is_err(), "expected error, got {:?}", result);
    }
}
