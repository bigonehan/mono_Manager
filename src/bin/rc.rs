#[path = "rc/browser.rs"]
mod browser;
#[path = "rc/config.rs"]
mod config;

use anyhow::{bail, Context, Result};
use browser::HeadMode;
use clap::{Args, Parser, Subcommand, ValueEnum};
use config::{debug_enabled, load_config, Config};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs;
use std::fs::OpenOptions;
use std::io::{IsTerminal, Read, Write};
use std::net::{IpAddr, UdpSocket};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DRAFTS_FILE: &str = ".project/drafts.yaml";
const JOB_FILE: &str = "job.md";
const SESSION_LOG_FILE: &str = ".rc-session-log.json";
const EXECUTION_RECORD_FILE: &str = ".rc-execution-records.jsonl";
const RUN_LOCK_FILE: &str = ".rc-run.lock";
const PROJECT_DIR: &str = ".project";
const PROJECT_LOG_FILE: &str = ".project/log.md";
const SCREENSHOT_DIR: &str = ".project/screenshot";
const STEP_HEARTBEAT_SEC: u64 = 15;

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
    RunPlaywrightQa(RunPlaywrightQaArgs),
    CheckFrontUiRules,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedCliInput {
    target_path: PathBuf,
    mode: String,
    headed: HeadMode,
}

#[derive(Debug, Args, Clone)]
struct RunPlaywrightQaArgs {
    #[arg(long = "web-root")]
    web_root: PathBuf,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    command: Vec<OsString>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedPlaywrightQaInput {
    web_root: PathBuf,
    command: Vec<OsString>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParsedCommand {
    ClitTest(ParsedCliInput),
    RunPlaywrightQa(ParsedPlaywrightQaInput),
    CheckFrontUiRules,
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

#[derive(Debug)]
struct CapturedCommandOutput {
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WebAction {
    Url(String),
    Selector(String),
    Wait(String),
    ClickLabel(String),
    ClickSelector(String),
    Fill { selector: String, value: String },
    Type(String),
    Assert(String),
    Sleep(u32),
    Reload,
    Snapshot,
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
    match run() {
        Ok(code) => std::process::exit(code),
        Err(err) => {
            eprintln!("{err:#}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<i32> {
    let config = load_config()?;
    let parsed = parse_cli_from(std::env::args_os())?;
    match parsed {
        ParsedCommand::ClitTest(input) => {
            let _ = config;
            let _ = input;
            bail!("rc clit test was removed; use `orc check_orc_code` and ORC helper commands instead")
        }
        ParsedCommand::RunPlaywrightQa(input) => execute_run_playwright_qa(input),
        ParsedCommand::CheckFrontUiRules => execute_check_front_ui_rules(),
    }
}

fn parse_cli_from<I, T>(args: I) -> Result<ParsedCommand>
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
            ClitSubcommand::Test(args) => validate_test_args(args)
                .map(ParsedCommand::ClitTest)
                .map_err(|error| anyhow::anyhow!(error.message)),
        },
        TopLevelCommand::RunPlaywrightQa(args) => validate_run_playwright_qa_args(args)
            .map(ParsedCommand::RunPlaywrightQa)
            .map_err(|error| anyhow::anyhow!(error.message)),
        TopLevelCommand::CheckFrontUiRules => Ok(ParsedCommand::CheckFrontUiRules),
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

fn validate_run_playwright_qa_args(
    args: RunPlaywrightQaArgs,
) -> std::result::Result<ParsedPlaywrightQaInput, CliInputError> {
    let web_root = args.web_root.canonicalize().map_err(|_| {
        CliInputError::invalid_input(format!(
            "invalid argument `--web-root`: path does not exist ({})",
            args.web_root.display()
        ))
    })?;
    if !web_root.is_dir() {
        return Err(CliInputError::invalid_input(format!(
            "invalid argument `--web-root`: not a directory ({})",
            web_root.display()
        )));
    }
    let mut command = args.command;
    if matches!(command.first().and_then(|value| value.to_str()), Some("--")) {
        command.remove(0);
    }
    if command.is_empty() {
        return Err(CliInputError::invalid_input(
            "run-playwright-qa requires a command after `--`",
        ));
    }
    Ok(ParsedPlaywrightQaInput { web_root, command })
}

fn execute_run_playwright_qa(input: ParsedPlaywrightQaInput) -> Result<i32> {
    let helper_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join("playwright_safe_helpers.mjs")
        .canonicalize()
        .with_context(|| "failed to resolve playwright helper path")?;
    let paths = browser::prepare_qa_env_paths(&input.web_root, &helper_path)?;
    let staged = browser::stage_node_entry_script(&input.command, &input.web_root)?;
    let mut command = Command::new(
        staged
            .command
            .first()
            .ok_or_else(|| anyhow::anyhow!("missing QA command"))?,
    );
    if staged.command.len() > 1 {
        command.args(&staged.command[1..]);
    }
    command.current_dir(&input.web_root);
    let mut env = std::env::vars_os().collect::<Vec<_>>();
    let _ = &mut env;
    command.env(
        "NODE_PATH",
        browser::prepend_env_list(
            std::env::var_os("NODE_PATH").as_deref(),
            &paths.node_modules,
        ),
    );
    if paths.bin_dir.exists() {
        command.env(
            "PATH",
            browser::prepend_env_list(std::env::var_os("PATH").as_deref(), &paths.bin_dir),
        );
    }
    command.env("ORC_QA_WEB_ROOT", &input.web_root);
    command.env("ORC_QA_INSTALLED_WORKSPACE", &input.web_root);
    command.env("ORC_QA_PLAYWRIGHT_HELPERS", &paths.helper_path);
    let status = command.status();
    if let Some(staging_dir) = staged.staging_dir {
        let _ = fs::remove_dir_all(staging_dir);
    }
    let status = status.with_context(|| "failed to execute QA command")?;
    Ok(status.code().unwrap_or(1))
}

fn execute_check_front_ui_rules() -> Result<i32> {
    println!("[ui-rule-check] running mono detail alignment e2e");
    let check = browser::build_front_ui_rule_check();
    let status = Command::new(&check.program)
        .args(&check.args)
        .status()
        .with_context(|| "failed to execute front UI rule check")?;
    Ok(status.code().unwrap_or(1))
}

fn execute_test(input: ParsedCliInput, config: &Config) -> Result<()> {
    let workdir = std::env::current_dir()?;
    let _run_lock = acquire_run_lock(&workdir)?;
    fs::create_dir_all(workdir.join(PROJECT_DIR))
        .with_context(|| format!("failed to create {}", PROJECT_DIR))?;
    fs::create_dir_all(workdir.join(SCREENSHOT_DIR))
        .with_context(|| format!("failed to create {}", SCREENSHOT_DIR))?;
    cleanup_legacy_rc_artifacts(&workdir)?;
    fs::create_dir_all(input.target_path.join(SCREENSHOT_DIR))
        .with_context(|| format!("failed to create {}", SCREENSHOT_DIR))?;
    let runner = detect_runner(&input.target_path)?;
    let plan_body = build_plan(
        &input.target_path,
        &input.mode,
        &runner,
        input.headed,
        config,
    )?;
    append_to_job_md(&workdir.join(JOB_FILE), &format!("{}\n", plan_body))?;
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
        "job.md plan appended".to_string(),
        true,
    );
    let check_result = run_check(&input.target_path, &drafts, &mut log, config, &mut recorder);
    match get_current_state(&input.target_path, &runner, config) {
        Ok(state) => log.output_log.push(format!("current-state:\n{state}")),
        Err(error) => log
            .errors
            .push(format!("get_current_state failed: {error:#}")),
    }
    collect_captures(&mut log)?;
    if check_result.is_ok() {
        cleanup_successful_screenshots(&workdir, &input.target_path, &mut log)?;
    }
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
    check_result
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
        .join("rc")
        .join("prompts")
        .join("build_plan.txt");
    let prompt_template = match fs::read_to_string(&prompt_path) {
        Ok(body) => body,
        Err(_) => {
            return Ok(fallback_plan_body(
                target_path,
                mode,
                runner,
                headed,
                config,
            ))
        }
    };
    let inventory = describe_target_path(target_path)?;
    let prompt = format!(
        "{}\n\npath: {}\nrunner: {:?}\nheaded: {:?}\nmode: {}\n\ninventory:\n{}\n\n{}\n{}",
        prompt_template,
        target_path.display(),
        runner,
        headed,
        mode,
        inventory,
        llm_role_instruction(),
        test_plan_output_instruction()
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
    let mut command = Command::new("bash");
    command.args([
        "-lc",
        &format!(
            "timeout 20 codex exec{} {}",
            danger_flag,
            shell_quote(prompt)
        ),
    ]);
    let output = run_command_capture_with_heartbeat(command, "codex-plan", |elapsed_sec| {
        format_rc_phase_heartbeat(
            "build_plan",
            "waiting for codex plan generation",
            elapsed_sec,
        )
    })
    .with_context(|| "failed to execute codex")?;
    if !output.status.success() {
        bail!("codex plan generation failed: {}", output.stderr.trim());
    }
    let stdout = output.stdout.trim().to_string();
    if stdout.is_empty() {
        bail!("codex plan generation returned empty output");
    }
    Ok(stdout)
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn format_rc_phase_heartbeat(phase: &str, detail: &str, elapsed_sec: u64) -> String {
    format!(
        "rc-status phase={} elapsed={}s detail={}",
        phase,
        elapsed_sec,
        trim_heartbeat_command(detail)
    )
}

fn run_command_capture_with_heartbeat<F>(
    mut command: Command,
    runtime_name: &str,
    mut heartbeat_message: F,
) -> Result<CapturedCommandOutput>
where
    F: FnMut(u64) -> String,
{
    let runtime_dir = Path::new(PROJECT_DIR).join("runtime");
    fs::create_dir_all(&runtime_dir)
        .with_context(|| format!("failed to create {}", runtime_dir.display()))?;
    let token = format!("{}-{}", std::process::id(), now_unix_ts());
    let stdout_path = runtime_dir.join(format!("{runtime_name}-{token}.stdout.log"));
    let stderr_path = runtime_dir.join(format!("{runtime_name}-{token}.stderr.log"));
    let stdout_file = fs::File::create(&stdout_path)
        .with_context(|| format!("failed to create {}", stdout_path.display()))?;
    let stderr_file = fs::File::create(&stderr_path)
        .with_context(|| format!("failed to create {}", stderr_path.display()))?;
    command.stdout(Stdio::from(stdout_file));
    command.stderr(Stdio::from(stderr_file));
    let mut child = command.spawn().with_context(|| "failed to spawn command")?;
    let started = Instant::now();
    let mut next_report_sec = STEP_HEARTBEAT_SEC;
    let status = loop {
        match child
            .try_wait()
            .with_context(|| "failed while waiting command")?
        {
            Some(status) => break status,
            None => {
                let elapsed_sec = started.elapsed().as_secs();
                if elapsed_sec >= next_report_sec {
                    let status = heartbeat_message(elapsed_sec);
                    println!("{}", status);
                    let _ = std::io::stdout().flush();
                    let _ = append_project_log(&format!("heartbeat: {}\n", status));
                    next_report_sec += STEP_HEARTBEAT_SEC;
                }
                thread::sleep(Duration::from_millis(250));
            }
        }
    };
    let stdout = fs::read_to_string(&stdout_path).unwrap_or_default();
    let stderr = fs::read_to_string(&stderr_path).unwrap_or_default();
    let _ = fs::remove_file(&stdout_path);
    let _ = fs::remove_file(&stderr_path);
    Ok(CapturedCommandOutput {
        status,
        stdout,
        stderr,
    })
}

fn fallback_plan_body(
    target_path: &Path,
    mode: &str,
    runner: &RunnerKind,
    headed: HeadMode,
    config: &Config,
) -> String {
    let execute_line = plan_execute_line(target_path, runner, config);
    let review_focus = if *runner == RunnerKind::Web {
        "브라우저 e2e 스크린샷으로 실제 렌더/상호작용 성공 여부를 확인하고, 성공 시 스크린샷을 즉시 정리한다."
    } else {
        "실행 로그와 결과 상태를 기준으로 실패 가능성을 먼저 찾고 차단한다."
    };
    format!(
        "# test plan\n\n- role: {}\n- path: {}\n- runner: {:?}\n- headed: {:?}\n- mode: {}\n\n## review focus\n- {}\n\n## execution steps\n- job.md에 테스트 계획을 먼저 기록한다.\n- {} \n- 결과 로그와 산출물을 검토한다.\n\n## expected evidence\n- .project/drafts.yaml 생성\n- session logs 수집\n- web인 경우 e2e 스크린샷 검증 후 성공 시 삭제\n",
        llm_role_instruction_line(),
        target_path.display(),
        runner,
        headed,
        mode,
        review_focus,
        execute_line
    )
}

fn llm_role_instruction() -> &'static str {
    "role: 경험많고 완벽주의적인 시니어 개발자가 코드 리뷰에서 거부할만한 것은 무엇일까요? 전부 수정하세요, 게으름 피우지 마세요"
}

fn llm_role_instruction_line() -> &'static str {
    "경험많고 완벽주의적인 시니어 개발자가 코드 리뷰에서 거부할만한 것은 무엇일까요? 전부 수정하세요, 게으름 피우지 마세요"
}

fn test_plan_output_instruction() -> &'static str {
    "출력은 markdown만 반환한다.\n반드시 `# test plan`, `## review focus`, `## execution steps`, `## expected evidence` 섹션을 포함한다.\nweb runner면 브라우저 e2e 스크린샷으로 동작 여부를 검증하고, 성공 시 스크린샷을 삭제한다는 단계를 명시한다.\n테스트 실행보다 먼저 job.md에 기록할 계획만 작성한다."
}

fn plan_execute_line(target_path: &Path, runner: &RunnerKind, config: &Config) -> String {
    match runner {
        RunnerKind::Web => format!(
            "{} -> browser open {}",
            detect_web_server_command(target_path),
            browser_reachable_url_for_target(target_path, &config.browser_url)
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
    let login_mode = mode.to_ascii_lowercase().contains("login");
    let actions = parse_web_actions(mode);
    let wait_selector = actions
        .iter()
        .find_map(|action| match action {
            WebAction::Selector(selector) | WebAction::Wait(selector) => Some(selector.clone()),
            _ => None,
        })
        .or_else(|| extract_action_value(mode, "selector"))
        .or_else(|| extract_action_value(mode, "wait"))
        .unwrap_or_else(|| {
            if login_mode {
                ".auth-card:nth-of-type(1) input[placeholder='user id']".to_string()
            } else {
                "body".to_string()
            }
        });
    let mut steps = vec![
        Step {
            command_template: format!(
                "if [ -f .rc-web-server.pid ]; then kill $(cat .rc-web-server.pid) >/dev/null 2>&1 || true; fi; nohup {} > .rc-web-server.log 2>&1 < /dev/null & echo $! > .rc-web-server.pid; sleep 5",
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
            command_template: browser::open_command(agent, &requested_url, headed),
            responses: Vec::new(),
        },
        Step {
            command_template: browser::wait_for_selector_command(agent, &wait_selector),
            responses: Vec::new(),
        },
        Step {
            command_template: browser::snapshot_command(agent),
            responses: Vec::new(),
        },
    ];
    if login_mode {
        steps.extend(login_steps(agent));
    }
    for action in actions {
        match action {
            WebAction::Url(_) | WebAction::Selector(_) => {}
            WebAction::Wait(selector) | WebAction::Assert(selector) => {
                steps.push(Step {
                    command_template: browser::wait_for_selector_command(agent, &selector),
                    responses: Vec::new(),
                });
            }
            WebAction::ClickLabel(label) => {
                steps.push(Step {
                    command_template: browser::click_command(agent, &label),
                    responses: build_responses(mode),
                });
            }
            WebAction::ClickSelector(selector) => {
                steps.push(Step {
                    command_template: browser::click_selector_command(agent, &selector),
                    responses: build_responses(mode),
                });
            }
            WebAction::Fill { selector, value } => {
                steps.push(Step {
                    command_template: browser::fill_command(agent, &selector, &value),
                    responses: Vec::new(),
                });
            }
            WebAction::Type(text) => {
                steps.push(Step {
                    command_template: browser::keyboard_type_command(agent, &text),
                    responses: build_responses(mode),
                });
            }
            WebAction::Sleep(seconds) => {
                steps.push(Step {
                    command_template: browser::sleep_command(seconds),
                    responses: Vec::new(),
                });
            }
            WebAction::Reload => {
                steps.push(Step {
                    command_template: browser::open_command(agent, &requested_url, headed),
                    responses: Vec::new(),
                });
            }
            WebAction::Snapshot => {
                steps.push(Step {
                    command_template: browser::snapshot_command(agent),
                    responses: Vec::new(),
                });
            }
        }
    }
    let screenshot_path = target_path.join(SCREENSHOT_DIR).join("rc-web.png");
    steps.push(Step {
        command_template: format!(
            "{}; if [ -f .rc-web-server.pid ]; then kill $(cat .rc-web-server.pid) >/dev/null 2>&1 || true; fi",
            browser::screenshot_and_close_command(agent, &screenshot_path)
        ),
        responses: Vec::new(),
    });
    DraftProcedure {
        name: if login_mode {
            "web_login_check".to_string()
        } else {
            "web_smoke_check".to_string()
        },
        expected: if login_mode {
            "user can log in through the UI".to_string()
        } else {
            "page loads through the UI".to_string()
        },
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
        return "http://localhost:5173".to_string();
    }
    config.browser_url.clone()
}

fn web_server_exposes_network_host(target_path: &Path) -> bool {
    let package_json = fs::read_to_string(target_path.join("package.json")).unwrap_or_default();
    let lowered = package_json.to_ascii_lowercase();
    ["--host", "--hostname", "0.0.0.0"]
        .iter()
        .any(|needle| lowered.contains(needle))
}

fn browser_reachable_url_for_target(target_path: &Path, url: &str) -> String {
    let replacement_host = local_ipv4_address();
    browser_reachable_url_for_target_with_host(target_path, url, replacement_host.as_deref())
}

fn browser_reachable_url_for_target_with_host(
    target_path: &Path,
    url: &str,
    replacement_host: Option<&str>,
) -> String {
    if !web_server_exposes_network_host(target_path) {
        return url.to_string();
    }
    let Some(host) = replacement_host else {
        return url.to_string();
    };
    rewrite_loopback_url(url, host)
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

fn parse_web_actions(mode: &str) -> Vec<WebAction> {
    let mut actions = Vec::new();
    let mut index = 0usize;
    while index < mode.len() {
        skip_ascii_whitespace(mode, &mut index);
        let Some((keyword, next_index)) = parse_keyword(mode, index) else {
            index += 1;
            continue;
        };
        index = next_index;
        let action = match keyword.as_str() {
            "url" => parse_action_arg(mode, &mut index).map(WebAction::Url),
            "selector" => parse_action_arg(mode, &mut index).map(WebAction::Selector),
            "wait" => parse_action_arg(mode, &mut index).map(WebAction::Wait),
            "click" => parse_action_arg(mode, &mut index).map(WebAction::ClickLabel),
            "click-selector" => parse_action_arg(mode, &mut index).map(WebAction::ClickSelector),
            "fill" => {
                let selector = parse_action_arg(mode, &mut index);
                let value = parse_action_arg(mode, &mut index);
                match (selector, value) {
                    (Some(selector), Some(value)) => Some(WebAction::Fill { selector, value }),
                    _ => None,
                }
            }
            "input" | "type" => parse_action_arg(mode, &mut index).map(WebAction::Type),
            "assert" => parse_action_arg(mode, &mut index).map(WebAction::Assert),
            "sleep" => parse_action_arg(mode, &mut index)
                .and_then(|value| value.parse::<u32>().ok())
                .map(WebAction::Sleep),
            "reload" => Some(WebAction::Reload),
            "snapshot" => Some(WebAction::Snapshot),
            _ => None,
        };
        if let Some(action) = action {
            actions.push(action);
        }
    }
    actions
}

fn skip_ascii_whitespace(input: &str, index: &mut usize) {
    while let Some(ch) = input.get(*index..).and_then(|rest| rest.chars().next()) {
        if !ch.is_ascii_whitespace() {
            break;
        }
        *index += ch.len_utf8();
    }
}

fn parse_keyword(input: &str, index: usize) -> Option<(String, usize)> {
    let rest = input.get(index..)?;
    let mut end = 0usize;
    for ch in rest.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' {
            end += ch.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 {
        return None;
    }
    let keyword = rest[..end].to_ascii_lowercase();
    Some((keyword, index + end))
}

fn parse_action_arg(input: &str, index: &mut usize) -> Option<String> {
    skip_ascii_whitespace(input, index);
    let rest = input.get(*index..)?;
    if let Some(quoted) = rest.strip_prefix('"') {
        let end = quoted.find('"')?;
        *index += end + 2;
        return Some(quoted[..end].to_string());
    }
    let mut end = 0usize;
    for ch in rest.chars() {
        if ch.is_ascii_whitespace() {
            break;
        }
        end += ch.len_utf8();
    }
    if end == 0 {
        return None;
    }
    *index += end;
    Some(rest[..end].to_string())
}

fn mission_requires_state_verification(mode: &str) -> bool {
    let lowered = mode.to_ascii_lowercase();
    [
        "save", "delete", "remove", "edit", "update", "assign", "create", "insert", "submit",
        "저장", "삭제", "수정", "추가", "생성", "할당",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

fn mission_has_persistence_check(actions: &[WebAction]) -> bool {
    let mut mutation_seen = false;
    let mut reload_seen = false;
    for action in actions {
        match action {
            WebAction::ClickLabel(label) => {
                let lowered = label.to_ascii_lowercase();
                if [
                    "save", "delete", "remove", "edit", "assign", "submit", "저장", "삭제", "수정",
                    "할당",
                ]
                .iter()
                .any(|needle| lowered.contains(needle))
                {
                    mutation_seen = true;
                }
            }
            WebAction::ClickSelector(selector) => {
                let lowered = selector.to_ascii_lowercase();
                if ["save", "delete", "remove", "edit", "assign", "submit"]
                    .iter()
                    .any(|needle| lowered.contains(needle))
                {
                    mutation_seen = true;
                }
            }
            WebAction::Fill { .. } | WebAction::Type(_) => {
                if mutation_seen {
                    continue;
                }
            }
            WebAction::Reload => {
                if mutation_seen {
                    reload_seen = true;
                }
            }
            WebAction::Wait(_) | WebAction::Assert(_) => {
                if mutation_seen && reload_seen {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
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
            let outcome = run_step(target_path, &procedure.name, step)?;
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
            if !outcome.errors.is_empty() {
                bail!(
                    "step failed: {}",
                    outcome
                        .errors
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>()
                        .join(" | ")
                );
            }
        }
    }
    Ok(())
}

fn format_step_heartbeat(procedure_name: &str, command: &str, elapsed_sec: u64) -> String {
    format!(
        "step~ procedure={} elapsed={}s command={}",
        procedure_name,
        elapsed_sec,
        trim_heartbeat_command(command)
    )
}

fn trim_heartbeat_command(command: &str) -> String {
    let normalized = command.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.len() <= 160 {
        normalized
    } else {
        format!("{}...", &normalized[..157])
    }
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

fn now_unix_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|v| v.as_secs())
        .unwrap_or(0)
}

fn run_step(target_path: &Path, procedure_name: &str, step: &Step) -> Result<StepOutcome> {
    let mut command = Command::new("bash");
    command.args(["-lc", &step.command_template]);
    command.current_dir(target_path);
    let output = run_command_capture_with_heartbeat(command, "step", |elapsed_sec| {
        format_step_heartbeat(procedure_name, &step.command_template, elapsed_sec)
    })
    .with_context(|| format!("failed to execute step `{}`", step.command_template))?;
    let stdout = output.stdout;
    let stderr = output.stderr;
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
    lowered.contains("✗")
        || lowered.contains("enoent")
        || lowered.contains("failed")
        || lowered
            .lines()
            .any(|line| line.trim_start().starts_with("error:"))
        || lowered
            .lines()
            .any(|line| line.trim_start().starts_with("error "))
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
    let screenshot_dir = workdir.join(SCREENSHOT_DIR);
    fs::create_dir_all(&screenshot_dir)
        .with_context(|| format!("failed to create {}", screenshot_dir.display()))?;
    let terminal_capture = capture_terminal_session(&workdir)?;
    log.captures.push(terminal_capture);
    let browser_capture = screenshot_dir.join("rc-web.png");
    if browser_capture.exists() {
        log.captures.push(browser_capture);
    }
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
        .push(bridge.capture_rect_to(&screenshot_dir.join("rect-capture.png"))?);
    log.captures
        .push(bridge.capture_screen_to(&screenshot_dir.join("screen-capture.png"))?);
    fs::write(
        workdir.join(SESSION_LOG_FILE),
        serde_json::to_string_pretty(log)?,
    )
    .with_context(|| "failed to write session log")?;
    Ok(())
}

fn capture_terminal_session(workdir: &Path) -> Result<PathBuf> {
    let output_path = workdir.join(SCREENSHOT_DIR).join("terminal-capture.txt");
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
    let web_actions = if log.runner == RunnerKind::Web {
        parse_web_actions(&log.mission)
    } else {
        Vec::new()
    };
    let requires_state_check =
        log.runner == RunnerKind::Web && mission_requires_state_verification(&log.mission);
    let persistence_checked = !requires_state_check || mission_has_persistence_check(&web_actions);
    let checklist = evaluate_checklist(workdir, log, &effective_errors)?;
    let mut unresolved_items = effective_errors.clone();
    unresolved_items.extend(checklist.unresolved.clone());
    if requires_state_check && !persistence_checked {
        unresolved_items.push(
            "state mutation not verified: reload/reopen 뒤 selector/assert 단계가 없어 render-only 검증에 머물렀다."
                .to_string(),
        );
    }
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
        if requires_state_check {
            "- 현재 draft 절차가 상태 변화 검증까지 포함했고 화면 근거(snapshot/screenshot)와 reload/assert 확인이 기록됐다."
                .to_string()
        } else {
            "- 현재 draft 절차가 체크리스트 기준을 만족했고 화면 근거(snapshot/screenshot)가 기록됐다."
                .to_string()
        }
    } else {
        "- clit 결과를 기준으로 plan/drafts 절차를 갱신해야 한다. render-only 검증과 상태 변화 검증을 구분하라."
            .to_string()
    };
    let result = format!(
        "\n# clit feedback\n\n## 결과\n- runner: {:?}\n- detected command: {}\n- verification: {}\n- steps: {}\n- captures: {}\n\n### 체크리스트\n{}\n\n### 미해결\n{}\n\n### 보완\n{}\n",
        log.runner,
        log.detected_command,
        if requires_state_check {
            if persistence_checked {
                "state-change verified"
            } else {
                "render-only for a mutation mission"
            }
        } else {
            "render-only"
        },
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
    let job_path = workdir.join(JOB_FILE);
    let mut job_body = fs::read_to_string(&job_path).unwrap_or_default();
    if !job_body.ends_with('\n') {
        job_body.push('\n');
    }
    if !job_body.contains("# clit feedback") {
        job_body.push_str(&result);
    } else {
        job_body.push_str(&result);
    }
    fs::write(&job_path, job_body)
        .with_context(|| format!("failed to write {}", job_path.display()))?;
    Ok(())
}

fn append_to_job_md(job_path: &Path, addition: &str) -> Result<()> {
    let mut body = fs::read_to_string(job_path).unwrap_or_default();
    if !body.ends_with('\n') {
        body.push('\n');
    }
    if !body.is_empty() {
        body.push('\n');
    }
    body.push_str(addition);
    fs::write(job_path, body).with_context(|| format!("failed to write {}", job_path.display()))
}

fn cleanup_legacy_rc_artifacts(workdir: &Path) -> Result<()> {
    let screenshot_dir = workdir.join(SCREENSHOT_DIR);
    for (legacy, target) in [
        (
            workdir.join("drafts.yaml"),
            workdir.join(".project").join("drafts.yaml"),
        ),
        (
            workdir.join("terminal-capture.txt"),
            screenshot_dir.join("terminal-capture.txt"),
        ),
        (
            workdir.join("rect-capture.png"),
            screenshot_dir.join("rect-capture.png"),
        ),
        (
            workdir.join("screen-capture.png"),
            screenshot_dir.join("screen-capture.png"),
        ),
        (
            workdir.join("rc-web.png"),
            screenshot_dir.join("rc-web.png"),
        ),
    ] {
        if legacy.exists() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
            let _ = fs::rename(&legacy, &target);
        }
    }
    Ok(())
}

fn evaluate_checklist(
    workdir: &Path,
    log: &SessionLog,
    effective_errors: &[String],
) -> Result<ChecklistEvaluation> {
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
    let body = run_codex_checklist_prompt(&prompt)
        .unwrap_or_else(|_| fallback_checklist(log, effective_errors));
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
    let web_rules = if log.runner == RunnerKind::Web {
        "\nweb_rules:\n- 브라우저 e2e 스크린샷이 실제 렌더 확인 근거인지 점검한다.\n- 저장/삭제/생성/수정처럼 상태가 바뀌는 mission이면 reload 또는 reopen 이후 selector/assert 검증이 있는지 점검한다.\n- 성공한 실행이면 스크린샷 정리 여부도 확인한다.\n"
    } else {
        ""
    };
    format!(
        "{}\n다음 실행 로그를 보고 check_list.md 형식만 출력해라.\n형식: - [x| ] {{입력}} -> {{출력}} : 기능설명\n규칙: 현재 결과가 충족되면 [x], 미충족이면 [ ].\nweb인 경우 e2e 스크린샷 검증과 성공 후 스크린샷 삭제 여부를 체크리스트에 포함한다.\n\nmission: {}\nrunner: {:?}\nsteps:\n{}\n\nrecent_output:\n{}\n\neffective_errors:\n{}{}\n",
        llm_role_instruction(),
        log.mission,
        log.runner,
        log.steps
            .iter()
            .map(|step| format!("- {}", step.command_template))
            .collect::<Vec<_>>()
            .join("\n"),
        outputs,
        errors,
        web_rules
    )
}

fn run_codex_checklist_prompt(prompt: &str) -> Result<String> {
    let danger_flag = if std::env::var("CODEX_DANGEROUSLY_BYPASS_APPROVALS_AND_SANDBOX").is_ok() {
        ""
    } else {
        " --dangerously-bypass-approvals-and-sandbox"
    };
    let mut command = Command::new("bash");
    command.args([
        "-lc",
        &format!(
            "timeout 20 codex exec{} {}",
            danger_flag,
            shell_quote(prompt)
        ),
    ]);
    let output = run_command_capture_with_heartbeat(command, "codex-checklist", |elapsed_sec| {
        format_rc_phase_heartbeat(
            "checklist",
            "waiting for codex checklist generation",
            elapsed_sec,
        )
    })
    .with_context(|| "failed to execute codex checklist prompt")?;
    if !output.status.success() {
        bail!(
            "codex checklist generation failed: {}",
            output.stderr.trim()
        );
    }
    let stdout = output.stdout.trim().to_string();
    if stdout.is_empty() {
        bail!("codex checklist generation returned empty output");
    }
    Ok(stdout)
}

fn fallback_checklist(log: &SessionLog, effective_errors: &[String]) -> String {
    let actions = if log.runner == RunnerKind::Web {
        parse_web_actions(&log.mission)
    } else {
        Vec::new()
    };
    let requires_state_check =
        log.runner == RunnerKind::Web && mission_requires_state_verification(&log.mission);
    let persistence_checked = !requires_state_check || mission_has_persistence_check(&actions);
    let status = if effective_errors.is_empty() {
        "x"
    } else {
        " "
    };
    let output = if effective_errors.is_empty() {
        "기본 점검 통과"
    } else {
        "기본 점검 실패"
    };
    let mut body = format!(
        "- [{}] {} -> {} : mode 기반 기본 체크리스트\n- [x] step 실행 -> output_log 기록 : 실행 로그 수집\n",
        status, log.mission, output
    );
    if log.runner == RunnerKind::Web {
        let screenshot_checked = log
            .output_log
            .iter()
            .any(|line| line.contains("validated web e2e screenshot"));
        let screenshot_cleaned = log
            .output_log
            .iter()
            .any(|line| line.contains("cleaned screenshot"));
        body.push_str(&format!(
            "- [{}] web e2e screenshot -> 렌더 확인 : 브라우저 스크린샷 검증\n",
            if screenshot_checked { "x" } else { " " }
        ));
        body.push_str(&format!(
            "- [{}] successful screenshot cleanup -> png removed : 성공 후 스크린샷 정리\n",
            if screenshot_cleaned { "x" } else { " " }
        ));
        if requires_state_check {
            body.push_str(&format!(
                "- [{}] mutation flow -> reload/assert after save/delete/create : 상태 변화 영속성 검증\n",
                if persistence_checked { "x" } else { " " }
            ));
        }
    }
    body
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
    let pane_output = Command::new("orc")
        .arg("worker-create")
        .output()
        .with_context(|| "failed to create tmux worker")?;
    if !pane_output.status.success() {
        return Ok(());
    }
    let worker_ref = String::from_utf8_lossy(&pane_output.stdout)
        .trim()
        .to_string();
    if worker_ref.is_empty() {
        return Ok(());
    }
    let message = format!(
        "{}\njob.md를 읽고 해결할 수 있는 문제와 개선점을 찾아서 개선하라",
        llm_role_instruction()
    )
    .replace('"', "\\\"");
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
    let status = Command::new("orc")
        .args(["worker-send", &worker_ref, &command, "enter"])
        .status()
        .with_context(|| "failed to start codex in tmux pane")?;
    if !status.success() {
        return Ok(());
    }
    Ok(())
}

fn cleanup_successful_screenshots(
    workdir: &Path,
    target_path: &Path,
    log: &mut SessionLog,
) -> Result<()> {
    let screenshot_candidates = [
        target_path.join(SCREENSHOT_DIR).join("rc-web.png"),
        workdir.join(SCREENSHOT_DIR).join("rc-web.png"),
        workdir.join(SCREENSHOT_DIR).join("rect-capture.png"),
        workdir.join(SCREENSHOT_DIR).join("screen-capture.png"),
    ];
    let mut removed = Vec::new();
    for path in screenshot_candidates {
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove screenshot {}", path.display()))?;
            removed.push(path);
        }
    }
    if !removed.is_empty() {
        log.output_log.push(format!(
            "validated web e2e screenshot before cleanup: {}",
            removed
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
        log.output_log.push(format!(
            "cleaned screenshot after successful test: {}",
            removed
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
        log.captures
            .retain(|path| !removed.iter().any(|removed_path| removed_path == path));
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
        .with_context(|| {
            format!(
                "another rc run is already in progress (lock: {})",
                path.display()
            )
        })?;
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
    fn parses_run_playwright_qa_arguments() {
        let dir = tempdir().expect("tempdir");
        let parsed = validate_run_playwright_qa_args(RunPlaywrightQaArgs {
            web_root: dir.path().to_path_buf(),
            command: vec![
                OsString::from("--"),
                OsString::from("node"),
                OsString::from("qa-check.mjs"),
            ],
        })
        .expect("parse");
        assert_eq!(
            parsed.web_root,
            dir.path().canonicalize().expect("canonicalize")
        );
        assert_eq!(
            parsed.command,
            vec![OsString::from("node"), OsString::from("qa-check.mjs")]
        );
    }

    #[test]
    fn parse_cli_supports_check_front_ui_rules_command() {
        let parsed = parse_cli_from(["rc", "check-front-ui-rules"]).expect("parse");
        assert_eq!(parsed, ParsedCommand::CheckFrontUiRules);
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
    fn requested_web_url_prefers_localhost_for_vite() {
        let dir = tempdir().expect("tempdir");
        fs::write(
            dir.path().join("package.json"),
            r#"{"scripts":{"dev":"vite --host 0.0.0.0 --port 5173"}}"#,
        )
        .expect("write");
        let config = load_config().expect("config");
        assert_eq!(
            requested_web_url(dir.path(), "smoke", &config),
            "http://localhost:5173"
        );
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
        assert!(drafts.procedures[0].steps.iter().any(|step| {
            step.command_template.contains("python3 - \"")
                && step.command_template.contains("<<'PY'")
        }));
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
    fn builds_generic_smoke_drafts_for_web_mode_without_login_selector() {
        let dir = tempdir().expect("tempdir");
        fs::write(
            dir.path().join("package.json"),
            r#"{"scripts":{"dev":"vite --host 0.0.0.0 --port 5173"}}"#,
        )
        .expect("write");
        let config = load_config().expect("config");
        let drafts = build_drafts(
            dir.path(),
            "PageEditor build verification",
            &RunnerKind::Web,
            HeadMode::Off,
            &config,
        )
        .expect("drafts");
        assert_eq!(drafts.procedures[0].name, "web_smoke_check");
        assert_eq!(drafts.procedures[0].expected, "page loads through the UI");
        assert!(drafts.procedures[0]
            .steps
            .iter()
            .any(|step| step.command_template == "agent-browser wait \"body\""));
        assert!(drafts.procedures[0]
            .steps
            .first()
            .is_some_and(|step| step.command_template.contains("nohup")));
        assert!(!drafts.procedures[0].steps.iter().any(|step| {
            step.command_template
                .contains(".auth-card:nth-of-type(1) input[placeholder='user id']")
        }));
    }

    #[test]
    fn parses_web_actions_in_order() {
        let actions = parse_web_actions(
            r##"url "http://localhost:5173/preset.html" selector "body" click "1번 슬롯" fill "#group-preset-title-input" "저장 테스트" fill "#group-preset-content-input" "내용" click-selector "#save-group-preset-btn" reload assert ".preset-item-card""##,
        );
        assert_eq!(
            actions,
            vec![
                WebAction::Url("http://localhost:5173/preset.html".to_string()),
                WebAction::Selector("body".to_string()),
                WebAction::ClickLabel("1번 슬롯".to_string()),
                WebAction::Fill {
                    selector: "#group-preset-title-input".to_string(),
                    value: "저장 테스트".to_string(),
                },
                WebAction::Fill {
                    selector: "#group-preset-content-input".to_string(),
                    value: "내용".to_string(),
                },
                WebAction::ClickSelector("#save-group-preset-btn".to_string()),
                WebAction::Reload,
                WebAction::Assert(".preset-item-card".to_string()),
            ]
        );
    }

    #[test]
    fn builds_web_steps_for_fill_click_reload_assert_flow() {
        let dir = tempdir().expect("tempdir");
        fs::write(
            dir.path().join("package.json"),
            r#"{"scripts":{"dev":"vite --host 0.0.0.0 --port 5173"}}"#,
        )
        .expect("write");
        let config = load_config().expect("config");
        let drafts = build_drafts(
            dir.path(),
            r##"url "http://localhost:5173/preset.html" selector "body" click "1번 슬롯" fill "#group-preset-title-input" "저장 테스트" click-selector "#save-group-preset-btn" reload assert ".preset-item-card""##,
            &RunnerKind::Web,
            HeadMode::Off,
            &config,
        )
        .expect("drafts");
        let commands = drafts.procedures[0]
            .steps
            .iter()
            .map(|step| step.command_template.as_str())
            .collect::<Vec<_>>();
        assert!(commands
            .iter()
            .any(|command| command.contains(r#"find role button click --name "1번 슬롯""#)));
        assert!(commands
            .iter()
            .any(|command| command.contains("fill \"#group-preset-title-input\" \"저장 테스트\"")));
        assert!(commands
            .iter()
            .any(|command| command.contains("click \"#save-group-preset-btn\"")));
        assert!(
            commands
                .iter()
                .filter(|command| command.contains("open http://localhost:5173/preset.html"))
                .count()
                >= 2
        );
        assert!(commands
            .iter()
            .any(|command| command.contains(r#"wait ".preset-item-card""#)));
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
    fn keeps_loopback_url_for_localhost_only_vite_server() {
        let dir = tempdir().expect("tempdir");
        fs::write(
            dir.path().join("package.json"),
            r#"{"scripts":{"dev":"vite"}}"#,
        )
        .expect("write package.json");
        assert_eq!(
            browser_reachable_url_for_target_with_host(
                dir.path(),
                "http://127.0.0.1:5173",
                Some("172.21.188.149"),
            ),
            "http://127.0.0.1:5173"
        );
    }

    #[test]
    fn rewrites_loopback_url_when_web_server_exposes_host() {
        let dir = tempdir().expect("tempdir");
        fs::write(
            dir.path().join("package.json"),
            r#"{"scripts":{"dev":"vite --host 0.0.0.0 --port 5173"}}"#,
        )
        .expect("write package.json");
        assert_eq!(
            browser_reachable_url_for_target_with_host(
                dir.path(),
                "http://127.0.0.1:5173",
                Some("172.21.188.149"),
            ),
            "http://172.21.188.149:5173"
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
        let body = fs::read_to_string(dir.path().join(JOB_FILE)).expect("read");
        assert!(body.contains("## 결과"));
        assert!(body.contains("### 미해결"));
        assert!(body.contains("### 보완"));
    }

    #[test]
    fn feedback_marks_mutation_without_reload_as_unresolved() {
        let dir = tempdir().expect("tempdir");
        let log = SessionLog {
            mission: r##"url "http://localhost:5173/preset.html" selector "body" click-selector "#save-group-preset-btn""##.to_string(),
            runner: RunnerKind::Web,
            detected_command: "npm run dev".to_string(),
            steps: vec![],
            output_log: vec![],
            errors: vec![],
            captures: vec![],
        };
        write_feedback(dir.path(), &log).expect("feedback");
        let body = fs::read_to_string(dir.path().join(JOB_FILE)).expect("read");
        assert!(body.contains("verification: render-only for a mutation mission"));
        assert!(body.contains("state mutation not verified"));
    }

    #[test]
    fn fallback_plan_mentions_test_plan_and_web_cleanup() {
        let dir = tempdir().expect("tempdir");
        let config = load_config().expect("config");
        let body = fallback_plan_body(
            dir.path(),
            "web smoke",
            &RunnerKind::Web,
            HeadMode::Off,
            &config,
        );
        assert!(body.contains("# test plan"));
        assert!(body.contains("role: 경험많고 완벽주의적인 시니어 개발자"));
        assert!(body.contains("e2e 스크린샷"));
        assert!(body.contains("성공 시 삭제"));
    }

    #[test]
    fn checklist_prompt_includes_strict_role_and_web_rules() {
        let log = SessionLog {
            mission: "web".to_string(),
            runner: RunnerKind::Web,
            detected_command: "npm run dev".to_string(),
            steps: vec![Step {
                command_template: "agent-browser screenshot".to_string(),
                responses: vec![],
            }],
            output_log: vec!["ok".to_string()],
            errors: vec![],
            captures: vec![],
        };
        let prompt = build_checklist_prompt(&log, &[]);
        assert!(prompt.contains("경험많고 완벽주의적인 시니어 개발자"));
        assert!(prompt.contains("e2e 스크린샷"));
        assert!(prompt.contains("성공 후 스크린샷 삭제"));
    }

    #[test]
    fn cleans_successful_screenshots_and_removes_capture_entries() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("app");
        fs::create_dir_all(target.join(".project/screenshot"))
            .expect("create target screenshot dir");
        fs::create_dir_all(dir.path().join(".project/screenshot"))
            .expect("create work screenshot dir");
        let target_capture = target.join(".project/screenshot/rc-web.png");
        let rect_capture = dir.path().join(".project/screenshot/rect-capture.png");
        let screen_capture = dir.path().join(".project/screenshot/screen-capture.png");
        fs::write(&target_capture, b"png").expect("write target capture");
        fs::write(&rect_capture, b"png").expect("write rect capture");
        fs::write(&screen_capture, b"png").expect("write screen capture");
        let mut log = SessionLog {
            mission: "web".to_string(),
            runner: RunnerKind::Web,
            detected_command: "agent-browser open".to_string(),
            steps: vec![],
            output_log: vec![],
            errors: vec![],
            captures: vec![
                target_capture.clone(),
                rect_capture.clone(),
                screen_capture.clone(),
            ],
        };
        cleanup_successful_screenshots(dir.path(), &target, &mut log).expect("cleanup");
        assert!(!target_capture.exists());
        assert!(!rect_capture.exists());
        assert!(!screen_capture.exists());
        assert!(log.captures.is_empty());
        assert!(log
            .output_log
            .iter()
            .any(|line| line.contains("validated web e2e screenshot")));
        assert!(log
            .output_log
            .iter()
            .any(|line| line.contains("cleaned screenshot")));
    }

    #[test]
    fn formats_step_heartbeat_with_elapsed_seconds() {
        let line = format_step_heartbeat("web_smoke_check", "bun run dev", 30);
        assert!(line.contains("procedure=web_smoke_check"));
        assert!(line.contains("elapsed=30s"));
        assert!(line.contains("command=bun run dev"));
    }

    #[test]
    fn detects_real_error_signals_without_matching_install_note() {
        assert!(contains_error_signal("✗ ENOENT: no such file or directory"));
        assert!(contains_error_signal(
            "command failed while saving screenshot"
        ));
        assert!(!contains_error_signal(
            "Note: If you see \"shared library\" errors when running, use install --with-deps"
        ));
    }

    #[test]
    fn builds_window_scripts() {
        assert!(PowerShellWindowsBridge::list_contexts_script().contains("Get-Process"));
        assert!(PowerShellWindowsBridge::select_context_script("1234").contains("1234"));
    }
}
