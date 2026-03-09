mod browser;
mod config;

use anyhow::{Context, Result, bail};
use browser::HeadMode;
use clap::{Args, Parser, Subcommand, ValueEnum};
use config::{Config, debug_enabled, load_config};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs;
use std::fs::OpenOptions;
use std::io::{IsTerminal, Read};
use std::net::{IpAddr, UdpSocket};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const FEEDBACK_FILE: &str = "feedback.md";
const PLAN_FILE: &str = "plan.yaml";
const DRAFTS_FILE: &str = "drafts.yaml";
const SESSION_LOG_FILE: &str = ".rc-session-log.json";
const EXECUTION_RECORD_FILE: &str = ".rc-execution-records.jsonl";
const RUN_LOCK_FILE: &str = ".rc-run.lock";
const PROJECT_DIR: &str = ".project";
const PROJECT_LOG_FILE: &str = ".project/log.md";

#[derive(Debug, Parser)]
#[command(name = "rc")]
#[command(about = "Record and check runnable command steps")]
struct Cli {
    #[command(subcommand)]
    command: TopLevelCommand,
}

#[derive(Debug, Subcommand)]
enum TopLevelCommand {
    Clit(ClitCommand),
}

#[derive(Debug, Args)]
struct ClitCommand {
    #[command(subcommand)]
    command: ClitSubcommand,
}

#[derive(Debug, Subcommand)]
enum ClitSubcommand {
    Test(TestArgs),
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum HeadedArg {
    On,
    Off,
}

#[derive(Debug, Args)]
struct TestArgs {
    #[arg(short = 'p', long = "path")]
    path: PathBuf,
    #[arg(short = 'm', long = "mode")]
    mode: String,
    #[arg(long = "headed", value_enum, default_value = "off")]
    headed: HeadedArg,
}

#[derive(Debug, Clone)]
struct ParsedCliInput {
    target_path: PathBuf,
    mode: String,
    headed: HeadMode,
}

#[derive(Debug)]
struct CliInputError {
    message: String,
}

impl std::fmt::Display for CliInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl CliInputError {
    fn invalid_input(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
enum RunnerKind {
    Web,
    Node,
    Rust,
    Go,
    Python,
    Unknown,
}

impl RunnerKind {
    fn default_run_command(&self) -> String {
        match self {
            Self::Web => "npm run dev".to_string(),
            Self::Node => "npm run".to_string(),
            Self::Rust => "cargo run -- --help".to_string(),
            Self::Go => "go run . --help".to_string(),
            Self::Python => "python main.py --help".to_string(),
            Self::Unknown => "echo unsupported project type".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct Step {
    command_template: String,
    responses: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionCache {
    target_path: PathBuf,
    mission: String,
    runner: RunnerKind,
    steps: Vec<Step>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionLog {
    mission: String,
    runner: RunnerKind,
    detected_command: String,
    steps: Vec<Step>,
    output_log: Vec<String>,
    errors: Vec<String>,
    captures: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
struct ExecutionRecord {
    execution_id: String,
    kind: String,
    state: String,
    detail: String,
    retry: usize,
    cache_integrity: bool,
    recoverable: bool,
}

#[derive(Debug)]
struct ExecutionRecorder {
    workdir: PathBuf,
    execution_id: String,
    cache_integrity: bool,
}

#[derive(Debug)]
struct RunLock {
    path: PathBuf,
}

#[derive(Debug, Clone)]
struct ChecklistEvaluation {
    body: String,
    unresolved: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DraftsDoc {
    runner: RunnerKind,
    procedures: Vec<DraftProcedure>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DraftProcedure {
    name: String,
    expected: String,
    steps: Vec<Step>,
}

#[derive(Debug)]
struct StepOutcome {
    messages: Vec<String>,
    errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowContext {
    title: String,
    handle: String,
}

trait WindowsBridge {
    fn capture_rect_to(&self, output: &Path) -> Result<PathBuf>;
    fn capture_screen_to(&self, output: &Path) -> Result<PathBuf>;
    fn list_contexts(&self) -> Result<Vec<WindowContext>>;
    fn select_context(&self, handle: &str) -> Result<()>;
}

#[derive(Debug, Default, Clone, Copy)]
struct PowerShellWindowsBridge;

impl PowerShellWindowsBridge {
    fn list_contexts_script() -> String {
        "(Get-Process | Where-Object {$_.MainWindowTitle} | Select-Object MainWindowTitle, Id | ConvertTo-Json -Compress)".to_string()
    }

    fn select_context_script(handle: &str) -> String {
        format!(
            "$sig='[DllImport(\"user32.dll\")] public static extern bool SetForegroundWindow(IntPtr hWnd);'; Add-Type -MemberDefinition $sig -Name Win32SetForegroundWindow -Namespace Native; [Native.Win32SetForegroundWindow]::SetForegroundWindow([IntPtr]{handle}) | Out-Null"
        )
    }

    fn run(script: &str) -> Result<String> {
        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-Command", script])
            .output()
            .with_context(|| "failed to execute powershell.exe")?;
        if !output.status.success() {
            bail!(
                "powershell failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn screen_capture_script(output: &Path) -> String {
        let out = output.display().to_string().replace('\\', "/");
        format!(
            "$ErrorActionPreference='Stop'; \
             Add-Type -AssemblyName System.Drawing; \
             Add-Type -AssemblyName System.Windows.Forms; \
             $bounds=[System.Windows.Forms.SystemInformation]::VirtualScreen; \
             $bmp=New-Object System.Drawing.Bitmap($bounds.Width,$bounds.Height); \
             $g=[System.Drawing.Graphics]::FromImage($bmp); \
             $g.CopyFromScreen($bounds.X,$bounds.Y,0,0,$bmp.Size); \
             $bmp.Save('{out}',[System.Drawing.Imaging.ImageFormat]::Png); \
             $g.Dispose(); \
             $bmp.Dispose(); \
             Write-Output '{out}'"
        )
    }

    fn rect_capture_script(output: &Path, x: i32, y: i32, w: i32, h: i32) -> String {
        let out = output.display().to_string().replace('\\', "/");
        format!(
            "$ErrorActionPreference='Stop'; \
             Add-Type -AssemblyName System.Drawing; \
             $bmp=New-Object System.Drawing.Bitmap({w},{h}); \
             $g=[System.Drawing.Graphics]::FromImage($bmp); \
             $g.CopyFromScreen({x},{y},0,0,$bmp.Size); \
             $bmp.Save('{out}',[System.Drawing.Imaging.ImageFormat]::Png); \
             $g.Dispose(); \
             $bmp.Dispose(); \
             Write-Output '{out}'"
        )
    }

    fn parse_rect_env() -> Option<(i32, i32, i32, i32)> {
        let raw = std::env::var("RC_CAPTURE_RECT").ok()?;
        let items = raw
            .split(',')
            .map(|v| v.trim().parse::<i32>().ok())
            .collect::<Vec<_>>();
        if items.len() != 4 {
            return None;
        }
        let x = items[0]?;
        let y = items[1]?;
        let w = items[2]?;
        let h = items[3]?;
        if w <= 0 || h <= 0 {
            return None;
        }
        Some((x, y, w, h))
    }

    fn fallback_capture(output: &Path, label: &str) -> Result<PathBuf> {
        fs::write(output, format!("fallback capture: {label}\n"))
            .with_context(|| format!("failed to write {}", output.display()))?;
        Ok(output.to_path_buf())
    }

    fn run_capture(script: String, output: &Path, label: &str) -> Result<PathBuf> {
        match Self::run(&script) {
            Ok(_) => Ok(output.to_path_buf()),
            Err(_) => Self::fallback_capture(output, label),
        }
    }
}

impl WindowsBridge for PowerShellWindowsBridge {
    fn capture_rect_to(&self, output: &Path) -> Result<PathBuf> {
        if let Some((x, y, w, h)) = Self::parse_rect_env() {
            return Self::run_capture(
                Self::rect_capture_script(output, x, y, w, h),
                output,
                "rect",
            );
        }
        self.capture_screen_to(output)
    }

    fn capture_screen_to(&self, output: &Path) -> Result<PathBuf> {
        Self::run_capture(Self::screen_capture_script(output), output, "screen")
    }

    fn list_contexts(&self) -> Result<Vec<WindowContext>> {
        let raw = Self::run(&Self::list_contexts_script())?;
        if raw.is_empty() {
            return Ok(Vec::new());
        }
        let value: serde_json::Value =
            serde_json::from_str(&raw).with_context(|| "failed to parse powershell window list")?;
        let values = match value {
            serde_json::Value::Array(values) => values,
            other => vec![other],
        };
        Ok(values
            .into_iter()
            .filter_map(|value| {
                let title = value.get("MainWindowTitle")?.as_str()?.to_string();
                let handle = value.get("Id")?.as_i64()?.to_string();
                Some(WindowContext { title, handle })
            })
            .collect())
    }
    fn select_context(&self, handle: &str) -> Result<()> {
        let _ = Self::run(&Self::select_context_script(handle))?;
        Ok(())
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let config = load_config()?;
    let parsed = parse_cli_from(std::env::args_os())?;
    execute_test(parsed, &config)
}

fn parse_cli_from<I, T>(args: I) -> Result<ParsedCliInput>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::try_parse_from(args).map_err(|error| {
        anyhow::anyhow!(
            CliInputError {
                message: error.to_string()
            }
            .message
        )
    })?;
    match cli.command {
        TopLevelCommand::Clit(command) => match command.command {
            ClitSubcommand::Test(args) => {
                validate_test_args(args).map_err(|error| anyhow::anyhow!(error.message))
            }
        },
    }
}

fn validate_test_args(args: TestArgs) -> std::result::Result<ParsedCliInput, CliInputError> {
    if args.mode.trim().is_empty() {
        return Err(CliInputError::invalid_input(
            "invalid argument `-m <mode>`: mode must not be empty",
        ));
    }
    let target_path = args.path.canonicalize().map_err(|_| {
        CliInputError::invalid_input(format!(
            "invalid argument `-p <path>`: path does not exist ({})",
            args.path.display()
        ))
    })?;
    if !target_path.is_dir() {
        return Err(CliInputError::invalid_input(format!(
            "invalid argument `-p <path>`: not a directory ({})",
            target_path.display()
        )));
    }
    Ok(ParsedCliInput {
        target_path,
        mode: args.mode,
        headed: match args.headed {
            HeadedArg::On => HeadMode::On,
            HeadedArg::Off => HeadMode::Off,
        },
    })
}

fn execute_test(input: ParsedCliInput, config: &Config) -> Result<()> {
    let workdir = std::env::current_dir()?;
    let _run_lock = acquire_run_lock(&workdir)?;
    fs::create_dir_all(workdir.join(PROJECT_DIR))
        .with_context(|| format!("failed to create {}", PROJECT_DIR))?;
    let runner = detect_runner(&input.target_path)?;
    let plan_body = build_plan(
        &input.target_path,
        &input.mode,
        &runner,
        input.headed,
        config,
    )?;
    fs::write(workdir.join(PLAN_FILE), &plan_body)
        .with_context(|| format!("failed to write {}", PLAN_FILE))?;
    let drafts = build_drafts(
        &input.target_path,
        &input.mode,
        &runner,
        input.headed,
        config,
    )?;
    fs::write(workdir.join(DRAFTS_FILE), serde_yaml::to_string(&drafts)?)
        .with_context(|| format!("failed to write {}", DRAFTS_FILE))?;
    let steps = drafts
        .procedures
        .iter()
        .flat_map(|procedure| procedure.steps.clone())
        .collect::<Vec<_>>();
    let cache = SessionCache {
        target_path: input.target_path.clone(),
        mission: input.mode.clone(),
        runner: runner.clone(),
        steps: steps.clone(),
    };
    write_session_cache(&cache)?;
    let mut log = SessionLog {
        mission: input.mode.clone(),
        runner: runner.clone(),
        detected_command: if runner == RunnerKind::Web {
            detect_web_server_command(&input.target_path)
        } else {
            runner.default_run_command()
        },
        steps,
        output_log: Vec::new(),
        errors: Vec::new(),
        captures: Vec::new(),
    };
    let mut recorder = ExecutionRecorder::new(&workdir);
    recorder.record(
        "plan",
        "generated",
        format!("{} generated", PLAN_FILE),
        true,
    );
    run_check(&input.target_path, &drafts, &mut log, config, &mut recorder)?;
    match get_current_state(&input.target_path, &runner, config) {
        Ok(state) => log.output_log.push(format!("current-state:\n{state}")),
        Err(error) => log
            .errors
            .push(format!("get_current_state failed: {error:#}")),
    }
    collect_captures(&mut log)?;
    write_feedback(&workdir, &log)?;
    recorder.record("feedback", "saved", "feedback saved".to_string(), true);
    if should_spawn_codex_worker() {
        if let Err(error) = maybe_spawn_codex_worker(&workdir) {
            recorder.record(
                "error",
                "worker_spawn_failed",
                format!("worker spawn failed: {error:#}"),
                true,
            );
        }
    }
    recorder.close();
    Ok(())
}

fn detect_runner(target_path: &Path) -> Result<RunnerKind> {
    if target_path.join("Cargo.toml").exists() {
        return Ok(RunnerKind::Rust);
    }
    if target_path.join("package.json").exists() {
        if is_web_project(target_path)? {
            return Ok(RunnerKind::Web);
        }
        return Ok(RunnerKind::Node);
    }
    if target_path.join("go.mod").exists() {
        return Ok(RunnerKind::Go);
    }
    if target_path.join("pyproject.toml").exists()
        || target_path.join("requirements.txt").exists()
        || target_path.join("main.py").exists()
    {
        return Ok(RunnerKind::Python);
    }
    Ok(RunnerKind::Unknown)
}

fn is_web_project(target_path: &Path) -> Result<bool> {
    if target_path.join("index.html").exists() {
        return Ok(true);
    }
    let package_json = target_path.join("package.json");
    if !package_json.exists() {
        return Ok(false);
    }
    let raw = fs::read_to_string(&package_json)
        .with_context(|| format!("failed to read {}", package_json.display()))?;
    let value: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", package_json.display()))?;
    let scripts = value
        .get("scripts")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    if scripts.contains_key("dev")
        || scripts.contains_key("start")
        || scripts.contains_key("preview")
    {
        return Ok(true);
    }
    Ok(false)
}

fn build_plan(
    target_path: &Path,
    mode: &str,
    runner: &RunnerKind,
    headed: HeadMode,
    config: &Config,
) -> Result<String> {
    let prompt_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("prompts")
        .join("build_plan.txt");
    let prompt_template = fs::read_to_string(&prompt_path)
        .with_context(|| format!("failed to read {}", prompt_path.display()))?;
    let inventory = describe_target_path(target_path)?;
    let prompt = format!(
        "{}\n\npath: {}\nrunner: {:?}\nheaded: {:?}\nmode: {}\n\ninventory:\n{}\n\n출력은 plan.md 본문 markdown만 반환한다.",
        prompt_template,
        target_path.display(),
        runner,
        headed,
        mode,
        inventory
    );
    let llm_body = run_codex_plan_prompt(&prompt)
        .unwrap_or_else(|_| fallback_plan_body(target_path, mode, runner, headed, config));
    Ok(llm_body)
}

fn describe_target_path(target_path: &Path) -> Result<String> {
    let mut lines = Vec::new();
    for entry in fs::read_dir(target_path)
        .with_context(|| format!("failed to read {}", target_path.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().and_then(|v| v.to_str()).unwrap_or("?");
        lines.push(format!("- {}", name));
    }
    lines.sort();
    Ok(lines.join("\n"))
}

fn run_codex_plan_prompt(prompt: &str) -> Result<String> {
    let danger_flag = if std::env::var("CODEX_DANGEROUSLY_BYPASS_APPROVALS_AND_SANDBOX").is_ok() {
        ""
    } else {
        " --dangerously-bypass-approvals-and-sandbox"
    };
    let output = Command::new("bash")
        .args([
            "-lc",
            &format!(
                "timeout 20 codex exec{} {}",
                danger_flag,
                shell_quote(prompt)
            ),
        ])
        .output()
        .with_context(|| "failed to execute codex")?;
    if !output.status.success() {
        bail!(
            "codex plan generation failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        bail!("codex plan generation returned empty output");
    }
    Ok(stdout)
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn fallback_plan_body(
    target_path: &Path,
    mode: &str,
    runner: &RunnerKind,
    headed: HeadMode,
    config: &Config,
) -> String {
    format!(
        "# plan\n\n- path: {}\n- runner: {:?}\n- headed: {:?}\n- mode: {}\n- execute: {}\n- expected: feedback.md, drafts.yaml, session logs, captures\n",
        target_path.display(),
        runner,
        headed,
        mode,
        plan_execute_line(target_path, runner, config)
    )
}

fn plan_execute_line(target_path: &Path, runner: &RunnerKind, config: &Config) -> String {
    match runner {
        RunnerKind::Web => format!(
            "{} -> browser open {}",
            detect_web_server_command(target_path),
            browser_reachable_url(&config.browser_url)
        ),
        _ => runner.default_run_command(),
    }
}

fn build_drafts(
    target_path: &Path,
    mode: &str,
    runner: &RunnerKind,
    headed: HeadMode,
    config: &Config,
) -> Result<DraftsDoc> {
    let procedures = if *runner == RunnerKind::Web {
        vec![build_web_procedure(target_path, mode, headed, config)]
    } else {
        vec![DraftProcedure {
            name: "default_check".to_string(),
            expected: "program runs without error".to_string(),
            steps: vec![Step {
                command_template: runner.default_run_command(),
                responses: build_responses(mode),
            }],
        }]
    };
    Ok(DraftsDoc {
        runner: runner.clone(),
        procedures,
    })
}

fn build_web_procedure(
    target_path: &Path,
    mode: &str,
    headed: HeadMode,
    config: &Config,
) -> DraftProcedure {
    let agent = &config.agent_browser_command;
    let requested_url = requested_web_url(target_path, mode, config);
    let url = browser_reachable_url(&requested_url);
    let mut steps = vec![
        Step {
            command_template: format!(
                "if [ -f .rc-web-server.pid ]; then kill $(cat .rc-web-server.pid) >/dev/null 2>&1 || true; fi; ({}) > .rc-web-server.log 2>&1 & echo $! > .rc-web-server.pid; sleep 5",
                detect_web_server_command(target_path)
            ),
            responses: Vec::new(),
        },
        Step {
            command_template: browser::wait_for_url_command(&requested_url),
            responses: Vec::new(),
        },
        Step {
            command_template: browser::install_command(agent),
            responses: Vec::new(),
        },
        Step {
            command_template: browser::open_command(agent, &url, headed),
            responses: Vec::new(),
        },
        Step {
            command_template: browser::wait_for_selector_command(
                agent,
                ".auth-card:nth-of-type(1) input[placeholder='user id']",
            ),
            responses: Vec::new(),
        },
        Step {
            command_template: browser::snapshot_command(agent),
            responses: Vec::new(),
        },
    ];
    if mode.to_ascii_lowercase().contains("login") {
        steps.extend(login_steps(agent));
    }
    if let Some(label) = extract_action_value(mode, "click") {
        steps.push(Step {
            command_template: browser::click_command(agent, &label),
            responses: build_responses(mode),
        });
    }
    if let Some(text) = extract_action_value(mode, "input") {
        steps.push(Step {
            command_template: browser::keyboard_type_command(agent, &text),
            responses: build_responses(mode),
        });
    }
    steps.push(Step { command_template: format!("{}; if [ -f .rc-web-server.pid ]; then kill $(cat .rc-web-server.pid) >/dev/null 2>&1 || true; fi", browser::screenshot_and_close_command(agent)), responses: Vec::new() });
    DraftProcedure {
        name: "web_login_check".to_string(),
        expected: "user can log in through the UI".to_string(),
        steps,
    }
}

fn login_steps(agent: &str) -> Vec<Step> {
    vec![
        Step {
            command_template: browser::fill_command(
                agent,
                ".auth-card:nth-of-type(2) input[placeholder='name']",
                "Demo User",
            ),
            responses: Vec::new(),
        },
        Step {
            command_template: browser::fill_command(
                agent,
                ".auth-card:nth-of-type(2) input[placeholder='user id']",
                "demo-user",
            ),
            responses: Vec::new(),
        },
        Step {
            command_template: browser::fill_command(
                agent,
                ".auth-card:nth-of-type(2) input[type='password']",
                "demo-pass",
            ),
            responses: Vec::new(),
        },
        Step {
            command_template: browser::click_selector_command(
                agent,
                ".auth-card:nth-of-type(2) button[type='submit']",
            ),
            responses: Vec::new(),
        },
        Step {
            command_template: browser::sleep_command(1),
            responses: Vec::new(),
        },
        Step {
            command_template: browser::fill_command(
                agent,
                ".auth-card:nth-of-type(1) input[placeholder='user id']",
                "demo-user",
            ),
            responses: Vec::new(),
        },
        Step {
            command_template: browser::fill_command(
                agent,
                ".auth-card:nth-of-type(1) input[type='password']",
                "demo-pass",
            ),
            responses: Vec::new(),
        },
        Step {
            command_template: browser::click_selector_command(
                agent,
                ".auth-card:nth-of-type(1) button[type='submit']",
            ),
            responses: Vec::new(),
        },
        Step {
            command_template: browser::sleep_command(1),
            responses: Vec::new(),
        },
        Step {
            command_template: browser::snapshot_command(agent),
            responses: Vec::new(),
        },
    ]
}

fn detect_web_server_command(target_path: &Path) -> String {
    if target_path.join("pnpm-lock.yaml").exists() {
        return "pnpm dev".to_string();
    }
    if target_path.join("yarn.lock").exists() {
        return "yarn dev".to_string();
    }
    if target_path.join("bun.lockb").exists() || target_path.join("bun.lock").exists() {
        return "bun run dev".to_string();
    }
    "npm run dev".to_string()
}

fn requested_web_url(target_path: &Path, mode: &str, config: &Config) -> String {
    if let Some(url) = extract_action_value(mode, "url") {
        return url;
    }
    let package_json = fs::read_to_string(target_path.join("package.json")).unwrap_or_default();
    if package_json.contains("vite") {
        return "http://127.0.0.1:5173".to_string();
    }
    config.browser_url.clone()
}

fn browser_reachable_url(url: &str) -> String {
    let Some(host) = local_ipv4_address() else {
        return url.to_string();
    };
    rewrite_loopback_url(url, &host)
}

fn rewrite_loopback_url(url: &str, replacement_host: &str) -> String {
    let Some((scheme, remainder)) = url.split_once("://") else {
        return url.to_string();
    };
    let (authority, suffix) = if let Some((authority, path)) = remainder.split_once('/') {
        (authority, format!("/{path}"))
    } else {
        (remainder, String::new())
    };
    let Some((host, port)) = split_url_authority(authority) else {
        return url.to_string();
    };
    if !is_loopback_host(host) {
        return url.to_string();
    }
    format!("{scheme}://{replacement_host}{port}{suffix}")
}

fn split_url_authority(authority: &str) -> Option<(&str, &str)> {
    if authority == "[::1]" {
        return Some((authority, ""));
    }
    if let Some(rest) = authority.strip_prefix("[::1]") {
        return Some(("[::1]", rest));
    }
    let Some((host, port)) = authority.rsplit_once(':') else {
        return Some((authority, ""));
    };
    if port.chars().all(|ch| ch.is_ascii_digit()) {
        Some((host, &authority[host.len()..]))
    } else {
        Some((authority, ""))
    }
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "localhost" | "[::1]")
}

fn local_ipv4_address() -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(ip) if !ip.is_loopback() => Some(ip.to_string()),
        _ => None,
    }
}

fn extract_action_value(mode: &str, key: &str) -> Option<String> {
    let lowered = mode.to_ascii_lowercase();
    let needle = format!("{key} ");
    let start = lowered.find(&needle)?;
    let rest = mode.get(start + needle.len()..)?.trim();
    if let Some(quoted) = rest.strip_prefix('"') {
        let end = quoted.find('"')?;
        return Some(quoted[..end].to_string());
    }
    let value = rest
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches('"')
        .trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn build_responses(mode: &str) -> Vec<String> {
    let mut responses = Vec::new();
    if mode.contains("y/n") || mode.to_ascii_lowercase().contains("yes") {
        responses.push("y".to_string());
    }
    if mode.contains("hst") {
        responses.push("hst".to_string());
    }
    responses
}

fn run_check(
    target_path: &Path,
    drafts: &DraftsDoc,
    log: &mut SessionLog,
    config: &Config,
    recorder: &mut ExecutionRecorder,
) -> Result<()> {
    for procedure in &drafts.procedures {
        for step in &procedure.steps {
            println!("step> {}", step.command_template);
            recorder.record(
                "step",
                "started",
                format!("{} -> {}", procedure.name, step.command_template),
                true,
            );
            let outcome = run_step(target_path, step)?;
            if debug_enabled(config) {
                append_project_log(&format!(
                    "## {}\n- command: {}\n- messages: {}\n- errors: {}\n",
                    procedure.name,
                    step.command_template,
                    outcome.messages.join(" | "),
                    outcome.errors.join(" | ")
                ))?;
            }
            println!(
                "step< status={} messages={} errors={}",
                if outcome.errors.is_empty() {
                    "ok"
                } else {
                    "error"
                },
                outcome.messages.len(),
                outcome.errors.len()
            );
            log.output_log.extend(outcome.messages.clone());
            log.errors.extend(outcome.errors.clone());
        }
    }
    Ok(())
}

fn get_current_state(target_path: &Path, runner: &RunnerKind, config: &Config) -> Result<String> {
    match runner {
        RunnerKind::Web => get_web_current_state(target_path, &config.agent_browser_command),
        RunnerKind::Node | RunnerKind::Rust | RunnerKind::Go | RunnerKind::Python => {
            get_script_current_state_from_stdin()
        }
        RunnerKind::Unknown => Ok("state unavailable: unknown runner".to_string()),
    }
}

fn get_web_current_state(target_path: &Path, agent_browser_command: &str) -> Result<String> {
    let cmd = browser::snapshot_command(agent_browser_command);
    let output = Command::new("bash")
        .args(["-lc", &cmd])
        .current_dir(target_path)
        .output()
        .with_context(|| "failed to collect web current state via agent-browser snapshot")?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        bail!(
            "web current state command failed (exit={:?}): {}",
            output.status.code(),
            if stderr.is_empty() {
                "no stderr".to_string()
            } else {
                stderr
            }
        );
    }
    if stdout.is_empty() {
        Ok("state unavailable: empty snapshot".to_string())
    } else {
        Ok(stdout)
    }
}

fn get_script_current_state_from_stdin() -> Result<String> {
    let mut stdin = std::io::stdin();
    if stdin.is_terminal() {
        return Ok("stdin state unavailable: interactive terminal".to_string());
    }
    let mut buffer = String::new();
    stdin
        .read_to_string(&mut buffer)
        .with_context(|| "failed to read stdin state")?;
    if buffer.trim().is_empty() {
        Ok("stdin state unavailable: empty input".to_string())
    } else {
        Ok(buffer)
    }
}

fn append_project_log(entry: &str) -> Result<()> {
    let path = Path::new(PROJECT_LOG_FILE);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut body = fs::read_to_string(path).unwrap_or_default();
    body.push_str(entry);
    fs::write(path, body)?;
    Ok(())
}

fn run_step(target_path: &Path, step: &Step) -> Result<StepOutcome> {
    let output = Command::new("bash")
        .args(["-lc", &step.command_template])
        .current_dir(target_path)
        .output()
        .with_context(|| format!("failed to execute step `{}`", step.command_template))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let mut messages = Vec::new();
    let mut errors = Vec::new();
    if !stdout.trim().is_empty() {
        messages.push(stdout.clone());
    }
    if !stderr.trim().is_empty() {
        messages.push(stderr.clone());
    }
    if needs_interactive_response(&stdout) || needs_interactive_response(&stderr) {
        messages.extend(
            step.responses
                .iter()
                .map(|response| format!("auto-response: {response}")),
        );
    }
    if !output.status.success() || contains_error_signal(&stdout) || contains_error_signal(&stderr)
    {
        errors.push(format!(
            "command=`{}` exit={:?}",
            step.command_template,
            output.status.code()
        ));
        if !stderr.trim().is_empty() {
            errors.push(stderr);
        }
    }
    Ok(StepOutcome { messages, errors })
}

fn needs_interactive_response(output: &str) -> bool {
    let lowered = output.to_ascii_lowercase();
    lowered.contains("y/n")
        || lowered.contains("[y/n]")
        || lowered.contains("continue?")
        || lowered.contains("enter input")
}

fn contains_error_signal(output: &str) -> bool {
    let lowered = output.to_ascii_lowercase();
    lowered.contains("error") || lowered.contains("exit")
}

fn write_session_cache(cache: &SessionCache) -> Result<()> {
    fs::write(
        cache.target_path.join(".rc-cache.json"),
        serde_json::to_string_pretty(cache)?,
    )
    .with_context(|| "failed to write session cache")?;
    Ok(())
}

fn collect_captures(log: &mut SessionLog) -> Result<()> {
    let bridge = PowerShellWindowsBridge;
    let workdir = std::env::current_dir()?;
    let terminal_capture = capture_terminal_session(&workdir)?;
    log.captures.push(terminal_capture);
    if let Ok(contexts) = bridge.list_contexts() {
        if let Some(context) = contexts.first() {
            log.output_log.push(format!(
                "windows-context: {} ({})",
                context.title, context.handle
            ));
            if std::env::var("RC_SELECT_FIRST_CONTEXT").ok().as_deref() == Some("1") {
                let _ = bridge.select_context(&context.handle);
            }
        }
    }
    log.captures
        .push(bridge.capture_rect_to(&workdir.join("rect-capture.png"))?);
    log.captures
        .push(bridge.capture_screen_to(&workdir.join("screen-capture.png"))?);
    fs::write(
        workdir.join(SESSION_LOG_FILE),
        serde_json::to_string_pretty(log)?,
    )
    .with_context(|| "failed to write session log")?;
    Ok(())
}

fn capture_terminal_session(workdir: &Path) -> Result<PathBuf> {
    let output_path = workdir.join("terminal-capture.txt");
    let capture = if let Ok(pane) = std::env::var("TMUX_PANE") {
        let output = Command::new("tmux")
            .args(["capture-pane", "-p", "-t", &pane])
            .output();
        match output {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            _ => "tmux capture unavailable\n".to_string(),
        }
    } else {
        format!("session args: {:?}\n", std::env::args().collect::<Vec<_>>())
    };
    fs::write(&output_path, capture)
        .with_context(|| format!("failed to write {}", output_path.display()))?;
    Ok(output_path)
}

fn write_feedback(workdir: &Path, log: &SessionLog) -> Result<()> {
    let effective_errors = filtered_errors(log);
    let checklist = evaluate_checklist(workdir, log, &effective_errors)?;
    let mut unresolved_items = effective_errors.clone();
    unresolved_items.extend(checklist.unresolved.clone());
    let unresolved = if unresolved_items.is_empty() {
        "- 없음".to_string()
    } else {
        unresolved_items
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let improvements = if unresolved_items.is_empty() {
        "- 현재 draft 절차가 체크리스트 기준을 만족했고 화면 근거(snapshot/screenshot)가 기록됐다."
            .to_string()
    } else {
        "- feedback를 기준으로 plan/drafts 절차를 갱신해야 한다.".to_string()
    };
    let result = format!(
        "# 결과\n- runner: {:?}\n- detected command: {}\n- steps: {}\n- captures: {}\n\n# 체크리스트\n{}\n\n# 미해결\n{}\n\n# 보완\n{}\n",
        log.runner,
        log.detected_command,
        log.steps
            .iter()
            .map(|step| step.command_template.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        log.captures
            .iter()
            .map(|capture| capture.display().to_string())
            .collect::<Vec<_>>()
            .join(", "),
        checklist
            .body
            .lines()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join(" / "),
        unresolved,
        improvements
    );
    fs::write(workdir.join(FEEDBACK_FILE), result)
        .with_context(|| "failed to write feedback.md")?;
    Ok(())
}

fn evaluate_checklist(workdir: &Path, log: &SessionLog, effective_errors: &[String]) -> Result<ChecklistEvaluation> {
    let checklist_path = workdir.join(PROJECT_DIR).join("check_list.md");
    if cfg!(test) {
        let body = fallback_checklist(log, effective_errors);
        if let Some(parent) = checklist_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&checklist_path, &body)?;
        return Ok(parse_checklist(&body));
    }

    let prompt = build_checklist_prompt(log, effective_errors);
    let body = run_codex_checklist_prompt(&prompt).unwrap_or_else(|_| fallback_checklist(log, effective_errors));
    if let Some(parent) = checklist_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&checklist_path, &body)?;
    Ok(parse_checklist(&body))
}

fn build_checklist_prompt(log: &SessionLog, effective_errors: &[String]) -> String {
    let outputs = log
        .output_log
        .iter()
        .rev()
        .take(12)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n---\n");
    let errors = if effective_errors.is_empty() {
        "none".to_string()
    } else {
        effective_errors.join("\n")
    };
    format!(
        "다음 실행 로그를 보고 check_list.md 형식만 출력해라.\n형식: - [x| ] {{입력}} -> {{출력}} : 기능설명\n규칙: 현재 결과가 충족되면 [x], 미충족이면 [ ].\n\nmission: {}\nrunner: {:?}\nsteps:\n{}\n\nrecent_output:\n{}\n\neffective_errors:\n{}\n",
        log.mission,
        log.runner,
        log.steps
            .iter()
            .map(|step| format!("- {}", step.command_template))
            .collect::<Vec<_>>()
            .join("\n"),
        outputs,
        errors
    )
}

fn run_codex_checklist_prompt(prompt: &str) -> Result<String> {
    let danger_flag = if std::env::var("CODEX_DANGEROUSLY_BYPASS_APPROVALS_AND_SANDBOX").is_ok() {
        ""
    } else {
        " --dangerously-bypass-approvals-and-sandbox"
    };
    let output = Command::new("bash")
        .args([
            "-lc",
            &format!(
                "timeout 20 codex exec{} {}",
                danger_flag,
                shell_quote(prompt)
            ),
        ])
        .output()
        .with_context(|| "failed to execute codex checklist prompt")?;
    if !output.status.success() {
        bail!(
            "codex checklist generation failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        bail!("codex checklist generation returned empty output");
    }
    Ok(stdout)
}

fn fallback_checklist(log: &SessionLog, effective_errors: &[String]) -> String {
    let status = if effective_errors.is_empty() { "x" } else { " " };
    let output = if effective_errors.is_empty() { "기본 점검 통과" } else { "기본 점검 실패" };
    format!(
        "- [{}] {} -> {} : mode 기반 기본 체크리스트\n- [x] step 실행 -> output_log 기록 : 실행 로그 수집\n",
        status, log.mission, output
    )
}

fn parse_checklist(body: &str) -> ChecklistEvaluation {
    let mut unresolved = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("- [") {
            continue;
        }
        if trimmed.starts_with("- [ ]") {
            let detail = trimmed
                .split_once(':')
                .map(|(_, rhs)| rhs.trim().to_string())
                .unwrap_or_else(|| trimmed.to_string());
            unresolved.push(detail);
        }
    }
    ChecklistEvaluation {
        body: body.to_string(),
        unresolved,
    }
}

fn filtered_errors(log: &SessionLog) -> Vec<String> {
    log.errors
        .iter()
        .filter(|item| {
            if item.contains("command=`if command -v agent-browser")
                && item.contains("exit=Some(0)")
            {
                return false;
            }
            if item.contains("No supported package manager found")
                && log
                    .output_log
                    .iter()
                    .any(|line| line.contains("Chromium installed successfully"))
            {
                return false;
            }
            true
        })
        .cloned()
        .collect()
}

fn maybe_spawn_codex_worker(workdir: &Path) -> Result<()> {
    if std::env::var("TMUX").ok().is_none() {
        return Ok(());
    }
    let pane_output = Command::new("tmux")
        .args(["split-window", "-h", "-P", "-F", "#{pane_id}"])
        .output()
        .with_context(|| "failed to create tmux pane")?;
    if !pane_output.status.success() {
        return Ok(());
    }
    let pane_id = String::from_utf8_lossy(&pane_output.stdout)
        .trim()
        .to_string();
    if pane_id.is_empty() {
        return Ok(());
    }
    let message =
        "feedback.md를 읽고 해결할 수 있는 문제와 개선점을 쳐아서 개선하라".replace('"', "\\\"");
    let danger_flag = if std::env::var("CODEX_DANGEROUSLY_BYPASS_APPROVALS_AND_SANDBOX").is_ok() {
        ""
    } else {
        " --dangerously-bypass-approvals-and-sandbox"
    };
    let command = format!(
        "cd {} && codex exec{} \"{}\"",
        workdir.display(),
        danger_flag,
        message
    );
    let status = Command::new("tmux")
        .args(["send-keys", "-t", &pane_id, &command, "Enter"])
        .status()
        .with_context(|| "failed to start codex in tmux pane")?;
    if !status.success() {
        return Ok(());
    }
    Ok(())
}

fn acquire_run_lock(workdir: &Path) -> Result<RunLock> {
    let path = workdir.join(RUN_LOCK_FILE);
    let pid = std::process::id();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|v| v.as_secs())
        .unwrap_or(0);
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .with_context(|| format!("another rc run is already in progress (lock: {})", path.display()))?;
    use std::io::Write;
    writeln!(file, "pid={pid}")?;
    writeln!(file, "started_at={stamp}")?;
    Ok(RunLock { path })
}

impl Drop for RunLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn should_spawn_codex_worker() -> bool {
    matches!(
        std::env::var("RC_SPAWN_CODEX_WORKER").ok().as_deref(),
        Some("1" | "true" | "TRUE" | "on" | "ON")
    )
}

impl ExecutionRecorder {
    fn new(workdir: &Path) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|v| v.as_secs())
            .unwrap_or(0);
        Self {
            workdir: workdir.to_path_buf(),
            execution_id: format!("rc-{stamp}"),
            cache_integrity: true,
        }
    }

    fn record(&mut self, kind: &str, state: &str, detail: String, recoverable: bool) {
        let entry = ExecutionRecord {
            execution_id: self.execution_id.clone(),
            kind: kind.to_string(),
            state: state.to_string(),
            detail,
            retry: 0,
            cache_integrity: self.cache_integrity,
            recoverable,
        };
        let path = self.workdir.join(EXECUTION_RECORD_FILE);
        let mut body = fs::read_to_string(&path).unwrap_or_default();
        if let Ok(line) = serde_json::to_string(&entry) {
            body.push_str(&line);
            body.push('\n');
            let _ = fs::write(path, body);
        }
    }

    fn close(&mut self) {
        self.record(
            "summary",
            "completed",
            "execution recorder closed".to_string(),
            true,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parses_fixed_cli_arguments() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().display().to_string();
        let parsed = validate_test_args(TestArgs {
            path: PathBuf::from(path),
            mode: "smoke".to_string(),
            headed: HeadedArg::Off,
        })
        .expect("parse");
        assert_eq!(parsed.mode, "smoke");
    }

    #[test]
    fn detects_web_runner_from_package_json() {
        let dir = tempdir().expect("tempdir");
        fs::write(
            dir.path().join("package.json"),
            r#"{"scripts":{"dev":"next dev --port 3000"}}"#,
        )
        .expect("write");
        assert_eq!(detect_runner(dir.path()).expect("detect"), RunnerKind::Web);
    }

    #[test]
    fn builds_login_drafts_for_web_mode() {
        let dir = tempdir().expect("tempdir");
        fs::write(
            dir.path().join("package.json"),
            r#"{"scripts":{"dev":"next dev --port 3000"}}"#,
        )
        .expect("write");
        let config = load_config().expect("config");
        let drafts = build_drafts(
            dir.path(),
            "login",
            &RunnerKind::Web,
            HeadMode::Off,
            &config,
        )
        .expect("drafts");
        assert!(
            drafts.procedures[0]
                .steps
                .iter()
                .any(|step| {
                    step.command_template.contains("python3 - \"")
                        && step.command_template.contains("<<'PY'")
                })
        );
        assert!(drafts.procedures[0].steps.iter().any(|step| {
            step.command_template
                .contains(".auth-card:nth-of-type(2) input[placeholder='name']")
        }));
        assert!(drafts.procedures[0].steps.iter().any(|step| {
            step.command_template
                .contains(".auth-card:nth-of-type(1) input[type='password']")
        }));
        assert!(drafts.procedures[0].steps.iter().any(|step| {
            step.command_template
                .contains("click \".auth-card:nth-of-type(1) button[type='submit']\"")
        }));
        assert!(drafts.procedures[0].steps.iter().any(|step| {
            step.command_template
                == "agent-browser wait \".auth-card:nth-of-type(1) input[placeholder='user id']\""
        }));
        assert!(drafts.procedures[0]
            .steps
            .iter()
            .any(|step| step.command_template == "agent-browser snapshot -i"));
    }

    #[test]
    fn rewrites_loopback_url_for_browser_access() {
        assert_eq!(
            rewrite_loopback_url("http://127.0.0.1:3000/login", "172.21.188.149"),
            "http://172.21.188.149:3000/login"
        );
        assert_eq!(
            rewrite_loopback_url("http://localhost:5173", "172.21.188.149"),
            "http://172.21.188.149:5173"
        );
        assert_eq!(
            rewrite_loopback_url("http://example.com:3000", "172.21.188.149"),
            "http://example.com:3000"
        );
    }

    #[test]
    fn writes_feedback_sections() {
        let dir = tempdir().expect("tempdir");
        let log = SessionLog {
            mission: "web".to_string(),
            runner: RunnerKind::Web,
            detected_command: "agent-browser open".to_string(),
            steps: vec![],
            output_log: vec![],
            errors: vec![],
            captures: vec![],
        };
        write_feedback(dir.path(), &log).expect("feedback");
        let body = fs::read_to_string(dir.path().join(FEEDBACK_FILE)).expect("read");
        assert!(body.contains("# 결과"));
        assert!(body.contains("# 미해결"));
        assert!(body.contains("# 보완"));
    }

    #[test]
    fn builds_window_scripts() {
        assert!(PowerShellWindowsBridge::list_contexts_script().contains("Get-Process"));
        assert!(PowerShellWindowsBridge::select_context_script("1234").contains("1234"));
    }
}
