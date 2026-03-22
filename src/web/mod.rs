use std::fs;
use std::net::TcpStream;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const WEB_HOST: &str = "127.0.0.1";
const WEB_PORT: u16 = 4175;

#[derive(Clone, Copy)]
enum WebPackageManager {
    Npm,
    Pnpm,
    Bun,
}

pub(crate) fn open_web_ui() -> Result<String, String> {
    let web_dir = resolve_web_dir()?;
    ensure_web_assets_exist(&web_dir)?;
    let web_port = resolve_web_port(&web_dir)?;
    if !is_web_server_alive(web_port) {
        clear_web_server_pid(web_port);
    }
    open_web_ui_debug(&web_dir, web_port)
}

pub(crate) fn open_web_ui_build() -> Result<String, String> {
    let web_dir = resolve_web_dir()?;
    ensure_web_assets_exist(&web_dir)?;
    let web_port = resolve_web_port(&web_dir)?;
    if !is_web_server_alive(web_port) {
        clear_web_server_pid(web_port);
    }
    open_web_ui_build_preview(&web_dir, web_port)
}

fn open_web_ui_detached(web_dir: &Path, web_port: u16) -> Result<String, String> {
    if !is_web_server_alive(web_port) {
        ensure_node_modules(web_dir)?;
        spawn_web_server_detached(web_dir, web_port)?;
        wait_for_web_server(Duration::from_secs(20), web_port)?;
    }

    let url = web_url(web_port);
    let opened = open_browser(&url);
    if opened {
        Ok(format!("web ui opened: {}", url))
    } else {
        Ok(format!("web ui ready (open manually): {}", url))
    }
}

fn open_web_ui_debug(web_dir: &Path, web_port: u16) -> Result<String, String> {
    let url = web_url(web_port);
    if is_web_server_alive(web_port) && stop_managed_web_server(web_port)? {
        println!("debug web ui: stopped existing managed server on {}", url);
    }
    if is_web_server_alive(web_port) {
        return Err(format!(
            "debug web ui needs an exclusive server on {}. stop the existing server and retry",
            url
        ));
    }

    ensure_node_modules(web_dir)?;
    let mut child = spawn_web_server_attached(web_dir, web_port)?;
    if let Err(err) = wait_for_web_server_with_child(Duration::from_secs(20), web_port, &mut child)
    {
        let _ = terminate_process(child.id());
        let _ = child.wait();
        clear_web_server_pid(web_port);
        return Err(err);
    }

    if open_browser(&url) {
        println!("web ui opened: {}", url);
    } else {
        println!("web ui ready (open manually): {}", url);
    }
    println!("debug web ui active: streaming dev-server logs, stop with Ctrl+C");

    let status = child
        .wait()
        .map_err(|e| format!("failed to wait for web dev server: {}", e))?;
    clear_web_server_pid(web_port);
    if status.success() {
        Ok(format!("web ui closed: {}", url))
    } else {
        Err(format!(
            "web ui server exited with {}",
            describe_exit_status(status)
        ))
    }
}

fn open_web_ui_build_preview(web_dir: &Path, web_port: u16) -> Result<String, String> {
    let url = web_url(web_port);
    if is_web_server_alive(web_port) && stop_managed_web_server(web_port)? {
        println!("build web ui: stopped existing managed server on {}", url);
    }
    if is_web_server_alive(web_port) {
        return Err(format!(
            "build web ui needs an exclusive server on {}. stop the existing server and retry",
            url
        ));
    }

    ensure_node_modules(web_dir)?;
    run_web_build(web_dir)?;
    let mut child = spawn_web_preview_attached(web_dir, web_port)?;
    if let Err(err) = wait_for_web_server_with_child(Duration::from_secs(20), web_port, &mut child)
    {
        let _ = terminate_process(child.id());
        let _ = child.wait();
        clear_web_server_pid(web_port);
        return Err(err);
    }

    if open_browser(&url) {
        println!("web ui opened: {}", url);
    } else {
        println!("web ui ready (open manually): {}", url);
    }
    println!("build web ui active: serving built assets, stop with Ctrl+C");

    let status = child
        .wait()
        .map_err(|e| format!("failed to wait for web preview server: {}", e))?;
    clear_web_server_pid(web_port);
    if status.success() {
        Ok(format!("web ui closed: {}", url))
    } else {
        Err(format!(
            "web ui preview server exited with {}",
            describe_exit_status(status)
        ))
    }
}

fn resolve_web_dir() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("failed to get cwd: {}", e))?;
    if let Some(path) = find_web_dir_from(&cwd) {
        return Ok(path);
    }

    let source_root = crate::source_root();
    if let Some(path) = find_web_dir_from(&source_root) {
        return Ok(path);
    }

    Err(format!("web assets not found from cwd: {}", cwd.display()))
}

fn find_web_dir_from(start: &Path) -> Option<PathBuf> {
    for ancestor in start.ancestors() {
        let candidate = ancestor.join("assets").join("web");
        if candidate.join("package.json").exists() {
            return Some(candidate);
        }
    }
    None
}

fn ensure_web_assets_exist(web_dir: &Path) -> Result<(), String> {
    let package_json = web_dir.join("package.json");
    if package_json.exists() {
        return Ok(());
    }
    Err(format!("web assets not found: {}", package_json.display()))
}

fn ensure_node_modules(web_dir: &Path) -> Result<(), String> {
    if web_dir.join("node_modules").exists() {
        return Ok(());
    }
    let manager = resolve_web_package_manager()?;
    let status = Command::new(command_for_web_package_manager(manager))
        .args(install_args_for_web_package_manager(manager))
        .current_dir(web_dir)
        .status()
        .map_err(|e| format!("failed to install web dependencies: {}", e))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "web dependency install failed with status: {:?}",
            status.code()
        ))
    }
}

fn run_web_build(web_dir: &Path) -> Result<(), String> {
    let manager = resolve_web_package_manager()?;
    let status = Command::new(command_for_web_package_manager(manager))
        .arg("run")
        .arg("build")
        .current_dir(web_dir)
        .status()
        .map_err(|e| format!("failed to execute web build: {}", e))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "web build failed with status: {:?}",
            status.code()
        ))
    }
}

fn resolve_web_port(_web_dir: &Path) -> Result<u16, String> {
    if let Ok(raw) = std::env::var("ORC_WEB_PORT") {
        let parsed = raw
            .trim()
            .parse::<u16>()
            .map_err(|_| format!("invalid ORC_WEB_PORT: {}", raw))?;
        return Ok(parsed);
    }
    Ok(WEB_PORT)
}

fn spawn_web_server_detached(web_dir: &Path, web_port: u16) -> Result<(), String> {
    let manager = resolve_web_package_manager()?;
    let mut cmd = Command::new(command_for_web_package_manager(manager));
    cmd.arg("run")
        .arg("dev")
        .arg("--")
        .arg("--host")
        .arg(WEB_HOST)
        .arg("--port")
        .arg(web_port.to_string())
        .current_dir(web_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null());
    configure_managed_child(&mut cmd);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to spawn web dev server: {}", e))?;
    if let Err(err) = write_web_server_pid(web_port, child.id()) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(err);
    }
    Ok(())
}

fn spawn_web_server_attached(web_dir: &Path, web_port: u16) -> Result<Child, String> {
    let manager = resolve_web_package_manager()?;
    let mut cmd = Command::new(command_for_web_package_manager(manager));
    cmd.arg("run")
        .arg("dev")
        .arg("--")
        .arg("--host")
        .arg(WEB_HOST)
        .arg("--port")
        .arg(web_port.to_string())
        .current_dir(web_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit());

    let child = cmd
        .spawn()
        .map_err(|e| format!("failed to spawn web dev server: {}", e))?;
    Ok(child)
}

fn spawn_web_preview_attached(web_dir: &Path, web_port: u16) -> Result<Child, String> {
    let manager = resolve_web_package_manager()?;
    let mut cmd = Command::new(command_for_web_package_manager(manager));
    cmd.arg("run")
        .arg("preview")
        .arg("--")
        .arg("--host")
        .arg(WEB_HOST)
        .arg("--port")
        .arg(web_port.to_string())
        .current_dir(web_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit());

    let child = cmd
        .spawn()
        .map_err(|e| format!("failed to spawn web preview server: {}", e))?;
    Ok(child)
}

fn resolve_web_package_manager() -> Result<WebPackageManager, String> {
    if is_command_available("npm") {
        return Ok(WebPackageManager::Npm);
    }
    if is_command_available("pnpm") {
        return Ok(WebPackageManager::Pnpm);
    }
    if is_command_available("bun") {
        return Ok(WebPackageManager::Bun);
    }
    Err("no supported package manager found for web ui (need npm, pnpm, or bun)".to_string())
}

fn is_command_available(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn command_for_web_package_manager(manager: WebPackageManager) -> &'static str {
    match manager {
        WebPackageManager::Npm => "npm",
        WebPackageManager::Pnpm => "pnpm",
        WebPackageManager::Bun => "bun",
    }
}

fn install_args_for_web_package_manager(manager: WebPackageManager) -> &'static [&'static str] {
    match manager {
        WebPackageManager::Npm => &["install"],
        WebPackageManager::Pnpm => &["install"],
        WebPackageManager::Bun => &["install"],
    }
}

fn wait_for_web_server(timeout: Duration, web_port: u16) -> Result<(), String> {
    wait_for_web_server_inner(timeout, web_port, None)
}

fn configure_managed_child(cmd: &mut Command) {
    #[cfg(unix)]
    {
        cmd.process_group(0);
    }
}

fn wait_for_web_server_with_child(
    timeout: Duration,
    web_port: u16,
    child: &mut Child,
) -> Result<(), String> {
    wait_for_web_server_inner(timeout, web_port, Some(child))
}

fn wait_for_web_server_inner(
    timeout: Duration,
    web_port: u16,
    mut child: Option<&mut Child>,
) -> Result<(), String> {
    let started = Instant::now();
    while started.elapsed() <= timeout {
        if is_web_server_alive(web_port) {
            return Ok(());
        }
        if let Some(server_child) = child.as_deref_mut() {
            if let Some(status) = server_child
                .try_wait()
                .map_err(|e| format!("failed while waiting for web dev server: {}", e))?
            {
                return Err(format!(
                    "web ui server exited before ready with {}",
                    describe_exit_status(status)
                ));
            }
        }
        thread::sleep(Duration::from_millis(250));
    }
    Err("web ui server did not become ready in time".to_string())
}

fn is_web_server_alive(web_port: u16) -> bool {
    TcpStream::connect((WEB_HOST, web_port)).is_ok()
}

fn web_url(web_port: u16) -> String {
    format!("http://{}:{}/", WEB_HOST, web_port)
}

fn web_server_pid_path(web_port: u16) -> PathBuf {
    crate::source_root()
        .join(".temp")
        .join(format!("open-ui-{}.pid", web_port))
}

fn write_web_server_pid(web_port: u16, pid: u32) -> Result<(), String> {
    let path = web_server_pid_path(web_port);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
    }
    fs::write(&path, format!("{}\n", pid))
        .map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn read_web_server_pid(web_port: u16) -> Result<Option<u32>, String> {
    let path = web_server_pid_path(web_port);
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("failed to read {}: {}", path.display(), err)),
    };
    let pid = raw
        .trim()
        .parse::<u32>()
        .map_err(|e| format!("failed to parse pid file {}: {}", path.display(), e))?;
    Ok(Some(pid))
}

fn clear_web_server_pid(web_port: u16) {
    let path = web_server_pid_path(web_port);
    if let Err(err) = fs::remove_file(&path) {
        if err.kind() != std::io::ErrorKind::NotFound {
            eprintln!("failed to remove {}: {}", path.display(), err);
        }
    }
}

fn stop_managed_web_server(web_port: u16) -> Result<bool, String> {
    let Some(pid) = read_web_server_pid(web_port)? else {
        return Ok(false);
    };
    terminate_process(pid)?;
    wait_for_port_release(Duration::from_secs(10), web_port)?;
    clear_web_server_pid(web_port);
    Ok(true)
}

fn wait_for_port_release(timeout: Duration, web_port: u16) -> Result<(), String> {
    let started = Instant::now();
    while started.elapsed() <= timeout {
        if !is_web_server_alive(web_port) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(250));
    }
    Err(format!(
        "web ui server on {} did not stop in time",
        web_url(web_port)
    ))
}

fn is_process_running(pid: u32) -> bool {
    if cfg!(windows) {
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid)])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    } else {
        Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .status()
            .is_ok_and(|status| status.success())
    }
}

fn terminate_process(pid: u32) -> Result<(), String> {
    if cfg!(windows) {
        let status = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status()
            .map_err(|e| format!("failed to execute taskkill for {}: {}", pid, e))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "taskkill failed for {} with status {:?}",
                pid,
                status.code()
            ))
        }
    } else {
        let process_group = format!("-{}", pid);
        let _ = Command::new("kill")
            .args(["-TERM", &process_group])
            .status();
        if wait_for_process_stop(pid, Duration::from_secs(3)) {
            return Ok(());
        }
        let _ = Command::new("kill")
            .args(["-KILL", &process_group])
            .status();
        if wait_for_process_stop(pid, Duration::from_secs(2)) {
            return Ok(());
        }

        let _ = Command::new("kill").arg(pid.to_string()).status();
        if wait_for_process_stop(pid, Duration::from_secs(2)) {
            Ok(())
        } else {
            Err(format!("kill failed for {}: process still running", pid))
        }
    }
}

fn wait_for_process_stop(pid: u32, timeout: Duration) -> bool {
    let started = Instant::now();
    while started.elapsed() <= timeout {
        if !is_process_running(pid) {
            return true;
        }
        thread::sleep(Duration::from_millis(150));
    }
    !is_process_running(pid)
}

fn describe_exit_status(status: ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("exit code {}", code),
        None => "termination by signal".to_string(),
    }
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

#[cfg(test)]
mod tests {
    use super::{describe_exit_status, find_web_dir_from};
    use std::fs;
    use std::process::Command;
    use tempfile::tempdir;

    #[test]
    fn finds_web_assets_from_nested_directory() {
        let dir = tempdir().expect("tempdir");
        let repo_root = dir.path().join("repo");
        let nested = repo_root.join("assets").join("presets").join("code");
        let web_dir = repo_root.join("assets").join("web");
        fs::create_dir_all(&nested).expect("nested dirs");
        fs::create_dir_all(&web_dir).expect("web dir");
        fs::write(web_dir.join("package.json"), "{}\n").expect("package.json");

        let resolved = find_web_dir_from(&nested).expect("resolved web dir");
        assert_eq!(resolved, web_dir);
    }

    #[test]
    fn returns_none_when_web_assets_do_not_exist() {
        let dir = tempdir().expect("tempdir");
        let repo_root = dir.path().join("repo");
        fs::create_dir_all(&repo_root).expect("repo root");

        assert!(find_web_dir_from(&repo_root).is_none());
    }

    #[test]
    fn exit_status_description_uses_exit_code() {
        let status = Command::new("sh")
            .arg("-c")
            .arg("exit 7")
            .status()
            .expect("exit status");

        assert_eq!(describe_exit_status(status), "exit code 7");
    }
}
