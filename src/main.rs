mod compoents;
mod input;

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use clap::{Args, Parser, Subcommand};
use compoents::working_pane::WorkingPaneEvent;
use include_dir::{Dir, include_dir};
use input::question::{InputAnswerKind, input_ask_question};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;

const PATH_PROMP_POSTPIX: &str = "assets/prompts/Prompt_Postpix.txt";
const PATH_TODOS_PROMPT: &str = "assets/prompts/Prompt_Todos.txt";
const TODO_BODY: &str = "{{body}}";
static DIR_TEMPLATE: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets");

#[derive(Parser, Debug)]
#[command(name = "orc")]
#[command(about = "Parallel codex worker orchestrator with callback server")]
struct InputCli {
    #[command(subcommand)]
    command: Option<InputCommand>,
}

#[derive(Subcommand, Debug)]
enum InputCommand {
    Serve(InputServeArgs),
    #[command(name = "run-paralles", alias = "run-parallel")]
    RunParalles(InputRunParallelArgs),
    #[command(name = "show-ui", alias = "run-test")]
    ShowUi(InputRunTestArgs),
    #[command(name = "make-spec")]
    MakeSpec(InputBuildSpecArgs),
    #[command(name = "fill-spec")]
    FillSpec(InputFillSpecFromInputArgs),
    #[command(name = "make-todos")]
    MakeTodos(InputMakeTodosArgs),
    CheckLast(InputCheckLastArgs),
}

#[derive(Args, Debug)]
struct InputServeArgs {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    #[arg(long, default_value_t = 7878)]
    port: u16,
}

#[derive(Args, Debug)]
struct InputRunParallelArgs {
    #[arg(long)]
    server_url: String,
    #[arg(long)]
    n: usize,
    #[arg(long = "msg", required = true)]
    msgs: Vec<String>,
    #[arg(long)]
    codex_bin: Option<String>,
    #[arg(long, default_value_t = false)]
    dry_run: bool,
    #[arg(long, default_value_t = true)]
    send_only: bool,
}

#[derive(Args, Debug)]
struct InputRunTestArgs {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    #[arg(long, default_value_t = 7878)]
    port: u16,
    #[arg(long)]
    codex_bin: Option<String>,
    #[arg(long, default_value_t = false)]
    dry_run: bool,
    #[arg(long, default_value_t = false)]
    send_only: bool,
    #[arg(long = "add-msg")]
    add_msgs: Vec<String>,
}

#[derive(Args, Debug)]
struct InputBuildSpecArgs {
    #[arg(long, default_value = "test")]
    project: String,
}

#[derive(Args, Debug)]
struct InputFillSpecFromInputArgs {
    #[arg(long, default_value = "test")]
    project: String,
    #[arg(long, default_value = "input.txt")]
    input_path: String,
}

#[derive(Args, Debug)]
struct InputMakeTodosArgs {
    #[arg(long, default_value = "test")]
    project: String,
}

#[derive(Args, Debug)]
struct InputCheckLastArgs {
    #[arg(long, default_value = "test")]
    project: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServerInfo {
    protocol: String,
    callback_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerCompletion {
    worker_id: usize,
    command_message: String,
    codex_finished: bool,
    exit_code: i32,
    stdout: String,
    stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerEnvelope {
    server: ServerInfo,
    result: WorkerCompletion,
}

#[derive(Debug, Clone, Deserialize)]
struct BlueprintFile {
    tasks: Option<Vec<BlueprintTaskItem>>,
    todos: Option<Vec<BlueprintTaskItem>>,
}

#[derive(Debug, Clone, Deserialize)]
struct BlueprintTaskItem {
    name: String,
    #[serde(rename = "type")]
    task_type: String,
    scope: Vec<String>,
    rule: Vec<String>,
    step: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AppConfigFile {
    ai: Option<AiConfig>,
    project: Option<ProjectConfig>,
}

#[derive(Debug, Clone, Deserialize)]
struct AiConfig {
    model: Option<String>,
    auto: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct ProjectConfig {
    path: Option<String>,
}

#[derive(Debug, Clone)]
struct AiRuntimeOptions {
    model: String,
    auto: bool,
}

#[derive(Clone, Default)]
struct AppState {
    results: Arc<Mutex<Vec<WorkerEnvelope>>>,
    quiet_server_logs: bool,
}

#[tokio::main]
async fn main() {
    if let Err(err) = flow_main().await {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

async fn flow_main() -> Result<()> {
    ensure_git_repository_for_root()?;
    #[allow(non_snake_case)]
    let currentProject = "test".to_string();
    let cli = input_parse_cli();
    match cli.command {
        Some(InputCommand::Serve(args)) => stage_open_server(args).await,
        Some(InputCommand::RunParalles(args)) => stage_start_parallel(args, None).await,
        Some(InputCommand::ShowUi(args)) => stage_run_test(args, currentProject.clone()).await,
        Some(InputCommand::MakeSpec(args)) => stage_build_spec(args).await,
        Some(InputCommand::FillSpec(args)) => stage_fill_spec_from_input(args),
        Some(InputCommand::MakeTodos(args)) => stage_make_todos(args).await,
        Some(InputCommand::CheckLast(args)) => stage_check_last(args).await,
        None => {
            stage_run_test(InputRunTestArgs {
                host: "127.0.0.1".to_string(),
                port: 7878,
                codex_bin: None,
                dry_run: false,
                send_only: true,
                add_msgs: vec![],
            }, currentProject)
            .await
        }
    }
}

fn input_parse_cli() -> InputCli {
    InputCli::parse()
}

async fn stage_open_server(args: InputServeArgs) -> Result<()> {
    let addr: SocketAddr = format!("{}:{}", args.host, args.port)
        .parse()
        .with_context(|| format!("invalid bind address: {}:{}", args.host, args.port))?;
    let state = AppState::default();
    let app = sever_router(state);
    println!(
        "server protocol=http+json callback_url=http://{}:{}/v1/results",
        args.host, args.port
    );
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn sever_router(state: AppState) -> Router {
    Router::new()
        .route("/v1/health", get(stage_health))
        .route("/v1/results", post(stage_receive_result))
        .with_state(state)
}

async fn stage_health() -> &'static str {
    "ok"
}

async fn stage_receive_result(
    State(state): State<AppState>,
    Json(payload): Json<WorkerEnvelope>,
) -> (StatusCode, String) {
    let mut results = state.results.lock().await;
    results.push(payload.clone());

    if !state.quiet_server_logs {
        let stdout_line = payload.result.stdout.trim().replace('\n', " ");
        if payload.result.exit_code == 0 {
            println!(
                "[result] worker={} finished={} code={} message={} response={}",
                payload.result.worker_id,
                payload.result.codex_finished,
                payload.result.exit_code,
                payload.result.command_message,
                stdout_line
            );
        } else {
            let stderr_line = payload.result.stderr.trim().replace('\n', " ");
            println!(
                "[result] worker={} finished={} code={} message={} response={} stderr={}",
                payload.result.worker_id,
                payload.result.codex_finished,
                payload.result.exit_code,
                payload.result.command_message,
                stdout_line,
                stderr_line
            );
        }
    }
    (StatusCode::OK, "accepted".to_string())
}

async fn stage_start_parallel(
    args: InputRunParallelArgs,
    ui_sender: Option<UnboundedSender<WorkingPaneEvent>>,
) -> Result<()> {
    if args.n == 0 {
        bail!("n must be >= 1");
    }
    if args.msgs.is_empty() {
        bail!("msgs must not be empty");
    }

    let callback_url = normalize_server_url(&args.server_url);
    let ai_options = resolve_ai_options(args.codex_bin.clone());
    let server_info = ServerInfo {
        protocol: "http+json".to_string(),
        callback_url: callback_url.clone(),
    };

    let mut handles = Vec::with_capacity(args.n);
    for worker_id in 0..args.n {
        let command_message = args.msgs[worker_id % args.msgs.len()].clone();
        let prompt = build_prompt(&server_info, worker_id, &command_message);
        let worker_server_info = server_info.clone();
        let worker_codex_bin = ai_options.model.clone();
        let worker_ai_auto = ai_options.auto;
        let worker_callback_url = callback_url.clone();
        let worker_dry_run = args.dry_run;
        let worker_send_only = args.send_only;
        let worker_ui_sender = ui_sender.clone();

        if let Some(sender) = &ui_sender {
            let _ = sender.send(WorkingPaneEvent::SetRunning { worker_id });
        }

        handles.push(tokio::spawn(async move {
            stage_run_worker(
                worker_id,
                worker_codex_bin,
                worker_ai_auto,
                worker_dry_run,
                worker_server_info,
                worker_callback_url,
                command_message,
                prompt,
                worker_send_only,
                worker_ui_sender,
            )
            .await
        }));
    }

    for handle in handles {
        let join = handle.await.context("worker task join failed")?;
        join?;
    }
    Ok(())
}

async fn stage_run_test(args: InputRunTestArgs, current_project: String) -> Result<()> {
    let state = AppState {
        quiet_server_logs: true,
        ..AppState::default()
    };
    let app = sever_router(state.clone());
    let addr: SocketAddr = format!("{}:{}", args.host, args.port)
        .parse()
        .with_context(|| format!("invalid bind address: {}:{}", args.host, args.port))?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!(
        "server protocol=http+json callback_url=http://{}:{}/v1/results",
        args.host, args.port
    );
    let server_task = tokio::spawn(async move { axum::serve(listener, app).await });

    ensure_project_yaml_location(&current_project)?;
    let task_file_name = resolve_task_blueprint_file_name(&current_project)?;
    let task_spec_path = resolve_project_dir(&current_project).join("spec.yaml");
    let task_spec_path_for_post = task_spec_path.clone();
    let request_messages = load_task_messages(&current_project, &task_file_name)?;
    let worker_requests = request_messages.clone();
    let (ui_tx, ui_rx) = tokio::sync::mpsc::unbounded_channel::<WorkingPaneEvent>();
    let (run_start_tx, mut run_start_rx) = tokio::sync::mpsc::unbounded_channel::<()>();
    let ui_handle = tokio::spawn(menu_function(
        worker_requests,
        task_spec_path,
        ui_rx,
        run_start_tx,
    ));

    let started = run_start_rx.recv().await;
    if started.is_none() {
        let _ = ui_handle.await?;
        server_task.abort();
        let _ = server_task.await;
        return Ok(());
    }
    tokio::time::sleep(std::time::Duration::from_millis(120)).await;
    let result = run_tasks_parallel(
        format!("http://{}:{}", args.host, args.port),
        current_project.as_str(),
        &task_file_name,
        args.codex_bin,
        args.dry_run,
        args.send_only,
        Some(ui_tx.clone()),
        args.add_msgs,
    )
    .await;
    if result.is_ok() {
        if let Err(err) = run_post_parallel_review_and_update_spec(&task_spec_path_for_post).await {
            eprintln!("post-review failed: {err:#}");
        }
    }
    let _ = ui_tx.send(WorkingPaneEvent::Finish);
    let _ = ui_handle.await?;

    server_task.abort();
    let _ = server_task.await;

    let received = state.results.lock().await.clone();
    println!("received_results={}", received.len());
    for item in received {
        if item.result.exit_code == 0 {
            println!(
                "worker={} request={} finished={} response={} exit_code={}",
                item.result.worker_id,
                item.result.command_message,
                item.result.codex_finished,
                item.result.stdout.trim().replace('\n', " | "),
                item.result.exit_code
            );
        } else {
            println!(
                "worker={} request={} finished={} response={} stderr={} exit_code={}",
                item.result.worker_id,
                item.result.command_message,
                item.result.codex_finished,
                item.result.stdout.trim().replace('\n', " | "),
                item.result.stderr.trim().replace('\n', " | "),
                item.result.exit_code
            );
        }
    }
    result
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SpecYamlDoc {
    #[serde(default)]
    name: String,
    #[serde(default)]
    framework: String,
    #[serde(default)]
    rule: Vec<String>,
    #[serde(default)]
    features: SpecFeatures,
    #[serde(default)]
    tasks: Vec<SpecTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SpecFeatures {
    #[serde(default, deserialize_with = "deserialize_string_or_vec_main")]
    domain: Vec<String>,
    #[serde(default)]
    feature: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SpecTask {
    #[serde(default)]
    name: String,
    #[serde(default, rename = "type")]
    task_type: String,
    #[serde(default)]
    domain: Vec<String>,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    scope: Vec<String>,
    #[serde(default)]
    state: Vec<String>,
    #[serde(default)]
    rule: Vec<String>,
    #[serde(default)]
    step: Vec<String>,
}

async fn stage_build_spec(args: InputBuildSpecArgs) -> Result<()> {
    ensure_project_yaml_location(&args.project)?;
    let project_name = input_ask_question("프로젝트 이름:".to_string(), InputAnswerKind::Text, None)?;
    let description = input_ask_question("설명:".to_string(), InputAnswerKind::Text, None)?;
    let framework = input_ask_question("사용 언어/프레임워크:".to_string(), InputAnswerKind::Text, None)?;
    let libraries = input_ask_question("라이브러리(쉼표 구분):".to_string(), InputAnswerKind::Text, None)?;
    let wanted_feature = input_ask_question("원하는 기능:".to_string(), InputAnswerKind::Text, None)?;

    let template = load_embedded_template_text("templates/spec.yaml")
        .unwrap_or_else(|| "name: \"\"\nframework: \"\"\nrule: []\nfeatures:\n  domain: []\n  feature: []\ntasks: []\n".to_string());
    let prompt = format!(
        "다음 입력을 기반으로 spec.yaml을 작성해줘.\n\
규칙:\n\
- templates/spec.yaml 형식을 반드시 따를 것\n\
- tasks는 최소 3개 이상 채울 것\n\
- tasks 각 항목은 name,type,domain,depends_on,scope,state,rule,step 키를 포함할 것\n\
- 순수 YAML만 출력\n\n\
입력:\n\
- name: {project_name}\n\
- description: {description}\n\
- framework: {framework}\n\
- libraries: {libraries}\n\
- wanted_feature: {wanted_feature}\n\n\
template:\n{template}"
    );
    let raw_output = execute_codex_prompt_for_post_review(&prompt).await?;
    let spec = parse_spec_yaml_from_codex(&raw_output)?;
    let spec_path = resolve_project_dir(&args.project).join("spec.yaml");
    std::fs::write(&spec_path, serde_yaml::to_string(&spec)?)
        .with_context(|| format!("failed to write generated spec: {}", spec_path.display()))?;
    println!("generated spec: {}", spec_path.display());
    Ok(())
}

fn stage_fill_spec_from_input(args: InputFillSpecFromInputArgs) -> Result<()> {
    ensure_project_yaml_location(&args.project)?;
    let input_path = std::path::Path::new(&args.input_path);
    let raw = std::fs::read_to_string(input_path)
        .with_context(|| format!("failed to read input txt: {}", input_path.display()))?;
    let parsed_tasks = parse_tasks_from_input_txt(&raw);
    let spec_path = resolve_project_dir(&args.project).join("spec.yaml");
    let mut spec = if spec_path.exists() {
        let spec_raw = std::fs::read_to_string(&spec_path)
            .with_context(|| format!("failed to read spec: {}", spec_path.display()))?;
        serde_yaml::from_str::<SpecYamlDoc>(&spec_raw).unwrap_or_default()
    } else {
        SpecYamlDoc::default()
    };
    if parsed_tasks.is_empty() {
        bail!("no task items parsed from input.txt");
    }
    spec.tasks = parsed_tasks;
    std::fs::write(&spec_path, serde_yaml::to_string(&spec)?)
        .with_context(|| format!("failed to write spec: {}", spec_path.display()))?;
    println!("updated tasks from {} -> {}", input_path.display(), spec_path.display());
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TodoDoc {
    #[serde(default)]
    tasks: Vec<TodoItem>,
    #[serde(default)]
    todos: Vec<TodoItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TodoItem {
    #[serde(default)]
    name: String,
    #[serde(default, rename = "type")]
    task_type: String,
    #[serde(default)]
    domain: Vec<String>,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    scope: Vec<String>,
    #[serde(default)]
    state: Vec<String>,
    #[serde(default)]
    rule: Vec<String>,
    #[serde(default)]
    step: Vec<String>,
}

async fn stage_make_todos(args: InputMakeTodosArgs) -> Result<()> {
    ensure_project_yaml_location(&args.project)?;
    let project_dir = resolve_project_dir(&args.project);
    let spec_path = project_dir.join("spec.yaml");
    let todos_path = project_dir.join("todos.yaml");

    let spec_text = std::fs::read_to_string(&spec_path)
        .with_context(|| format!("failed to read spec: {}", spec_path.display()))?;
    let template = load_embedded_template_text("templates/todos.yaml")
        .unwrap_or_else(|| "tasks: []\n".to_string());
    let domains = extract_domains_from_spec_yaml(&spec_text);
    let domain_text = if domains.is_empty() {
        "(none)".to_string()
    } else {
        domains.join(", ")
    };
    let prompt = format!(
        "spec.yaml을 기준으로 todos.yaml에 append할 tasks를 작성해줘.\n\
규칙:\n\
- 순수 YAML만 출력\n\
- 최상위 키는 tasks만 사용\n\
- todos item 키는 name,type,domain,depends_on,scope,state,rule,step\n\
- domain은 allowed_domains에서 선택\n\
- 기존 todos 전체를 재작성하지 말고 append 대상 tasks만 출력\n\n\
allowed_domains: [{domain_text}]\n\n\
todos template:\n{template}\n\n\
spec.yaml:\n{spec_text}"
    );
    let raw_output = execute_codex_prompt_for_post_review(&prompt).await?;
    let generated = parse_todo_output_from_codex(&raw_output)?;
    if generated.is_empty() {
        bail!("make-todos returned empty tasks");
    }

    let mut existing = load_existing_todos(&todos_path)?;
    existing.extend(generated);
    let out = TodoDoc {
        tasks: existing,
        todos: Vec::new(),
    };
    let yaml = serde_yaml::to_string(&out).context("failed to serialize todos")?;
    let _: serde_yaml::Value = serde_yaml::from_str(&yaml).context("generated todos yaml invalid")?;
    std::fs::write(&todos_path, yaml)
        .with_context(|| format!("failed to write todos: {}", todos_path.display()))?;
    println!("updated todos: {}", todos_path.display());
    Ok(())
}

fn parse_todo_output_from_codex(raw: &str) -> Result<Vec<TodoItem>> {
    let candidate = extract_yaml_candidate(raw);
    let parsed = serde_yaml::from_str::<TodoDoc>(&candidate)
        .map_err(|e| anyhow::anyhow!("failed to parse generated todos yaml: {e}"))?;
    if parsed.tasks.is_empty() {
        Ok(parsed.todos)
    } else {
        Ok(parsed.tasks)
    }
}

fn load_existing_todos(path: &std::path::Path) -> Result<Vec<TodoItem>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read todos: {}", path.display()))?;
    let parsed = serde_yaml::from_str::<TodoDoc>(&raw)
        .with_context(|| format!("failed to parse todos: {}", path.display()))?;
    if parsed.tasks.is_empty() {
        Ok(parsed.todos)
    } else {
        Ok(parsed.tasks)
    }
}

async fn stage_check_last(args: InputCheckLastArgs) -> Result<()> {
    ensure_project_yaml_location(&args.project)?;
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let jj_status = std::process::Command::new("jj")
        .arg("new")
        .arg("-m")
        .arg("refactor: check_last")
        .current_dir(&cwd)
        .status()
        .with_context(|| format!("failed to run jj new in {}", cwd.display()))?;
    if !jj_status.success() {
        bail!("jj new failed with status {}", jj_status);
    }

    let prompt_template = load_embedded_template_text("prompts/prompt_check_last.txt").unwrap_or_else(|| {
        "스킬 사용:\n- /home/tree/ai/skills/functional-code-structure/SKILL.md\n요청:\n코드 개선점이 있으면 수정까지 진행해줘.".to_string()
    });
    let prompt = format!(
        "{prompt_template}\n\n추가 규칙:\n- 현재 작업 트리는 refactor branch에서만 수정한다.\n- 수정 후 핵심 변경 사항을 간단히 요약한다."
    );
    let _ = execute_codex_prompt_for_post_review(&prompt).await?;
    println!("check_last finished");
    Ok(())
}

fn parse_spec_yaml_from_codex(raw: &str) -> Result<SpecYamlDoc> {
    let candidate = extract_yaml_candidate(raw);
    serde_yaml::from_str::<SpecYamlDoc>(&candidate)
        .map_err(|e| anyhow::anyhow!("failed to parse generated spec yaml: {e}"))
}

fn parse_tasks_from_input_txt(raw: &str) -> Vec<SpecTask> {
    let mut tasks = Vec::new();
    let mut current: Option<SpecTask> = None;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix('#') {
            if let Some(item) = current.take() {
                if !item.name.trim().is_empty() {
                    tasks.push(item);
                }
            }
            let name = rest.trim();
            if name.is_empty() {
                continue;
            }
            current = Some(SpecTask {
                name: name.to_string(),
                task_type: "action".to_string(),
                ..SpecTask::default()
            });
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix('>') {
            if let Some(item) = current.as_mut() {
                let step = rest.trim();
                if !step.is_empty() {
                    item.step.push(step.to_string());
                }
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix('-') {
            if let Some(item) = current.as_mut() {
                let rule = rest.trim();
                if !rule.is_empty() {
                    item.rule.push(rule.to_string());
                }
            }
        }
    }
    if let Some(item) = current {
        if !item.name.trim().is_empty() {
            tasks.push(item);
        }
    }
    tasks
}

fn deserialize_string_or_vec_main<'de, D>(deserializer: D) -> std::result::Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        One(String),
        Many(Vec<String>),
    }
    match StringOrVec::deserialize(deserializer)? {
        StringOrVec::One(v) => Ok(if v.trim().is_empty() { Vec::new() } else { vec![v] }),
        StringOrVec::Many(vs) => Ok(vs),
    }
}

fn set_default_requset_messages() -> Vec<String> {
    let mut request_messages = Vec::new();
    add_request_message(&mut request_messages, "숫자이름 말하기");
    add_request_message(&mut request_messages, "한국 관광도시 말하기");
    add_request_message(&mut request_messages, "현재 일본 시각");
    add_request_message(&mut request_messages, "내일 서울 행사 안내");
    request_messages
}

fn add_request_message(request_messages: &mut Vec<String>, message: impl Into<String>) {
    request_messages.push(message.into());
}

fn read_blueprint(project_name: &str, file_name: &str) -> Result<String> {
    let path = resolve_blueprint_file_path(project_name, file_name);
    std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read blueprint yaml: {}", path.display()))
}

fn resolve_task_blueprint_file_name(project_name: &str) -> Result<String> {
    let candidates = ["todos.yaml", "tasks.yaml", "tasks.ymal"];
    for file_name in candidates {
        let path = resolve_blueprint_file_path(project_name, file_name);
        if path.exists() {
            return Ok(file_name.to_string());
        }
    }
    initialize_empty_todos_blueprint(project_name)?;
    Ok("todos.yaml".to_string())
}

fn initialize_empty_todos_blueprint(project_name: &str) -> Result<()> {
    let project_dir = resolve_project_dir(project_name);
    std::fs::create_dir_all(&project_dir).with_context(|| {
        format!(
            "failed to create blueprint directory: {}",
            project_dir.display()
        )
    })?;
    let todos_path = project_dir.join("todos.yaml");
    if !todos_path.exists() {
        let template = load_default_todos_template_text();
        std::fs::write(&todos_path, template).with_context(|| {
            format!(
                "failed to initialize empty todos blueprint: {}",
                todos_path.display()
            )
        })?;
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize, Default)]
struct PostReviewOutput {
    #[serde(default)]
    review: String,
    #[serde(default)]
    feature: Vec<String>,
}

async fn run_post_parallel_review_and_update_spec(spec_path: &std::path::Path) -> Result<()> {
    let spec_text = std::fs::read_to_string(spec_path)
        .with_context(|| format!("failed to read spec for post-review: {}", spec_path.display()))?;
    let domains = extract_domains_from_spec_yaml(&spec_text);
    let prompt = build_post_parallel_review_prompt(&spec_text, &domains);
    let raw_output = execute_codex_prompt_for_post_review(&prompt).await?;
    let review = parse_post_review_output(&raw_output)?;
    append_features_to_spec(spec_path, &review.feature)?;
    println!("post-review: {}", review.review.trim());
    Ok(())
}

fn build_post_parallel_review_prompt(spec_text: &str, domains: &[String]) -> String {
    let domain_text = if domains.is_empty() {
        "(none)".to_string()
    } else {
        domains.join(", ")
    };
    format!(
        "병렬 기능 구현이 모두 끝났다. 전체 소스코드를 점검하고 리팩토링 가능성을 평가해줘.\n\
그리고 spec.yaml의 features.feature에 추가할 기능 목록을 작성해줘.\n\
규칙:\n\
- 기능 문자열은 반드시 도메인 아래 기능 형태로 작성(예: message.send_note)\n\
- domain은 spec.yaml의 features.domain 후보를 우선 사용\n\
- 중복 기능은 제거\n\
출력 형식:\n\
- 순수 YAML만 출력\n\
- review: string\n\
- feature: string[]\n\n\
allowed_domains: [{domain_text}]\n\n\
spec.yaml:\n{spec_text}"
    )
}

async fn execute_codex_prompt_for_post_review(prompt: &str) -> Result<String> {
    let output_path = std::env::temp_dir().join(format!(
        "orchestra_post_review_{}_{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let output = Command::new("codex")
        .arg("exec")
        .arg("--color")
        .arg("never")
        .arg("-o")
        .arg(&output_path)
        .arg(prompt)
        .output()
        .await
        .context("failed to execute post-review codex")?;
    let fallback_stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let text = std::fs::read_to_string(&output_path).unwrap_or(fallback_stdout);
    let _ = std::fs::remove_file(&output_path);
    if !output.status.success() {
        bail!(
            "post-review codex exited with code {}",
            output.status.code().unwrap_or(-1)
        );
    }
    Ok(text)
}

fn parse_post_review_output(raw: &str) -> Result<PostReviewOutput> {
    let candidate = extract_yaml_candidate(raw);
    serde_yaml::from_str::<PostReviewOutput>(&candidate)
        .map_err(|e| anyhow::anyhow!("failed to parse post-review yaml: {e}"))
}

fn extract_yaml_candidate(raw: &str) -> String {
    if let Some(start) = raw.find("```yaml") {
        let remain = &raw[start + "```yaml".len()..];
        if let Some(end) = remain.find("```") {
            return remain[..end].trim().to_string();
        }
    }
    if let Some(start) = raw.find("```") {
        let remain = &raw[start + 3..];
        if let Some(end) = remain.find("```") {
            return remain[..end].trim().to_string();
        }
    }
    raw.trim().to_string()
}

fn extract_domains_from_spec_yaml(spec_text: &str) -> Vec<String> {
    let parsed = serde_yaml::from_str::<serde_yaml::Value>(spec_text).ok();
    let Some(root) = parsed else {
        return Vec::new();
    };
    let Some(features) = root.get("features") else {
        return Vec::new();
    };
    let Some(domain_value) = features.get("domain") else {
        return Vec::new();
    };
    if let Some(list) = domain_value.as_sequence() {
        return list
            .iter()
            .filter_map(|v| v.as_str().map(ToString::to_string))
            .collect::<Vec<_>>();
    }
    if let Some(one) = domain_value.as_str() {
        let trimmed = one.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }
        return vec![trimmed.to_string()];
    }
    Vec::new()
}

fn append_features_to_spec(spec_path: &std::path::Path, new_features: &[String]) -> Result<()> {
    let raw = std::fs::read_to_string(spec_path)
        .with_context(|| format!("failed to read spec for feature append: {}", spec_path.display()))?;
    let mut root = serde_yaml::from_str::<serde_yaml::Value>(&raw)
        .with_context(|| format!("failed to parse spec yaml: {}", spec_path.display()))?;

    let root_map = root
        .as_mapping_mut()
        .ok_or_else(|| anyhow::anyhow!("spec root must be mapping"))?;
    let features_key = serde_yaml::Value::from("features");
    if !root_map.contains_key(&features_key) {
        root_map.insert(features_key.clone(), serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    }
    let features_map = root_map
        .get_mut(&features_key)
        .and_then(serde_yaml::Value::as_mapping_mut)
        .ok_or_else(|| anyhow::anyhow!("features must be mapping"))?;
    let feature_key = serde_yaml::Value::from("feature");
    if !features_map.contains_key(&feature_key) {
        features_map.insert(feature_key.clone(), serde_yaml::Value::Sequence(Vec::new()));
    }
    let feature_seq = features_map
        .get_mut(&feature_key)
        .and_then(serde_yaml::Value::as_sequence_mut)
        .ok_or_else(|| anyhow::anyhow!("features.feature must be sequence"))?;

    for item in new_features {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        let already = feature_seq
            .iter()
            .any(|v| v.as_str().map(|s| s == trimmed).unwrap_or(false));
        if !already {
            feature_seq.push(serde_yaml::Value::from(trimmed));
        }
    }

    let output = serde_yaml::to_string(&root).context("failed to serialize updated spec")?;
    std::fs::write(spec_path, output)
        .with_context(|| format!("failed to write updated spec: {}", spec_path.display()))?;
    Ok(())
}

fn load_task_messages(project_name: &str, file_name: &str) -> Result<Vec<String>> {
    let yaml = read_blueprint(project_name, file_name)?;
    if yaml.trim().is_empty() {
        return Ok(Vec::new());
    }
    let parsed: BlueprintFile = serde_yaml::from_str(&yaml)
        .with_context(|| {
            format!(
                "failed to parse yaml: {}",
                resolve_blueprint_file_path(project_name, file_name).display()
            )
        })?;
    let tasks = parsed.tasks.or(parsed.todos).unwrap_or_default();
    Ok(tasks.iter().map(task_to_message).collect())
}

fn task_to_message(task: &BlueprintTaskItem) -> String {
    let scope = task
        .scope
        .iter()
        .map(|v| format!("    - {v}"))
        .collect::<Vec<_>>()
        .join("\n");
    let rule = task
        .rule
        .iter()
        .map(|v| format!("    - {v}"))
        .collect::<Vec<_>>()
        .join("\n");
    let step = task
        .step
        .iter()
        .map(|v| format!("    - {v}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "  - name: {}\n    type: {}\n    scope:\n{}\n    rule:\n{}\n    step:\n{}",
        task.name, task.task_type, scope, rule, step
    )
}

#[allow(clippy::too_many_arguments)]
async fn run_tasks_parallel(
    server_url: String,
    project_name: &str,
    file_name: &str,
    codex_bin: Option<String>,
    dry_run: bool,
    send_only: bool,
    ui_sender: Option<UnboundedSender<WorkingPaneEvent>>,
    extra_messages: Vec<String>,
) -> Result<()> {
    let mut messages = load_task_messages(project_name, file_name)?;
    for msg in extra_messages {
        add_request_message(&mut messages, msg);
    }
    if messages.is_empty() {
        bail!(
            "no todo tasks found. add items to {}/todos.yaml and retry",
            resolve_project_dir(project_name).display()
        );
    }
    stage_send_request_messages_parallel(
        server_url,
        messages,
        codex_bin,
        dry_run,
        send_only,
        ui_sender,
    )
    .await
}

async fn stage_send_request_messages_parallel(
    server_url: String,
    request_messages: Vec<String>,
    codex_bin: Option<String>,
    dry_run: bool,
    send_only: bool,
    ui_sender: Option<UnboundedSender<WorkingPaneEvent>>,
) -> Result<()> {
    let n = request_messages.len();
    stage_start_parallel(
        InputRunParallelArgs {
            server_url,
            n,
            msgs: request_messages,
            codex_bin,
            dry_run,
            send_only,
        },
        ui_sender,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn stage_run_worker(
    worker_id: usize,
    codex_bin: String,
    ai_auto: bool,
    dry_run: bool,
    server: ServerInfo,
    callback_url: String,
    command_message: String,
    prompt: String,
    send_only: bool,
    ui_sender: Option<UnboundedSender<WorkingPaneEvent>>,
) -> Result<()> {
    let worker_result = if send_only {
        stage_send_message_only_and_exit(
            server,
            callback_url,
            codex_bin,
            ai_auto,
            dry_run,
            worker_id,
            command_message,
            prompt,
        )
        .await
    } else {
        stage_send_message_and_receive_response(
            worker_id,
            codex_bin,
            ai_auto,
            dry_run,
            server,
            callback_url,
            command_message,
            prompt,
        )
        .await
    };

    if let Some(sender) = ui_sender {
        match &worker_result {
            Ok(envelope) => {
                let ui_result = if send_only {
                    "(send-only)".to_string()
                } else {
                    extract_result_value_for_ui(&envelope.result.stdout)
                };
                let _ = sender.send(WorkingPaneEvent::SetDone {
                    worker_id,
                    result: ui_result,
                });
            }
            Err(err) => {
                let _ = sender.send(WorkingPaneEvent::SetDone {
                    worker_id,
                    result: format!("error: {err}"),
                });
            }
        }
    }

    worker_result.map(|_| ())
}

async fn stage_send_message_only_and_exit(
    server: ServerInfo,
    callback_url: String,
    codex_bin: String,
    ai_auto: bool,
    dry_run: bool,
    worker_id: usize,
    command_message: String,
    prompt: String,
) -> Result<WorkerEnvelope> {
    let (exit_code, stderr) = stage_execute_codex(worker_id, codex_bin, ai_auto, dry_run, prompt)
        .await
        .map(|(exit_code, _stdout, stderr)| (exit_code, stderr))?;

    let envelope = WorkerEnvelope {
        server,
        result: WorkerCompletion {
            worker_id,
            command_message,
            codex_finished: true,
            exit_code,
            stdout: String::new(),
            stderr,
        },
    };
    stage_send_worker_result_to_server(&callback_url, &envelope).await?;
    Ok(envelope)
}

#[allow(clippy::too_many_arguments)]
async fn stage_send_message_and_receive_response(
    worker_id: usize,
    codex_bin: String,
    ai_auto: bool,
    dry_run: bool,
    server: ServerInfo,
    callback_url: String,
    command_message: String,
    prompt: String,
) -> Result<WorkerEnvelope> {
    let full_prompt = build_prompt_with_postpix(prompt, &command_message)?;
    let (exit_code, stdout, stderr) =
        stage_execute_codex(worker_id, codex_bin, ai_auto, dry_run, full_prompt).await?;
    let normalized_stdout = extrac_postpix_lines(&stdout);

    let envelope = WorkerEnvelope {
        server,
        result: WorkerCompletion {
            worker_id,
            command_message,
            codex_finished: true,
            exit_code,
            stdout: normalized_stdout,
            stderr,
        },
    };
    stage_send_worker_result_to_server(&callback_url, &envelope).await?;
    Ok(envelope)
}

fn build_prompt_with_postpix(base_prompt: String, todo_item_body: &str) -> Result<String> {
    let run_todos_prompt = build_run_todos_prompt(todo_item_body)?;
    let postpix = load_postpix_prompt()?;
    Ok(format!("{base_prompt}\n\n{run_todos_prompt}\n\n{postpix}"))
}

fn extrac_postpix_lines(raw: &str) -> String {
    let mut summary = None::<String>;
    let mut result = None::<String>;
    let mut report = None::<String>;

    for line in raw.lines() {
        let trimmed = line.trim();
        if summary.is_none() && trimmed.starts_with("SUMMARY:") {
            summary = Some(trimmed.to_string());
            continue;
        }
        if result.is_none() && trimmed.starts_with("RESULT:") {
            result = Some(trimmed.to_string());
            continue;
        }
        if report.is_none() && trimmed.starts_with("REPORT:") {
            report = Some(trimmed.to_string());
            continue;
        }
    }

    match (summary, result, report) {
        (Some(s), Some(r), Some(p)) => format!("{s}\n{r}\n{p}"),
        _ => raw.trim().to_string(),
    }
}

fn extract_result_value_for_ui(raw: &str) -> String {
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(pos) = trimmed.find("answer=") {
            let value = trimmed[pos + "answer=".len()..].trim();
            if !value.is_empty() {
                return value.to_string();
            }
        }
    }

    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("RESULT:") {
            let value = value.trim();
            if !value.is_empty() {
                return value.to_string();
            }
        }
    }
    raw.trim().replace('\n', " / ")
}

fn load_postpix_prompt() -> Result<String> {
    if let Ok(path) = std::env::var("ORCHESTRA_POSTPIX_PROMPT_PATH") {
        if let Ok(content) = std::fs::read_to_string(path) {
            return Ok(content);
        }
    }

    if let Some(content) = load_embedded_template_text("prompts/Prompt_Postpix.txt") {
        return Ok(content);
    }

    if let Ok(content) = std::fs::read_to_string(PATH_PROMP_POSTPIX) {
        return Ok(content);
    }

    bail!("failed to read postpix prompt file: {}", PATH_PROMP_POSTPIX)
}

fn build_run_todos_prompt(todo_item_body: &str) -> Result<String> {
    let prompt = load_todos_prompt()?;
    Ok(parse_todos_prompt_template(&prompt, todo_item_body))
}

fn parse_todos_prompt_template(prompt: &str, todos_template: &str) -> String {
    prompt.replace(TODO_BODY, todos_template)
}

fn load_todos_prompt() -> Result<String> {
    if let Some(content) = load_embedded_template_text("prompts/Prompt_Todos.txt") {
        return Ok(content);
    }
    if let Ok(content) = std::fs::read_to_string(PATH_TODOS_PROMPT) {
        return Ok(content);
    }
    bail!("failed to read run_todos prompt file: {}", PATH_TODOS_PROMPT);
}

fn load_embedded_template_text(path_from_template_root: &str) -> Option<String> {
    DIR_TEMPLATE
        .get_file(path_from_template_root)
        .and_then(|f| f.contents_utf8())
        .map(ToString::to_string)
}

async fn stage_execute_codex(
    worker_id: usize,
    codex_bin: String,
    ai_auto: bool,
    dry_run: bool,
    prompt: String,
) -> Result<(i32, String, String)> {
    if dry_run {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        return Ok((0, format!("dry-run worker {}", worker_id), String::new()));
    }

    let output_last_message_path = codex_last_message_output_path(worker_id);
    let mut command = Command::new(&codex_bin);
    command.arg("exec");
    if ai_auto {
        command.arg("--dangerously-bypass-approvals-and-sandbox");
    }
    let output = command
        .arg("--color")
        .arg("never")
        .arg("-o")
        .arg(&output_last_message_path)
        .arg(prompt)
        .output()
        .await
        .with_context(|| format!("failed to execute {} for worker {}", codex_bin, worker_id))?;

    let fallback_stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let last_message = std::fs::read_to_string(&output_last_message_path).unwrap_or(fallback_stdout);
    let _ = std::fs::remove_file(&output_last_message_path);

    Ok((
        output.status.code().unwrap_or(-1),
        last_message,
        String::from_utf8_lossy(&output.stderr).to_string(),
    ))
}

fn codex_last_message_output_path(worker_id: usize) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "orchestra_codex_last_message_{}_{}.txt",
        std::process::id(),
        worker_id
    ))
}

fn build_prompt(server: &ServerInfo, _worker_id: usize, message: &str) -> String {
    format!(
        "Server protocol is {protocol}. Callback URL is {url}. Do not perform network calls yourself; only return the requested formatted output. Task: {message}",
        protocol = server.protocol,
        url = server.callback_url
    )
}

fn normalize_server_url(input: &str) -> String {
    let trimmed = input.trim_end_matches('/');
    format!("{trimmed}/v1/results")
}

fn resolve_ai_options(cli_value: Option<String>) -> AiRuntimeOptions {
    let mut model = "codex".to_string();
    let mut auto = false;

    if let Some(cfg) = load_ai_config_from_config() {
        if let Some(v) = cfg.model.as_deref() {
            let trimmed = v.trim();
            if !trimmed.is_empty() {
                model = trimmed.to_string();
            }
        }
        auto = cfg.auto.unwrap_or(false);
    }

    if let Some(v) = cli_value {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            model = trimmed.to_string();
        }
    }

    AiRuntimeOptions { model, auto }
}

fn load_ai_config_from_config() -> Option<AiConfig> {
    let content = load_app_config_text()?;
    let parsed: AppConfigFile = serde_yaml::from_str(&content).ok()?;
    parsed.ai
}

fn resolve_project_base_path() -> std::path::PathBuf {
    get_root_dir()
}

fn get_root_dir() -> std::path::PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let root = find_git_root_dir(&cwd).unwrap_or(cwd);
    normalize_root_dir(root)
}

fn find_git_root_dir(start: &std::path::Path) -> Option<std::path::PathBuf> {
    let search_limit = std::path::Path::new("/home/tree");
    let mut current = Some(start.to_path_buf());
    while let Some(path) = current {
        if path == search_limit {
            break;
        }
        if path.join(".git").exists() {
            return Some(path);
        }
        current = path.parent().map(std::path::Path::to_path_buf);
    }
    None
}

fn normalize_root_dir(path: std::path::PathBuf) -> std::path::PathBuf {
    let is_dot_project = path
        .file_name()
        .and_then(|v| v.to_str())
        .map(|v| v == ".project")
        .unwrap_or(false);
    if is_dot_project {
        return path
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or(path);
    }
    path
}

fn ensure_git_repository_for_root() -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    if find_git_root_dir(&cwd).is_some() {
        return Ok(());
    }

    let answer = input_ask_question(
        "git 저장소가 없습니다. 현재 위치에 `jj git init --colocate`를 실행할까요? (y/n)".to_string(),
        InputAnswerKind::YesNo,
        None,
    )?;
    if answer == "yes" {
        let status = std::process::Command::new("jj")
            .arg("git")
            .arg("init")
            .arg("--colocate")
            .current_dir(&cwd)
            .status()
            .with_context(|| format!("failed to run `jj git init --colocate` in {}", cwd.display()))?;
        if !status.success() {
            bail!("`jj git init --colocate` failed with status {}", status);
        }
    }
    Ok(())
}

fn resolve_project_dir(project_name: &str) -> std::path::PathBuf {
    resolve_project_base_path()
        .join(".project")
        .join(project_name)
}

fn resolve_blueprint_file_path(project_name: &str, file_name: &str) -> std::path::PathBuf {
    resolve_project_dir(project_name).join(file_name)
}

fn load_app_config_text() -> Option<String> {
    std::fs::read_to_string(std::path::Path::new("configs/app.yaml")).ok()
}

fn load_default_todos_template_text() -> String {
    load_embedded_template_text("templates/todos.yaml")
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "tasks: []\n".to_string())
}

fn ensure_project_yaml_location(project_name: &str) -> Result<()> {
    let base = resolve_project_base_path();
    let current_dir = base.join(".project").join(project_name);
    std::fs::create_dir_all(&current_dir)
        .with_context(|| format!("failed to create .project dir: {}", current_dir.display()))?;

    let legacy_dir = base.join("project").join(project_name);
    let target_todos = current_dir.join("todos.yaml");
    if !target_todos.exists() {
        let legacy_todos = legacy_dir.join("todos.yaml");
        let legacy_tasks_yaml = legacy_dir.join("tasks.yaml");
        let legacy_tasks_ymal = legacy_dir.join("tasks.ymal");
        if legacy_todos.exists() {
            std::fs::copy(&legacy_todos, &target_todos).with_context(|| {
                format!(
                    "failed to migrate todos.yaml: {} -> {}",
                    legacy_todos.display(),
                    target_todos.display()
                )
            })?;
        } else if legacy_tasks_yaml.exists() {
            std::fs::copy(&legacy_tasks_yaml, &target_todos).with_context(|| {
                format!(
                    "failed to migrate tasks.yaml: {} -> {}",
                    legacy_tasks_yaml.display(),
                    target_todos.display()
                )
            })?;
        } else if legacy_tasks_ymal.exists() {
            std::fs::copy(&legacy_tasks_ymal, &target_todos).with_context(|| {
                format!(
                    "failed to migrate tasks.ymal: {} -> {}",
                    legacy_tasks_ymal.display(),
                    target_todos.display()
                )
            })?;
        } else {
            let template = load_default_todos_template_text();
            std::fs::write(&target_todos, template).with_context(|| {
                format!(
                    "failed to initialize todos.yaml from template: {}",
                    target_todos.display()
                )
            })?;
        }
    } else if std::fs::read_to_string(&target_todos)
        .map(|v| v.trim().is_empty())
        .unwrap_or(false)
    {
        let template = load_default_todos_template_text();
        std::fs::write(&target_todos, template).with_context(|| {
            format!(
                "failed to normalize empty todos blueprint: {}",
                target_todos.display()
            )
        })?;
    }

    let target_spec = current_dir.join("spec.yaml");
    if !target_spec.exists() {
        let legacy_spec = legacy_dir.join("spec.yaml");
        let template_spec = resolve_project_base_path().join("assets").join("templates").join("spec.yaml");
        if legacy_spec.exists() {
            std::fs::copy(&legacy_spec, &target_spec).with_context(|| {
                format!(
                    "failed to migrate spec.yaml: {} -> {}",
                    legacy_spec.display(),
                    target_spec.display()
                )
            })?;
        } else if template_spec.exists() {
            std::fs::copy(&template_spec, &target_spec).with_context(|| {
                format!(
                    "failed to initialize spec.yaml: {} -> {}",
                    template_spec.display(),
                    target_spec.display()
                )
            })?;
        }
    }
    Ok(())
}

async fn client_send_completion(callback_url: &str, payload: &WorkerEnvelope) -> Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .post(callback_url)
        .json(payload)
        .send()
        .await
        .with_context(|| format!("failed to send completion to {}", callback_url))?;
    if !response.status().is_success() {
        bail!("server returned status {}", response.status());
    }
    Ok(())
}

async fn menu_function(
    worker_requests: Vec<String>,
    task_spec_path: std::path::PathBuf,
    ui_rx: tokio::sync::mpsc::UnboundedReceiver<WorkingPaneEvent>,
    run_start_tx: tokio::sync::mpsc::UnboundedSender<()>,
) -> Result<()> {
    compoents::working_pane::stage_run_working_pane(
        worker_requests,
        task_spec_path,
        ui_rx,
        run_start_tx,
    )
    .await
}

async fn stage_send_worker_result_to_server(
    callback_url: &str,
    payload: &WorkerEnvelope,
) -> Result<()> {
    client_send_completion(callback_url, payload).await
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use super::{
        AppState, InputRunParallelArgs, ServerInfo, build_prompt,
        build_prompt_with_postpix, build_run_todos_prompt, sever_router,
        add_request_message,
        set_default_requset_messages,
        extrac_postpix_lines, extract_result_value_for_ui, load_embedded_template_text,
        find_git_root_dir,
        load_postpix_prompt, load_todos_prompt,
        normalize_server_url, parse_todos_prompt_template, resolve_ai_options,
        stage_start_parallel,
    };
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn normalize_server_url_appends_results_path() {
        assert_eq!(
            normalize_server_url("http://127.0.0.1:7878/"),
            "http://127.0.0.1:7878/v1/results"
        );
    }

    #[test]
    fn resolve_ai_options_prefers_cli_model() {
        let options = resolve_ai_options(Some("my-codex".to_string()));
        assert_eq!(options.model, "my-codex");
    }

    #[test]
    fn prompt_contains_server_info() {
        let server = ServerInfo {
            protocol: "http+json".to_string(),
            callback_url: "http://127.0.0.1:7878/v1/results".to_string(),
        };
        let prompt = build_prompt(&server, 1, "run test");
        assert!(prompt.contains("http+json"));
        assert!(!prompt.contains("worker #1"));
    }

    #[test]
    fn default_request_messages_has_expected_values() {
        let request_messages = set_default_requset_messages();
        assert_eq!(request_messages.len(), 4);
    }

    #[test]
    fn add_request_message_appends_value() {
        let mut request_messages = vec!["base".to_string()];
        add_request_message(&mut request_messages, "extra");
        assert_eq!(request_messages, vec!["base".to_string(), "extra".to_string()]);
    }

    #[test]
    fn postpix_prompt_file_is_loadable() {
        let postpix = load_postpix_prompt().expect("postpix prompt should load");
        assert!(postpix.contains("REPORT:"));
    }

    #[test]
    fn embedded_template_is_loadable() {
        let postpix = load_embedded_template_text("prompts/Prompt_Postpix.txt")
            .expect("embedded template should load");
        assert!(postpix.contains("REPORT:"));
    }

    #[test]
    fn postpix_prompt_is_appended_to_base_prompt() {
        let combined = build_prompt_with_postpix(
            "base prompt".to_string(),
            "  - name: sample\n    type: action",
        )
        .expect("prompt should work");
        assert!(combined.contains("REPORT:"));
    }

    #[test]
    fn run_todos_prompt_is_loadable() {
        let prompt = load_todos_prompt().expect("run_todos prompt should load");
        assert!(prompt.contains("Step 순서"));
    }

    #[test]
    fn run_todos_prompt_includes_single_item_body() {
        let built = build_run_todos_prompt("  - name: dday-calculation")
            .expect("run_todos prompt should be built");
        assert!(built.contains("dday-calculation"));
        assert!(!built.contains("{{body}}"));
    }

    #[test]
    fn parse_todos_prompt_template_replaces_placeholder() {
        let prompt = "A\n{{body}}\nB";
        let parsed = parse_todos_prompt_template(prompt, "TODO-BODY");
        assert_eq!(parsed, "A\nTODO-BODY\nB");
    }

    #[test]
    fn find_git_root_dir_uses_nearest_repository() {
        let unique = format!(
            "orchestra_git_root_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should be valid")
                .as_nanos()
        );
        let base = std::env::temp_dir().join(unique);
        let outer = base.join("outer");
        let inner = outer.join("inner");
        let deep = inner.join("nested").join("src");
        fs::create_dir_all(&deep).expect("test directories should be created");
        fs::create_dir_all(outer.join(".git")).expect("outer git dir should exist");
        fs::create_dir_all(inner.join(".git")).expect("inner git dir should exist");

        let resolved = find_git_root_dir(&deep).expect("git root should be found");
        assert_eq!(resolved, inner);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn find_git_root_dir_does_not_return_home_tree_root() {
        let resolved = find_git_root_dir(std::path::Path::new("/home/tree/project/orchestra"));
        assert_ne!(resolved.as_deref(), Some(std::path::Path::new("/home/tree")));
    }

    #[test]
    fn extract_postpix_lines_keeps_only_required_three_lines() {
        let raw = "noise\nSUMMARY: s\nother\nRESULT: r\nREPORT: task complete - answer=a\nextra";
        assert_eq!(
            extrac_postpix_lines(raw),
            "SUMMARY: s\nRESULT: r\nREPORT: task complete - answer=a"
        );
    }

    #[test]
    fn extract_result_value_for_ui_prefers_answer_value() {
        let raw = "SUMMARY: s\nRESULT: 제주\nREPORT: task complete - answer=부산";
        assert_eq!(extract_result_value_for_ui(raw), "부산");
    }

    #[tokio::test]
    async fn parallel_workers_send_fruit_result_after_sleep() {
        let state = AppState::default();
        let app = sever_router(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind should succeed");
        let addr = listener.local_addr().expect("local addr should exist");
        let server_task = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("server should run");
        });

        let fake_codex = make_fake_codex_script().expect("fake codex script should be created");
        let args = InputRunParallelArgs {
            server_url: format!("http://{}", addr),
            n: 3,
            msgs: vec![
                "worker task one".to_string(),
                "worker task two".to_string(),
                "worker task three".to_string(),
            ],
            codex_bin: Some(fake_codex.to_string_lossy().to_string()),
            dry_run: false,
            send_only: false,
        };

        let started = Instant::now();
        stage_start_parallel(args, None)
            .await
            .expect("parallel stage should succeed");
        let elapsed = started.elapsed();

        tokio::time::sleep(Duration::from_millis(300)).await;
        let received = state.results.lock().await.clone();
        server_task.abort();
        let _ = fs::remove_file(&fake_codex);

        assert_eq!(received.len(), 3);
        assert!(elapsed < Duration::from_secs(9));
    }

    #[tokio::test]
    async fn send_only_mode_still_reports_codex_finished_signal() {
        let state = AppState::default();
        let app = sever_router(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind should succeed");
        let addr = listener.local_addr().expect("local addr should exist");
        let server_task = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("server should run");
        });

        let fake_codex = make_fake_codex_script().expect("fake codex script should be created");
        let args = InputRunParallelArgs {
            server_url: format!("http://{}", addr),
            n: 2,
            msgs: vec!["worker task one".to_string(), "worker task two".to_string()],
            codex_bin: Some(fake_codex.to_string_lossy().to_string()),
            dry_run: false,
            send_only: true,
        };

        stage_start_parallel(args, None)
            .await
            .expect("parallel stage should succeed");
        tokio::time::sleep(Duration::from_millis(300)).await;
        let received = state.results.lock().await.clone();
        server_task.abort();
        let _ = fs::remove_file(&fake_codex);

        assert_eq!(received.len(), 2);
    }

    fn make_fake_codex_script() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join(format!("fake_codex_{}", std::process::id()));
        let script = r#"#!/usr/bin/env bash
if [ "$1" = "exec" ]; then
  sleep 5
  echo apple
else
  echo "unsupported args" >&2
  exit 2
fi
"#;
        fs::write(&path, script)?;
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&path, perms)?;
        }
        Ok(path)
    }
}
