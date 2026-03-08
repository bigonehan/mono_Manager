use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const WEB_PORT: u16 = 4173;
const WEB_HOST: &str = "127.0.0.1";

pub(crate) fn open_web_ui() -> Result<String, String> {
    let web_dir = resolve_web_dir()?;
    ensure_web_assets_exist(&web_dir)?;

    if !is_web_server_alive() {
        ensure_node_modules(&web_dir)?;
        spawn_web_server(&web_dir)?;
        wait_for_web_server(Duration::from_secs(20))?;
    }

    let url = format!("http://{}:{}/", WEB_HOST, WEB_PORT);
    let opened = open_browser(&url);
    if opened {
        Ok(format!("web ui opened: {}", url))
    } else {
        Ok(format!("web ui ready (open manually): {}", url))
    }
}

fn resolve_web_dir() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("failed to get cwd: {}", e))?;
    Ok(cwd.join("assets").join("web"))
}

fn ensure_web_assets_exist(web_dir: &Path) -> Result<(), String> {
    let package_json = web_dir.join("package.json");
    if package_json.exists() {
        return Ok(());
    }
    Err(format!(
        "web assets not found: {}",
        package_json.display()
    ))
}

fn ensure_node_modules(web_dir: &PathBuf) -> Result<(), String> {
    if web_dir.join("node_modules").exists() {
        return Ok(());
    }
    let status = Command::new("npm")
        .arg("install")
        .current_dir(web_dir)
        .status()
        .map_err(|e| format!("failed to execute npm install: {}", e))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("npm install failed with status: {:?}", status.code()))
    }
}

fn spawn_web_server(web_dir: &PathBuf) -> Result<(), String> {
    let mut cmd = Command::new("npm");
    cmd.arg("run")
        .arg("dev")
        .arg("--")
        .arg("--host")
        .arg(WEB_HOST)
        .arg("--port")
        .arg(WEB_PORT.to_string())
        .current_dir(web_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null());

    cmd.spawn()
        .map_err(|e| format!("failed to spawn web dev server: {}", e))?;
    Ok(())
}

fn wait_for_web_server(timeout: Duration) -> Result<(), String> {
    let started = Instant::now();
    while started.elapsed() <= timeout {
        if is_web_server_alive() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(250));
    }
    Err("web ui server did not become ready in time".to_string())
}

fn is_web_server_alive() -> bool {
    TcpStream::connect((WEB_HOST, WEB_PORT)).is_ok()
}

fn open_browser(url: &str) -> bool {
    if try_spawn("xdg-open", &[url]) {
        return true;
    }

    if try_spawn("open", &[url]) {
        return true;
    }

    if try_spawn("cmd", &["/C", "start", url]) {
        return true;
    }

    false
}

fn try_spawn(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .is_ok()
}
