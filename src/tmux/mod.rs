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

#[derive(Debug, Clone, PartialEq, Eq)]
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

    pub fn encode(&self) -> String {
        format!(
            "{}::{}::{}",
            self.worker_id,
            self.pane_id,
            self.pane_pid.clone().unwrap_or_default()
        )
    }

    pub fn decode(raw: &str) -> Result<Self, String> {
        let trimmed = raw.trim();
        let mut parts = trimmed.splitn(3, "::");
        let worker_id = parts.next().unwrap_or_default().trim();
        let pane_id = parts.next().unwrap_or_default().trim();
        let pane_pid = parts.next().unwrap_or_default().trim();
        if worker_id.is_empty() || pane_id.is_empty() {
            return Err(format!("invalid worker ref: {}", trimmed));
        }
        Ok(Self {
            worker_id: worker_id.to_string(),
            pane_id: pane_id.to_string(),
            pane_pid: if pane_pid.is_empty() {
                None
            } else {
                Some(pane_pid.to_string())
            },
        })
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
        return Ok(Some(
            String::from_utf8_lossy(&output.stdout).trim().to_string(),
        ));
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

fn resolve_send_option(option: &str) -> Result<SendOption, String> {
    match option {
        "display" => Ok(SendOption::Display),
        "enter-exit" => Ok(SendOption::EnterExit),
        "raw" => Ok(SendOption::Raw),
        "enter" => Ok(SendOption::Enter),
        _ => Err("tsend option must be `enter`, `enter-exit`, `raw`, or `display`".to_string()),
    }
}

fn send_literal(pane_id: &str, msg: &str) -> Result<(), String> {
    if msg.is_empty() {
        return Ok(());
    }
    run_tmux(&["send-keys", "-l", "-t", pane_id, msg])?;
    Ok(())
}

pub fn send_keys(pane_id: &str, msg: &str, option: SendOption) -> Result<(), String> {
    match option {
        SendOption::Enter => {
            send_literal(pane_id, msg)?;
            run_tmux(&["send-keys", "-t", pane_id, "C-m"])?;
        }
        SendOption::EnterExit => {
            let wrapped = format!("{msg}\nexit");
            send_literal(pane_id, &wrapped)?;
            run_tmux(&["send-keys", "-t", pane_id, "C-m"])?;
        }
        SendOption::Raw => {
            send_literal(pane_id, msg)?;
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

fn pane_pid_optional(pane_id: &str) -> Result<Option<String>, String> {
    run_tmux_optional(&["display-message", "-p", "-t", pane_id, "#{pane_pid}"])
}

pub fn capture_pane_tail(pane_id: &str, lines: usize) -> Result<String, String> {
    let line_count = lines.max(1).to_string();
    match run_tmux_optional(&[
        "capture-pane",
        "-p",
        "-t",
        pane_id,
        "-S",
        &format!("-{}", line_count),
    ])? {
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
    let patterns: Vec<&str> = pattern
        .split('|')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .collect();
    if patterns.is_empty() {
        return Err("wait_for_ready requires non-empty pattern".to_string());
    }
    let deadline = Instant::now() + Duration::from_millis(timeout_ms.max(1));
    let capture_lines = lines.max(20);
    loop {
        let tail = capture_pane_tail(pane_id, capture_lines)?;
        if patterns.iter().any(|candidate| tail.contains(candidate)) {
            return Ok(format!(
                "wait-ready matched: pane={} pattern={}",
                pane_id, pattern
            ));
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "wait-ready timeout: pane={} pattern={} tail={}",
                pane_id,
                pattern,
                tail.lines()
                    .rev()
                    .take(10)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<Vec<_>>()
                    .join("\\n")
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
        return Err(format!("http-healthcheck failed: {} {}", url, stderr)
            .trim()
            .to_string());
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

pub fn resolve_worker_ref(raw: &str) -> Result<WorkerPaneRef, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("worker ref must not be empty".to_string());
    }
    if trimmed.contains("::") {
        return WorkerPaneRef::decode(trimmed);
    }
    Ok(register_worker_pane(trimmed))
}

fn ensure_worker_target(worker: &WorkerPaneRef) -> Result<(), String> {
    let current_pid = pane_pid_optional(&worker.pane_id)?;
    let Some(current_pid) = current_pid else {
        return Err(format!("worker pane not found: {}", worker.pane_id));
    };
    if let Some(expected_pid) = worker.pane_pid.as_deref() {
        let expected_pid = expected_pid.trim();
        if !expected_pid.is_empty() && current_pid.trim() != expected_pid {
            return Err(format!(
                "worker pane pid mismatch: pane={} expected_pid={} actual_pid={}",
                worker.pane_id, expected_pid, current_pid
            ));
        }
    }
    Ok(())
}

pub fn worker_create() -> Result<WorkerPaneRef, String> {
    let pane_id = split_window_pane()?;
    Ok(register_worker_pane(&pane_id))
}

pub fn worker_send(worker: &WorkerPaneRef, msg: &str, option: SendOption) -> Result<(), String> {
    ensure_worker_target(worker)?;
    send_keys(&worker.pane_id, msg, option)
}

pub fn worker_wait(
    worker: &WorkerPaneRef,
    pattern: &str,
    timeout_ms: u64,
    lines: usize,
) -> Result<String, String> {
    ensure_worker_target(worker)?;
    wait_for_ready(&worker.pane_id, pattern, timeout_ms, lines)
}

fn extract_dev_url_from_tail(tail: &str) -> Option<String> {
    tail.lines().rev().find_map(|line| {
        let start = line.find("dev=http://").or_else(|| line.find("dev=https://"))?;
        let rest = &line[(start + 4)..];
        let end = rest.find(';').unwrap_or(rest.len());
        let candidate = rest[..end].trim();
        if candidate.starts_with("http://") || candidate.starts_with("https://") {
            Some(candidate.to_string())
        } else {
            None
        }
    })
}

pub fn worker_dev_url(worker: &WorkerPaneRef, lines: usize) -> Result<String, String> {
    ensure_worker_target(worker)?;
    let tail = capture_pane_tail(&worker.pane_id, lines.max(20))?;
    extract_dev_url_from_tail(&tail)
        .ok_or_else(|| format!("worker-dev-url not found: pane={}", worker.pane_id))
}

pub fn kill_worker_pane(worker: &WorkerPaneRef) -> Result<(), String> {
    kill_pane_if_pid_matches(&worker.pane_id, worker.pane_pid.as_deref())
}

pub fn worker_close(worker: &WorkerPaneRef) -> Result<(), String> {
    ensure_worker_target(worker)?;
    kill_worker_pane(worker)
}

pub fn display_message(pane_id: &str, msg: &str) -> Result<(), String> {
    if pane_id.trim().is_empty() {
        return Ok(());
    }
    run_tmux(&["display-message", "-t", pane_id, msg])?;
    Ok(())
}

pub fn tsend(pane_id: &str, msg: &str, option: &str) -> Result<String, String> {
    let send_option = resolve_send_option(option)?;
    send_keys(pane_id, msg, send_option)?;
    Ok(format!(
        "tsend done: pane={} option={} msg={}",
        pane_id, option, msg
    ))
}

#[cfg(test)]
mod tests {
    use super::{extract_dev_url_from_tail, http_healthcheck, resolve_worker_ref, WorkerPaneRef};
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

    #[test]
    fn worker_ref_round_trip_preserves_fields() {
        let worker = WorkerPaneRef {
            worker_id: "worker-123".to_string(),
            pane_id: "%42".to_string(),
            pane_pid: Some("1000".to_string()),
        };
        let encoded = worker.encode();
        let decoded = WorkerPaneRef::decode(&encoded).expect("decode");
        assert_eq!(decoded, worker);
    }

    #[test]
    fn resolve_worker_ref_accepts_encoded_and_pane_id() {
        let worker = WorkerPaneRef {
            worker_id: "worker-123".to_string(),
            pane_id: "%42".to_string(),
            pane_pid: Some("1000".to_string()),
        };
        assert_eq!(
            resolve_worker_ref(&worker.encode()).expect("encoded worker ref"),
            worker
        );
        let pane_only = resolve_worker_ref("%17").expect("pane id");
        assert_eq!(pane_only.pane_id, "%17".to_string());
        assert!(!pane_only.worker_id.trim().is_empty());
    }

    #[test]
    fn extract_dev_url_from_worker_done_tail_prefers_latest_match() {
        let tail = "\
worker:impl-test:done:dev=http://127.0.0.1:4173;report=first\n\
other log\n\
worker:impl-test:done:dev=http://127.0.0.1:4174;report=second\n";
        assert_eq!(
            extract_dev_url_from_tail(tail).as_deref(),
            Some("http://127.0.0.1:4174")
        );
    }

    #[test]
    fn extract_dev_url_from_worker_done_tail_returns_none_without_dev_marker() {
        assert!(extract_dev_url_from_tail("worker:impl:done:report=none").is_none());
    }
}
