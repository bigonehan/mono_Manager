use crate::ProjectRecord;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Terminal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub fn render_project_list(projects: &[ProjectRecord]) -> String {
    const MAX_CARDS: usize = 9;
    const COLUMNS: usize = 3;
    const CARD_WIDTH: usize = 38;
    const GAP: &str = "  ";

    if projects.is_empty() {
        return "등록된 프로젝트가 없습니다.\n".to_string();
    }

    let visible = &projects[..projects.len().min(MAX_CARDS)];
    let mut out = String::new();

    for row in visible.chunks(COLUMNS) {
        let mut top = Vec::new();
        let mut name = Vec::new();
        let mut desc = Vec::new();
        let mut meta = Vec::new();
        let mut bottom = Vec::new();

        for p in row {
            let title = format!(
                " {}{}",
                if p.selected { "* " } else { "" },
                truncate_with_ellipsis(&p.name, CARD_WIDTH - 4)
            );
            let body_desc = truncate_with_ellipsis(&p.description, CARD_WIDTH - 4);
            let body_meta = truncate_with_ellipsis(
                &format!("created:{} updated:{}", p.created_at, p.updated_at),
                CARD_WIDTH - 4,
            );
            top.push(format!("+{}+", "-".repeat(CARD_WIDTH - 2)));
            name.push(format!("| {:<width$} |", title, width = CARD_WIDTH - 4));
            desc.push(format!("| {:<width$} |", body_desc, width = CARD_WIDTH - 4));
            meta.push(format!("| {:<width$} |", body_meta, width = CARD_WIDTH - 4));
            bottom.push(format!("+{}+", "-".repeat(CARD_WIDTH - 2)));
        }

        out.push_str(&top.join(GAP));
        out.push('\n');
        out.push_str(&name.join(GAP));
        out.push('\n');
        out.push_str(&desc.join(GAP));
        out.push('\n');
        out.push_str(&meta.join(GAP));
        out.push('\n');
        out.push_str(&bottom.join(GAP));
        out.push('\n');
        out.push('\n');
    }

    if projects.len() > MAX_CARDS {
        out.push_str(&format!(
            "... 외 {}개 프로젝트가 더 있습니다.\n",
            projects.len() - MAX_CARDS
        ));
    }

    out.trim_end().to_string() + "\n"
}

fn truncate_with_ellipsis(value: &str, max_chars: usize) -> String {
    let mut iter = value.chars();
    let count = iter.clone().count();
    if count <= max_chars {
        return value.to_string();
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }
    let keep = max_chars - 3;
    iter.by_ref().take(keep).collect::<String>() + "..."
}

pub fn render_task_validation(runnable: &[String], blocked: &[String]) -> String {
    let mut out = String::new();
    out.push_str("runnable:\n");
    for name in runnable {
        out.push_str(&format!("- {}\n", name));
    }
    out.push_str("blocked:\n");
    for name in blocked {
        out.push_str(&format!("- {}\n", name));
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskRuntimeState {
    Inactive,
    Active,
    Clear,
}

pub fn render_task_runtime_status(task_name: &str, state: TaskRuntimeState) -> String {
    let badge = match state {
        TaskRuntimeState::Inactive => "[ ]",
        TaskRuntimeState::Active => "[>]",
        TaskRuntimeState::Clear => "[x]",
    };
    format!("{} {}", badge, task_name)
}

pub fn render_parallel_modal(statuses: &[(String, TaskRuntimeState)]) -> String {
    let mut out = String::new();
    out.push_str("parallel task modal\n");
    for (task_name, state) in statuses {
        out.push_str(&render_task_runtime_status(task_name, *state));
        out.push('\n');
    }
    out
}

#[derive(Debug, Clone, Copy)]
pub enum DraftCommand {
    Create,
    Add,
    Delete,
}

pub fn action_resolve_feature_draft_path(feature_name: &str) -> PathBuf {
    PathBuf::from(".project")
        .join("feature")
        .join(feature_name)
        .join("draft.yaml")
}

pub fn action_apply_draft_create_update_delete(
    command: DraftCommand,
    feature_name: &str,
    patch_content: Option<&str>,
) -> Result<PathBuf, String> {
    let draft_path = action_resolve_feature_draft_path(feature_name);
    let parent = draft_path
        .parent()
        .ok_or_else(|| "failed to resolve draft parent path".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;

    match command {
        DraftCommand::Create => {
            let template_path = action_resolve_draft_template_path()?;
            let template = fs::read_to_string(&template_path)
                .map_err(|e| format!("failed to read {}: {}", template_path.display(), e))?;
            fs::write(&draft_path, template)
                .map_err(|e| format!("failed to write {}: {}", draft_path.display(), e))?;
        }
        DraftCommand::Add => {
            let patch = patch_content.unwrap_or("");
            let mut existing = fs::read_to_string(&draft_path).unwrap_or_default();
            if !existing.ends_with('\n') && !existing.is_empty() {
                existing.push('\n');
            }
            existing.push_str(patch);
            fs::write(&draft_path, existing)
                .map_err(|e| format!("failed to update {}: {}", draft_path.display(), e))?;
        }
        DraftCommand::Delete => {
            if draft_path.exists() {
                fs::remove_file(&draft_path)
                    .map_err(|e| format!("failed to delete {}: {}", draft_path.display(), e))?;
            }
        }
    }
    Ok(draft_path)
}

#[derive(Debug, Clone, Deserialize)]
struct PaneStyleValue {
    border: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PaneStyleDoc {
    active: Option<PaneStyleValue>,
    normal: Option<PaneStyleValue>,
    inactive: Option<PaneStyleValue>,
}

#[derive(Debug, Clone, Copy)]
struct BorderPalette {
    active: Color,
    normal: Color,
    inactive: Color,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
struct DraftsListDoc {
    #[serde(default)]
    feature: Vec<String>,
    #[serde(default)]
    planned: Vec<String>,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct BootstrapRulesDoc {
    #[serde(default)]
    rules: Vec<BootstrapRule>,
}

#[derive(Debug, Clone, Deserialize)]
struct BootstrapRule {
    #[serde(default)]
    name: String,
    #[serde(default)]
    match_any: Vec<String>,
    #[serde(default)]
    template: String,
}

#[derive(Debug, Clone)]
struct CreateProjectModal {
    mode: ProjectModalMode,
    source_index: Option<usize>,
    original_path: String,
    name: String,
    description: String,
    spec: String,
    path: String,
    name_is_default: bool,
    description_is_default: bool,
    spec_is_default: bool,
    path_is_default: bool,
    field_index: usize,
    confirm_selected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectModalMode {
    Create,
    Edit,
}

#[derive(Debug, Clone)]
struct PathChangeConfirm {
    source_index: usize,
    new_name: String,
    new_description: String,
    old_path: String,
    new_path: String,
    confirm_selected: bool,
}

#[derive(Debug, Clone)]
struct DeleteProjectConfirm {
    source_index: usize,
    project_name: String,
    project_path: String,
    confirm_selected: bool,
}

#[derive(Debug)]
struct UiApp {
    tab_index: usize,
    project_index: usize,
    pane_focus: usize,
    parallel_statuses: Vec<(String, TaskRuntimeState)>,
    parallel_running: bool,
    last_tick: Instant,
    status_line: String,
    create_modal: Option<CreateProjectModal>,
    detail_fill_confirm: Option<DetailFillConfirm>,
    draft_create_confirm: Option<DraftCreateConfirm>,
    draft_bulk_add_modal: Option<DraftBulkAddModal>,
    list_edit_modal: Option<ListEditModal>,
    bootstrap_confirm: Option<BootstrapConfirm>,
    ai_chat_modal: Option<AiChatModal>,
    path_change_confirm: Option<PathChangeConfirm>,
    delete_confirm: Option<DeleteProjectConfirm>,
    pending_action: Option<PendingUiAction>,
    busy_message: Option<String>,
    menu_active: bool,
    changed: bool,
    pane_activate_started_at: Option<Instant>,
    pane_activate_index: usize,
}

#[derive(Debug, Clone)]
enum PendingUiAction {
    SubmitProjectModal(CreateProjectModal),
    ApplyPathChange {
        confirm: PathChangeConfirm,
        move_dir: bool,
    },
    ApplyDelete {
        confirm: DeleteProjectConfirm,
        accepted: bool,
    },
    ApplyCreateDraft {
        project_index: usize,
    },
    ApplyDraftBulkAdd {
        project_index: usize,
        raw_input: String,
    },
}

#[derive(Debug, Clone)]
struct DetailFillConfirm {
    project_index: usize,
    confirm_selected: bool,
}

#[derive(Debug, Clone)]
struct DraftCreateConfirm {
    project_index: usize,
    confirm_selected: bool,
}

#[derive(Debug, Clone)]
struct DraftBulkAddModal {
    project_index: usize,
    input: String,
    input_focus: bool,
    confirm_selected: bool,
}

#[derive(Debug, Clone, Copy)]
enum ListEditTarget {
    Rule,
    Constraint,
    Feature,
}

#[derive(Debug, Clone, Copy)]
enum ListEditInputMode {
    Add,
    Edit,
}

#[derive(Debug, Clone)]
struct ListEditModal {
    project_index: usize,
    target: ListEditTarget,
    items: Vec<String>,
    selected_index: usize,
    input_mode: Option<ListEditInputMode>,
    input: String,
    confirm_selected: bool,
}

#[derive(Debug, Clone)]
struct BootstrapConfirm {
    project_index: usize,
    spec: String,
    confirm_selected: bool,
}

#[derive(Debug)]
enum AiStreamEvent {
    Chunk(String),
    Done,
    Error(String),
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AiDetailFocus {
    Input,
    CloseButton,
}

#[derive(Debug)]
struct AiChatModal {
    project_index: usize,
    project_path: String,
    project_name: String,
    project_description: String,
    model_bin: String,
    warmup_inflight: bool,
    input: String,
    input_enter_streak: u8,
    focus: AiDetailFocus,
    input_active: bool,
    allow_full_md_response: bool,
    history: Vec<String>,
    streaming: bool,
    streaming_buffer: String,
    stream_rx: Option<Receiver<AiStreamEvent>>,
    stream_cancel: Option<Arc<AtomicBool>>,
}

pub struct UiRunResult {
    pub changed: bool,
    pub message: String,
    pub auto_mode_project: Option<String>,
}

fn calc_parse_color(name: Option<&str>, fallback: Color) -> Color {
    match name.unwrap_or("").to_ascii_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "white" => Color::White,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "dark_gray" => Color::DarkGray,
        _ => fallback,
    }
}

fn action_load_border_palette() -> BorderPalette {
    let root = action_binary_root();
    let candidates = [
        root.join("configs").join("style.yaml"),
        root.join("assets").join("style").join("pane_style.yaml"),
        PathBuf::from("assets").join("style").join("pane_style.yaml"),
        PathBuf::from("configs").join("style.yaml"),
        root.join("src").join("assets").join("style").join("pane_style.yaml"),
        PathBuf::from("src").join("assets").join("style").join("pane_style.yaml"),
    ];

    for path in candidates {
        if !path.exists() {
            continue;
        }
        if let Ok(raw) = fs::read_to_string(&path) {
            if let Ok(doc) = serde_yaml::from_str::<PaneStyleDoc>(&raw) {
                let active = calc_parse_color(
                    doc.active.as_ref().and_then(|v| v.border.as_deref()),
                    Color::Green,
                );
                let normal = calc_parse_color(
                    doc.normal.as_ref().and_then(|v| v.border.as_deref()),
                    Color::Black,
                );
                let inactive = calc_parse_color(
                    doc.inactive.as_ref().and_then(|v| v.border.as_deref()),
                    Color::Gray,
                );
                return BorderPalette {
                    active,
                    normal,
                    inactive,
                };
            }
        }
    }

    BorderPalette {
        active: Color::Green,
        normal: Color::Black,
        inactive: Color::Gray,
    }
}

fn action_binary_root() -> PathBuf {
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            return parent.to_path_buf();
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn action_resolve_draft_template_path() -> Result<PathBuf, String> {
    let root = action_binary_root();
    let candidates = [
        root.join("assets").join("templates").join("draft.yaml"),
        PathBuf::from("assets").join("templates").join("draft.yaml"),
        root.join("src").join("assets").join("templates").join("draft.yaml"),
        PathBuf::from("src").join("assets").join("templates").join("draft.yaml"),
    ];
    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "draft template not found (binary root: {})",
        root.display()
    ))
}

fn action_run_plan_init_in_project_dir(
    project_dir: &Path,
    name: &str,
    description: &str,
    spec: &str,
) -> Result<(), String> {
    let exe = env::current_exe().map_err(|e| format!("failed to resolve current exe: {}", e))?;
    let mut child = Command::new(exe)
        .current_dir(project_dir)
        .arg("plan-init")
        .arg("-n")
        .arg(name)
        .arg("-d")
        .arg(description)
        .arg("-s")
        .arg(spec)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn plan-init: {}", e))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(b"\n\n")
            .map_err(|e| format!("failed to write plan-init stdin: {}", e))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|e| format!("failed to wait plan-init: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Err(format!(
            "plan-init failed (code={:?}) stderr=`{}` stdout=`{}`",
            output.status.code(),
            stderr,
            stdout
        ))
    }
}

fn action_ui_model_bin() -> String {
    let root = action_binary_root();
    let candidates = [
        root.join("configs.yaml"),
        root.join("config.yaml"),
        root.join("assets").join("config").join("config.yaml"),
        root.join("src").join("assets").join("config").join("config.yaml"),
        PathBuf::from("configs.yaml"),
        PathBuf::from("config.yaml"),
        PathBuf::from("assets").join("config").join("config.yaml"),
        PathBuf::from("src").join("assets").join("config").join("config.yaml"),
    ];
    for path in candidates {
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(doc) = serde_yaml::from_str::<serde_yaml::Value>(&raw) else {
            continue;
        };
        let model = doc
            .get("ai")
            .and_then(|v| v.get("model"))
            .and_then(|v| v.as_str())
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        if let Some(model_bin) = model {
            return model_bin;
        }
    }
    "codex".to_string()
}

fn calc_extract_markdown_block(raw: &str) -> Option<String> {
    if let Some(start) = raw.find("```markdown") {
        let rest = &raw[start + 11..];
        if let Some(end) = rest.find("```") {
            return Some(rest[..end].trim().to_string());
        }
    }
    if let Some(start) = raw.find("# info") {
        return Some(raw[start..].trim().to_string());
    }
    None
}

fn action_open_detail_fill_confirm(app: &mut UiApp, project_index: usize) {
    app.detail_fill_confirm = Some(DetailFillConfirm {
        project_index,
        confirm_selected: true,
    });
    app.status_line = "project created: fill detail now? (y/n)".to_string();
}

fn action_apply_draft_create_via_cli(
    projects: &[ProjectRecord],
    app: &mut UiApp,
    project_index: usize,
) -> Result<(), String> {
    let Some(project) = projects.get(project_index) else {
        return Err("selected project index out of range".to_string());
    };
    let exe = env::current_exe().map_err(|e| format!("failed to resolve current exe: {}", e))?;
    let output = Command::new(exe)
        .current_dir(&project.path)
        .arg("create-draft")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("failed to run create-draft: {}", e))?;
    if output.status.success() {
        app.status_line = "draft create requested (create-draft)".to_string();
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!(
            "create-draft failed (code={:?}) {}",
            output.status.code(),
            stderr
        ))
    }
}

fn action_open_draft_bulk_add_modal(app: &mut UiApp, project_index: usize) {
    app.draft_bulk_add_modal = Some(DraftBulkAddModal {
        project_index,
        input: String::new(),
        input_focus: true,
        confirm_selected: true,
    });
    app.status_line = "draft bulk add modal opened".to_string();
}

fn action_apply_draft_bulk_add_via_cli(
    projects: &[ProjectRecord],
    app: &mut UiApp,
    project_index: usize,
    raw_input: &str,
) -> Result<(), String> {
    let Some(project) = projects.get(project_index) else {
        return Err("selected project index out of range".to_string());
    };
    if raw_input.trim().is_empty() {
        return Err("draft add requires non-empty input".to_string());
    }
    let exe = env::current_exe().map_err(|e| format!("failed to resolve current exe: {}", e))?;
    let output = Command::new(&exe)
        .current_dir(&project.path)
        .arg("add-function")
        .arg(raw_input)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("failed to run add-function: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "add-function failed (code={:?}) {}",
            output.status.code(),
            stderr
        ));
    }
    app.status_line = "draft add requested via add-function".to_string();
    Ok(())
}

fn action_render_draft_bulk_add_modal(
    f: &mut ratatui::Frame,
    area: Rect,
    modal: &DraftBulkAddModal,
) -> Option<(u16, u16)> {
    f.render_widget(Clear, area);
    let block = Block::default().title("Add Drafts").borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(2)])
        .split(inner);
    let input_border = if modal.input_focus {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM)
    };
    f.render_widget(
        Paragraph::new(modal.input.clone())
            .block(
                Block::default()
                    .title("Input | multiline, object format: #이름 / >step / -규칙")
                    .borders(Borders::ALL)
                    .border_style(input_border),
            )
            .wrap(Wrap { trim: false }),
        rows[0],
    );
    action_render_confirm_buttons_bottom_right(
        f,
        inner,
        "Confirm",
        "Cancel",
        modal.confirm_selected,
    );
    if modal.input_focus {
        Some(calc_cursor_in_input(rows[0], &modal.input))
    } else {
        None
    }
}

fn action_build_ai_seed_prompt(project: &ProjectRecord, project_md: &str) -> String {
    format!(
        "System:\n`$plan-project-code` 스킬 워크플로우로 진행합니다.\n현재 project.md에는 name/description이 이미 포함되어 있습니다.\n\n\
Context:\nname={}\ndescription={}\npath={}\n\n\
Current project.md:\n{}\n\n\
위 컨텍스트를 내부 기준으로만 저장하고, 출력은 반드시 `READY` 한 단어만 반환하세요.",
        project.name, project.description, project.path, project_md
    )
}

fn action_new_ai_chat_modal_template(
    project: &ProjectRecord,
    project_index: usize,
    model_bin: String,
) -> AiChatModal {
    AiChatModal {
        project_index,
        project_path: project.path.clone(),
        project_name: project.name.clone(),
        project_description: project.description.clone(),
        model_bin,
        warmup_inflight: false,
        input: String::new(),
        input_enter_streak: 0,
        focus: AiDetailFocus::Input,
        input_active: false,
        allow_full_md_response: false,
        history: Vec::new(),
        streaming: false,
        streaming_buffer: String::new(),
        stream_rx: None,
        stream_cancel: None,
    }
}

fn action_start_ai_chat_warmup(modal: &mut AiChatModal, seed_prompt: String) {
    let (seed_rx, seed_cancel) = action_spawn_ai_stream(&modal.model_bin, seed_prompt);
    modal.warmup_inflight = true;
    modal.streaming = true;
    modal.stream_rx = Some(seed_rx);
    modal.stream_cancel = Some(seed_cancel);
}

fn action_open_ai_chat_modal(app: &mut UiApp, projects: &[ProjectRecord], project_index: usize) {
    let Some(project) = projects.get(project_index) else {
        app.status_line = "no project selected".to_string();
        return;
    };
    let project_md_path = Path::new(&project.path).join(".project").join("project.md");
    let project_md = fs::read_to_string(&project_md_path).unwrap_or_default();
    let seed_prompt = action_build_ai_seed_prompt(project, &project_md);
    let model_bin = action_ui_model_bin();
    let mut modal = action_new_ai_chat_modal_template(project, project_index, model_bin);
    modal.input_active = true;
    action_start_ai_chat_warmup(&mut modal, seed_prompt);
    app.ai_chat_modal = Some(modal);
    app.status_line = "ai detail warmup started".to_string();
}

fn action_open_bootstrap_confirm(app: &mut UiApp, projects: &[ProjectRecord], project_index: usize) {
    let Some(project) = projects.get(project_index) else {
        app.status_line = "no project selected".to_string();
        return;
    };
    let spec = action_read_project_md(project)
        .map(|md| action_parse_project_md(&md).spec)
        .unwrap_or_default();
    app.bootstrap_confirm = Some(BootstrapConfirm {
        project_index,
        spec,
        confirm_selected: true,
    });
    app.status_line = "project bootstrap 실행 여부를 선택하세요".to_string();
}

fn calc_extract_yaml_codeblock(raw: &str) -> Option<String> {
    for marker in ["```yaml", "```yml"] {
        if let Some(start) = raw.find(marker) {
            let rest = &raw[start + marker.len()..];
            if let Some(end) = rest.find("```") {
                return Some(rest[..end].trim().to_string());
            }
        }
    }
    None
}

fn action_load_bootstrap_rule_for_spec(spec: &str) -> Option<BootstrapRule> {
    let root = action_binary_root();
    let candidates = [
        root.join("configs").join("bootstrap.md"),
        PathBuf::from("configs").join("bootstrap.md"),
    ];
    let spec_lc = spec.to_ascii_lowercase();
    for path in candidates {
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let Some(yaml_block) = calc_extract_yaml_codeblock(&raw) else {
            continue;
        };
        let Ok(doc) = serde_yaml::from_str::<BootstrapRulesDoc>(&yaml_block) else {
            continue;
        };
        for rule in doc.rules {
            if rule.template.trim().is_empty() {
                continue;
            }
            let matched = rule
                .match_any
                .iter()
                .filter_map(|v| {
                    let v = v.trim().to_ascii_lowercase();
                    if v.is_empty() { None } else { Some(v) }
                })
                .any(|kw| spec_lc.contains(&kw));
            if matched {
                return Some(rule);
            }
        }
    }
    None
}

fn action_apply_bootstrap_node_template(project_root: &Path, project_name: &str) -> Result<(), String> {
    let pkg = project_root.join("package.json");
    if !pkg.exists() {
        let name = project_name.replace(' ', "-").to_ascii_lowercase();
        let raw = format!(
            "{{\n  \"name\": \"{}\",\n  \"version\": \"0.1.0\",\n  \"private\": true,\n  \"scripts\": {{\n    \"dev\": \"echo setup dev\",\n    \"build\": \"echo setup build\"\n  }}\n}}\n",
            name
        );
        fs::write(&pkg, raw).map_err(|e| format!("failed to write {}: {}", pkg.display(), e))?;
    }
    fs::create_dir_all(project_root.join("src"))
        .map_err(|e| format!("failed to create src: {}", e))?;
    Ok(())
}

fn action_apply_bootstrap_rust_template(project_root: &Path, project_name: &str) -> Result<(), String> {
    let cargo_toml = project_root.join("Cargo.toml");
    if !cargo_toml.exists() {
        let name = project_name.replace(' ', "-").to_ascii_lowercase();
        let raw = format!(
            "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
            name
        );
        fs::write(&cargo_toml, raw)
            .map_err(|e| format!("failed to write {}: {}", cargo_toml.display(), e))?;
    }
    let src_dir = project_root.join("src");
    fs::create_dir_all(&src_dir).map_err(|e| format!("failed to create src: {}", e))?;
    let main_rs = src_dir.join("main.rs");
    if !main_rs.exists() {
        fs::write(&main_rs, "fn main() {\n    println!(\"hello\");\n}\n")
            .map_err(|e| format!("failed to write {}: {}", main_rs.display(), e))?;
    }
    Ok(())
}

fn calc_is_bootstrap_target_empty(project_root: &Path) -> Result<bool, String> {
    let entries = fs::read_dir(project_root)
        .map_err(|e| format!("failed to read {}: {}", project_root.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read dir entry: {}", e))?;
        let name = entry.file_name().to_string_lossy().to_string();
        // `.project` is internal metadata; ignore it for bootstrap emptiness check.
        if name == ".project" {
            continue;
        }
        // Hidden entries are not treated as visible project files.
        if name.starts_with('.') {
            continue;
        }
        return Ok(false);
    }
    Ok(true)
}

fn action_write_bootstrap_note(
    project_root: &Path,
    spec: &str,
    reason: &str,
) -> Result<(), String> {
    let note = project_root.join(".project").join("bootstrap.md");
    fs::write(
        &note,
        format!(
            "spec 기반 수동 bootstrap 필요\n\nreason: {}\nspec: {}\n",
            reason, spec
        ),
    )
    .map_err(|e| format!("failed to write {}: {}", note.display(), e))?;
    Ok(())
}

fn action_apply_bootstrap(
    projects: &[ProjectRecord],
    app: &mut UiApp,
    confirm: &BootstrapConfirm,
) -> Result<(), String> {
    let Some(project) = projects.get(confirm.project_index) else {
        return Err("selected project index out of range".to_string());
    };
    let finalize_ui = |app: &mut UiApp| {
        action_cancel_ai_stream(app);
        app.ai_chat_modal = None;
        app.tab_index = 1;
        app.pane_focus = 0;
        app.menu_active = true;
    };
    let project_root = Path::new(&project.path);
    if !calc_is_bootstrap_target_empty(project_root)? {
        app.status_line = "bootstrap skipped: target folder is not empty".to_string();
        finalize_ui(app);
        return Ok(());
    }
    if let Some(rule) = action_load_bootstrap_rule_for_spec(&confirm.spec) {
        match rule.template.trim().to_ascii_lowercase().as_str() {
            "node-react" | "node" | "react" => {
                action_apply_bootstrap_node_template(project_root, &project.name)?;
                app.status_line = format!(
                    "bootstrap completed: {}",
                    if rule.name.trim().is_empty() {
                        "node/react"
                    } else {
                        rule.name.trim()
                    }
                );
                finalize_ui(app);
                return Ok(());
            }
            "rust" => {
                action_apply_bootstrap_rust_template(project_root, &project.name)?;
                app.status_line = format!(
                    "bootstrap completed: {}",
                    if rule.name.trim().is_empty() {
                        "rust"
                    } else {
                        rule.name.trim()
                    }
                );
                finalize_ui(app);
                return Ok(());
            }
            other => {
                action_write_bootstrap_note(
                    project_root,
                    &confirm.spec,
                    &format!("unknown template in configs/bootstrap.md: {}", other),
                )?;
                app.status_line = "bootstrap note created (manual required)".to_string();
                finalize_ui(app);
                return Ok(());
            }
        }
    }
    let spec_lc = confirm.spec.to_ascii_lowercase();
    if spec_lc.contains("react")
        || spec_lc.contains("next")
        || spec_lc.contains("node")
        || spec_lc.contains("typescript")
        || spec_lc.contains("javascript")
    {
        action_apply_bootstrap_node_template(project_root, &project.name)?;
        app.status_line = "bootstrap completed: node/react template".to_string();
        finalize_ui(app);
        return Ok(());
    }
    if spec_lc.contains("rust") {
        action_apply_bootstrap_rust_template(project_root, &project.name)?;
        app.status_line = "bootstrap completed: rust template".to_string();
        finalize_ui(app);
        return Ok(());
    }
    action_write_bootstrap_note(project_root, &confirm.spec, "no matching bootstrap rule")?;
    app.status_line = "bootstrap note created (manual required)".to_string();
    finalize_ui(app);
    Ok(())
}

fn action_build_ai_chat_prompt(modal: &AiChatModal, user_message: &str) -> String {
    let full_md_requested = calc_is_full_project_md_request(user_message);
    format!(
        "당신은 `$plan-project-code` 스킬을 따라 project.md를 완성하는 도우미다.\n\
현재 project의 확정 정보(name/description)는 유지해야 한다.\n\
- name: {}\n- description: {}\n\n\
초기 컨텍스트는 이미 전달되었다. 아래 대화만 기반으로 답변하라.\n\n\
대화 이력:\n{}\n\n\
사용자 최신 입력:\n{}\n\n\
전체 project.md 출력 명시 요청 여부: {}\n\n\
응답 규칙:\n1) 기본 응답은 2~6줄의 짧은 대화형 답변으로 작성한다.\n\
2) 사용자가 명시적으로 전체 문서 갱신을 요청한 경우에만 업데이트된 `.project/project.md` 전체 markdown 본문을 출력한다.\n\
3) 전체 문서를 출력하지 않는 경우 코드펜스/장문 템플릿/원문 덤프를 금지한다.\n\
4) 코드펜스 설명문 금지.",
        modal.project_name,
        modal.project_description,
        modal.history.join("\n\n"),
        user_message,
        if full_md_requested { "yes" } else { "no" }
    )
}

fn calc_is_full_project_md_request(user_message: &str) -> bool {
    let lower = user_message.to_ascii_lowercase();
    let mentions_project_md = lower.contains("project.md");
    let asks_full_update = [
        "전체",
        "full",
        "갱신",
        "업데이트",
        "재작성",
        "rewrite",
        "replace",
    ]
    .iter()
    .any(|kw| user_message.contains(kw) || lower.contains(kw));
    mentions_project_md && asks_full_update
}

fn calc_is_project_md_dump(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    (lower.contains("# info") && lower.contains("## rule"))
        || (lower.contains("## features") && lower.contains("## structure"))
        || (lower.contains("project.md") && text.len() > 700)
}

fn action_spawn_ai_stream(model_bin: &str, prompt: String) -> (Receiver<AiStreamEvent>, Arc<AtomicBool>) {
    let (tx, rx) = mpsc::channel::<AiStreamEvent>();
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_flag = cancel.clone();
    let model = model_bin.to_string();
    thread::spawn(move || {
        let mut cmd = Command::new(&model);
        cmd.arg("exec");
        if model.eq_ignore_ascii_case("codex") {
            cmd.arg("--dangerously-bypass-approvals-and-sandbox");
        }
        let spawn_result = cmd
            .arg(prompt)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        let mut child = match spawn_result {
            Ok(v) => v,
            Err(e) => {
                let _ = tx.send(AiStreamEvent::Error(format!(
                    "failed to spawn {}: {}",
                    model, e
                )));
                return;
            }
        };
        let tx_out = tx.clone();
        if let Some(stdout) = child.stdout.take() {
            thread::spawn(move || {
                let mut reader = io::BufReader::new(stdout);
                let mut line = String::new();
                loop {
                    line.clear();
                    match reader.read_line(&mut line) {
                        Ok(0) => break,
                        Ok(_) => {
                            let _ = tx_out.send(AiStreamEvent::Chunk(line.clone()));
                        }
                        Err(e) => {
                            let _ = tx_out.send(AiStreamEvent::Error(format!(
                                "stdout read failed: {}",
                                e
                            )));
                            break;
                        }
                    }
                }
            });
        }
        loop {
            if cancel_flag.load(Ordering::Relaxed) {
                let _ = child.kill();
                let _ = child.wait();
                let _ = tx.send(AiStreamEvent::Cancelled);
                break;
            }
            match child.try_wait() {
                Ok(Some(status)) if status.success() => {
                    let _ = tx.send(AiStreamEvent::Done);
                    break;
                }
                Ok(Some(status)) => {
                    let _ = tx.send(AiStreamEvent::Error(format!(
                        "{} failed: code={:?}",
                        model,
                        status.code(),
                    )));
                    break;
                }
                Ok(None) => {
                    thread::sleep(Duration::from_millis(40));
                }
                Err(e) => {
                    let _ = tx.send(AiStreamEvent::Error(format!("wait failed: {}", e)));
                    break;
                }
            }
        }
    });
    (rx, cancel)
}

fn action_cancel_ai_stream(app: &mut UiApp) {
    if let Some(modal) = app.ai_chat_modal.as_mut() {
        if let Some(cancel) = modal.stream_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        modal.streaming = false;
        modal.warmup_inflight = false;
        modal.streaming_buffer.clear();
        modal.stream_rx = None;
        modal.stream_cancel = None;
    }
}

fn action_close_ai_chat_modal_and_open_bootstrap(
    app: &mut UiApp,
    projects: &[ProjectRecord],
    project_index: usize,
) {
    action_cancel_ai_stream(app);
    app.ai_chat_modal = None;
    app.status_line = "ai modal closed".to_string();
    action_open_bootstrap_confirm(app, projects, project_index);
}

fn calc_now_unix() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_string()
}

fn calc_generate_project_id(existing: &BTreeSet<String>) -> String {
    const ALNUM: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    for _ in 0..512 {
        let mut out = String::with_capacity(4);
        for _ in 0..4 {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1);
            let idx = (seed as usize) % ALNUM.len();
            out.push(ALNUM[idx] as char);
        }
        if !existing.contains(&out) {
            return out;
        }
    }
    "0000".to_string()
}

fn action_assign_missing_project_ids(projects: &mut [ProjectRecord]) -> bool {
    let mut changed = false;
    let mut existing: BTreeSet<String> = projects
        .iter()
        .filter_map(|p| if p.id.is_empty() { None } else { Some(p.id.clone()) })
        .collect();
    for project in projects {
        if project.id.is_empty() {
            let id = calc_generate_project_id(&existing);
            existing.insert(id.clone());
            project.id = id;
            changed = true;
        }
    }
    changed
}

fn action_promote_recent_project_to_front(projects: &mut Vec<ProjectRecord>, recent_id: Option<&str>) {
    let Some(recent) = recent_id else {
        return;
    };
    let Some(pos) = projects.iter().position(|p| p.id == recent) else {
        return;
    };
    if pos == 0 {
        return;
    }
    let item = projects.remove(pos);
    projects.insert(0, item);
}

fn action_pick_selected_project_index(projects: &[ProjectRecord]) -> usize {
    projects
        .iter()
        .position(|p| p.selected)
        .unwrap_or(0)
        .min(projects.len().saturating_sub(1))
}

fn action_set_selected(projects: &mut [ProjectRecord], selected_index: usize) {
    for (idx, p) in projects.iter_mut().enumerate() {
        p.selected = idx == selected_index;
    }
}

fn action_collect_feature_names(project: Option<&ProjectRecord>) -> Vec<String> {
    let Some(project) = project else {
        return Vec::new();
    };

    let base = Path::new(&project.path).join(".project");
    let drafts_list_path = base.join("drafts_list.yaml");
    if let Ok(raw) = fs::read_to_string(&drafts_list_path) {
        if let Ok(doc) = serde_yaml::from_str::<DraftsListDoc>(&raw) {
            let mut set: BTreeSet<String> = BTreeSet::new();
            for feature in doc.feature {
                set.insert(feature);
            }
            for planned in doc.planned {
                set.insert(planned);
            }
            if !set.is_empty() {
                return set.into_iter().collect();
            }
        }
    }

    let mut set: BTreeSet<String> = BTreeSet::new();
    let feature_root = base.join("feature");
    if let Ok(entries) = fs::read_dir(feature_root) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(name) = entry.file_name().to_str() {
                    set.insert(name.to_string());
                }
            }
        }
    }
    set.into_iter().collect()
}

fn calc_pane_border_style(active: bool, palette: BorderPalette) -> Style {
    let color = if active {
        palette.active
    } else {
        palette.normal
    };
    Style::default().fg(color)
}

fn calc_has_overlay_modal(app: &UiApp) -> bool {
    app.create_modal.is_some()
        || app.path_change_confirm.is_some()
        || app.delete_confirm.is_some()
        || app.detail_fill_confirm.is_some()
        || app.draft_bulk_add_modal.is_some()
        || app.list_edit_modal.is_some()
        || app.draft_create_confirm.is_some()
        || app.bootstrap_confirm.is_some()
        || app.ai_chat_modal.is_some()
        || app.busy_message.is_some()
}

fn calc_color_rgb(color: Color) -> Option<(u8, u8, u8)> {
    match color {
        Color::Black => Some((0, 0, 0)),
        Color::Red => Some((128, 0, 0)),
        Color::Green => Some((0, 128, 0)),
        Color::Yellow => Some((128, 128, 0)),
        Color::Blue => Some((0, 0, 128)),
        Color::Magenta => Some((128, 0, 128)),
        Color::Cyan => Some((0, 128, 128)),
        Color::Gray => Some((192, 192, 192)),
        Color::DarkGray => Some((128, 128, 128)),
        Color::LightRed => Some((255, 0, 0)),
        Color::LightGreen => Some((0, 255, 0)),
        Color::LightYellow => Some((255, 255, 0)),
        Color::LightBlue => Some((0, 0, 255)),
        Color::LightMagenta => Some((255, 0, 255)),
        Color::LightCyan => Some((0, 255, 255)),
        Color::White => Some((255, 255, 255)),
        Color::Rgb(r, g, b) => Some((r, g, b)),
        Color::Reset | Color::Indexed(_) => None,
    }
}

fn calc_lerp_u8(from: u8, to: u8, t: f32) -> u8 {
    let start = from as f32;
    let end = to as f32;
    (start + (end - start) * t).round().clamp(0.0, 255.0) as u8
}

fn calc_lerp_color(from: Color, to: Color, t: f32) -> Color {
    let ratio = t.clamp(0.0, 1.0);
    let Some((fr, fg, fb)) = calc_color_rgb(from) else {
        return if ratio < 1.0 { from } else { to };
    };
    let Some((tr, tg, tb)) = calc_color_rgb(to) else {
        return if ratio < 1.0 { from } else { to };
    };
    Color::Rgb(
        calc_lerp_u8(fr, tr, ratio),
        calc_lerp_u8(fg, tg, ratio),
        calc_lerp_u8(fb, tb, ratio),
    )
}

fn calc_active_pane_tween_progress(app: &UiApp, pane_index: usize) -> Option<f32> {
    if !app.menu_active || app.tab_index != 1 || app.pane_focus != pane_index {
        return None;
    }
    let Some(started_at) = app.pane_activate_started_at else {
        return Some(1.0);
    };
    if app.pane_activate_index != pane_index {
        return Some(1.0);
    }
    let elapsed_ms = started_at.elapsed().as_millis() as f32;
    Some((elapsed_ms / 180.0).clamp(0.0, 1.0))
}

fn calc_tweened_pane_border_style(app: &UiApp, pane_index: usize, palette: BorderPalette) -> Style {
    if calc_has_overlay_modal(app) {
        return Style::default().fg(palette.inactive);
    }
    let Some(progress) = calc_active_pane_tween_progress(app, pane_index) else {
        return Style::default().fg(palette.normal);
    };
    Style::default().fg(calc_lerp_color(palette.normal, palette.active, progress))
}

fn calc_inset_rect(area: Rect, margin: u16) -> Rect {
    if margin == 0 {
        return area;
    }
    let doubled = margin.saturating_mul(2);
    let width = area.width.saturating_sub(doubled).max(1);
    let height = area.height.saturating_sub(doubled).max(1);
    Rect {
        x: area.x.saturating_add(margin),
        y: area.y.saturating_add(margin),
        width,
        height,
    }
}

fn calc_active_pane_margin(_app: &UiApp, _pane_index: usize) -> u16 {
    0
}

fn action_start_pane_activate_tween(app: &mut UiApp) {
    app.pane_activate_started_at = Some(Instant::now());
    app.pane_activate_index = app.pane_focus;
}

fn action_reset_parallel_runtime(app: &mut UiApp) {
    app.parallel_running = false;
    app.parallel_statuses.clear();
}

fn action_move_project_grid_selection(projects: &mut [ProjectRecord], app: &mut UiApp, delta: isize) {
    if !app.menu_active || app.tab_index != 0 || projects.is_empty() {
        return;
    }
    const MAX_CARDS: usize = 9;
    let visible_len = projects.len().min(MAX_CARDS);
    if visible_len == 0 {
        return;
    }
    let max_idx = visible_len.saturating_sub(1) as isize;
    let current = (app.project_index.min(visible_len.saturating_sub(1))) as isize;
    let next = (current + delta).clamp(0, max_idx) as usize;
    if next == app.project_index {
        return;
    }
    app.project_index = next;
    action_set_selected(projects, app.project_index);
    app.changed = true;
    action_reset_parallel_runtime(app);
    app.status_line = format!("selected project: {}", projects[app.project_index].name);
}

fn action_move_detail_pane_focus(app: &mut UiApp, key: KeyCode) {
    if !app.menu_active || app.tab_index != 1 {
        return;
    }
    app.pane_focus = match (app.pane_focus, key) {
        (0, KeyCode::Right) => 4,
        (0, KeyCode::Down) => 1,
        (1, KeyCode::Up) => 0,
        (1, KeyCode::Right) => 2,
        (1, KeyCode::Down) => 3,
        (2, KeyCode::Up) => 0,
        (2, KeyCode::Left) => 1,
        (2, KeyCode::Right) => 4,
        (2, KeyCode::Down) => 3,
        (3, KeyCode::Up) => 1,
        (3, KeyCode::Left) => 1,
        (3, KeyCode::Right) => 2,
        (4, KeyCode::Left) => 2,
        (4, KeyCode::Up) => 0,
        (4, KeyCode::Down) => 3,
        _ => app.pane_focus,
    };
    action_start_pane_activate_tween(app);
}

fn action_start_parallel_runtime(app: &mut UiApp, features: &[String]) {
    if features.is_empty() {
        app.parallel_running = false;
        app.parallel_statuses.clear();
        app.status_line = "no draft feature found for selected project".to_string();
        return;
    }
    app.parallel_statuses = features
        .iter()
        .map(|name| (name.clone(), TaskRuntimeState::Inactive))
        .collect();
    app.parallel_running = true;
    app.last_tick = Instant::now();
    app.status_line = format!(
        "parallel runtime started ({} tasks)",
        app.parallel_statuses.len()
    );
}

fn action_advance_parallel_runtime(app: &mut UiApp, projects: &[ProjectRecord]) {
    if !app.parallel_running {
        return;
    }

    if let Some((_, state)) = app
        .parallel_statuses
        .iter_mut()
        .find(|(_, state)| *state == TaskRuntimeState::Active)
    {
        *state = TaskRuntimeState::Clear;
        return;
    }

    if let Some((_, state)) = app
        .parallel_statuses
        .iter_mut()
        .find(|(_, state)| *state == TaskRuntimeState::Inactive)
    {
        *state = TaskRuntimeState::Active;
        return;
    }

    app.parallel_running = false;
    if let Some(project) = projects.get(app.project_index) {
        let planned = action_collect_planned_drafts_from_project(project);
        if planned.is_empty() {
            app.status_line = "parallel runtime finished; no draft item".to_string();
        } else {
            app.status_line = format!(
                "parallel runtime finished; {} planned draft(s) remain",
                planned.len()
            );
        }
    } else {
        app.status_line = "parallel runtime finished".to_string();
    }
}

fn calc_default_project_name_from_parent() -> String {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    cwd.file_name()
        .and_then(|v| v.to_str())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "new-project".to_string())
}

fn action_open_create_modal(app: &mut UiApp) {
    let default_name = calc_default_project_name_from_parent();
    let default_description = "프로젝트 설명".to_string();
    let default_spec = "auto".to_string();
    let default_path = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| ".".to_string());
    app.create_modal = Some(CreateProjectModal {
        mode: ProjectModalMode::Create,
        source_index: None,
        original_path: String::new(),
        name: default_name.clone(),
        description: default_description.clone(),
        spec: default_spec.clone(),
        path: default_path.clone(),
        name_is_default: true,
        description_is_default: true,
        spec_is_default: true,
        path_is_default: true,
        field_index: 0,
        confirm_selected: true,
    });
    app.status_line = "create project modal opened".to_string();
}

fn action_open_edit_modal(app: &mut UiApp, projects: &[ProjectRecord]) {
    let Some(project) = projects.get(app.project_index) else {
        app.status_line = "no project selected".to_string();
        return;
    };
    app.create_modal = Some(CreateProjectModal {
        mode: ProjectModalMode::Edit,
        source_index: Some(app.project_index),
        original_path: project.path.clone(),
        name: project.name.clone(),
        description: project.description.clone(),
        spec: "auto".to_string(),
        path: project.path.clone(),
        name_is_default: false,
        description_is_default: false,
        spec_is_default: true,
        path_is_default: false,
        field_index: 0,
        confirm_selected: true,
    });
    app.status_line = format!("project edit modal opened: {}", project.name);
}

fn action_resolve_project_path(raw_path: &str) -> Result<PathBuf, String> {
    let mut path = if raw_path.trim().is_empty() {
        PathBuf::from(".")
    } else {
        PathBuf::from(raw_path.trim())
    };
    if path.is_relative() {
        path = std::env::current_dir()
            .map_err(|e| format!("failed to read current dir: {}", e))?
            .join(path);
    }
    if path.exists() && !path.is_dir() {
        return Err(format!(
            "project path exists but is not a directory: {}",
            path.display()
        ));
    }
    Ok(path)
}

fn action_apply_project_create(
    projects: &mut Vec<ProjectRecord>,
    app: &mut UiApp,
    modal: &CreateProjectModal,
) -> Result<(), String> {
    let name = modal.name.trim();
    if name.is_empty() {
        return Err("name is required".to_string());
    }
    let path = action_resolve_project_path(modal.path.trim())?;
    fs::create_dir_all(&path)
        .map_err(|e| format!("failed to create project dir {}: {}", path.display(), e))?;
    fs::create_dir_all(path.join(".project"))
        .map_err(|e| format!("failed to create project meta dir: {}", e))?;

    let now = calc_now_unix();
    let mut created_new = false;
    let selected_index = if let Some((idx, p)) = projects
        .iter_mut()
        .enumerate()
        .find(|(_, p)| p.name == name)
    {
        p.path = path.display().to_string();
        p.description = modal.description.trim().to_string();
        p.updated_at = now;
        app.status_line = format!("project updated: {}", name);
        idx
    } else {
        let existing_ids: BTreeSet<String> = projects
            .iter()
            .filter_map(|p| if p.id.is_empty() { None } else { Some(p.id.clone()) })
            .collect();
        action_run_plan_init_in_project_dir(&path, name, modal.description.trim(), modal.spec.trim())?;
        created_new = true;
        projects.push(ProjectRecord {
            id: calc_generate_project_id(&existing_ids),
            name: name.to_string(),
            path: path.display().to_string(),
            description: modal.description.trim().to_string(),
            created_at: now.clone(),
            updated_at: now,
            selected: false,
        });
        app.status_line = format!("project created: {}", name);
        projects.len().saturating_sub(1)
    };
    action_set_selected(projects, selected_index);
    app.project_index = selected_index;
    app.changed = true;
    action_reset_parallel_runtime(app);
    if created_new {
        action_open_detail_fill_confirm(app, selected_index);
    }
    Ok(())
}

fn action_try_submit_edit_project(
    projects: &mut [ProjectRecord],
    app: &mut UiApp,
    modal: &CreateProjectModal,
) -> Result<(), String> {
    let source_index = modal.source_index.unwrap_or(app.project_index);
    if source_index >= projects.len() {
        return Err("selected project index is out of range".to_string());
    }
    let name = modal.name.trim();
    if name.is_empty() {
        return Err("name is required".to_string());
    }
    if projects
        .iter()
        .enumerate()
        .any(|(idx, p)| idx != source_index && p.name == name)
    {
        return Err(format!("project name already exists: {}", name));
    }
    let path = action_resolve_project_path(modal.path.trim())?;
    let old_path = PathBuf::from(&modal.original_path);
    if path != old_path {
        app.path_change_confirm = Some(PathChangeConfirm {
            source_index,
            new_name: name.to_string(),
            new_description: modal.description.trim().to_string(),
            old_path: modal.original_path.clone(),
            new_path: path.display().to_string(),
            confirm_selected: true,
        });
        app.status_line = "path changed: y/n to move directory".to_string();
        return Ok(());
    }

    let now = calc_now_unix();
    {
        let target = &mut projects[source_index];
        target.name = name.to_string();
        target.description = modal.description.trim().to_string();
        target.updated_at = now;
    }
    action_set_selected(projects, source_index);
    app.project_index = source_index;
    app.changed = true;
    action_reset_parallel_runtime(app);
    app.status_line = format!("project updated: {}", projects[source_index].name);
    Ok(())
}

fn action_apply_path_change_confirm(
    projects: &mut [ProjectRecord],
    app: &mut UiApp,
    confirm: PathChangeConfirm,
    move_dir: bool,
) -> Result<(), String> {
    if confirm.source_index >= projects.len() {
        return Err("selected project index is out of range".to_string());
    }
    if projects.iter().enumerate().any(|(idx, p)| {
        idx != confirm.source_index && p.name == confirm.new_name
    }) {
        return Err(format!("project name already exists: {}", confirm.new_name));
    }

    let old_path = PathBuf::from(&confirm.old_path);
    let new_path = PathBuf::from(&confirm.new_path);
    if move_dir && old_path != new_path {
        if old_path.exists() {
            if new_path.exists() {
                return Err(format!(
                    "target path already exists: {}",
                    new_path.display()
                ));
            }
            if let Some(parent) = new_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
            }
            fs::rename(&old_path, &new_path).map_err(|e| {
                format!(
                    "failed to move project dir {} -> {}: {}",
                    old_path.display(),
                    new_path.display(),
                    e
                )
            })?;
        } else {
            fs::create_dir_all(&new_path).map_err(|e| {
                format!("failed to create project dir {}: {}", new_path.display(), e)
            })?;
            fs::create_dir_all(new_path.join(".project"))
                .map_err(|e| format!("failed to create project meta dir: {}", e))?;
        }
    }

    let now = calc_now_unix();
    {
        let target = &mut projects[confirm.source_index];
        target.name = confirm.new_name;
        target.description = confirm.new_description;
        if move_dir {
            target.path = confirm.new_path;
        }
        target.updated_at = now;
    }
    action_set_selected(projects, confirm.source_index);
    app.project_index = confirm.source_index;
    app.changed = true;
    action_reset_parallel_runtime(app);
    let updated_name = projects[confirm.source_index].name.clone();
    app.status_line = if move_dir {
        format!("project updated and moved: {}", updated_name)
    } else {
        format!("project updated without path move: {}", updated_name)
    };
    Ok(())
}

fn action_open_delete_confirm(app: &mut UiApp, projects: &[ProjectRecord]) {
    let Some(project) = projects.get(app.project_index) else {
        app.status_line = "no project selected".to_string();
        return;
    };
    app.delete_confirm = Some(DeleteProjectConfirm {
        source_index: app.project_index,
        project_name: project.name.clone(),
        project_path: project.path.clone(),
        confirm_selected: true,
    });
    app.status_line = format!("delete confirm: {}", project.name);
}

fn action_apply_delete_confirm(
    projects: &mut Vec<ProjectRecord>,
    app: &mut UiApp,
    confirm: DeleteProjectConfirm,
    accepted: bool,
) -> Result<(), String> {
    if !accepted {
        app.status_line = "delete canceled".to_string();
        return Ok(());
    }
    if confirm.source_index >= projects.len() {
        return Err("selected project index is out of range".to_string());
    }
    if projects[confirm.source_index].name != confirm.project_name {
        return Err("project selection changed; delete canceled".to_string());
    }

    let project_meta = Path::new(&confirm.project_path).join(".project");
    if project_meta.exists() {
        let entries = fs::read_dir(&project_meta)
            .map_err(|e| format!("failed to read {}: {}", project_meta.display(), e))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("failed to read dir entry: {}", e))?;
            let entry_path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|e| format!("failed to read file type: {}", e))?;
            if file_type.is_dir() {
                fs::remove_dir_all(&entry_path)
                    .map_err(|e| format!("failed to remove {}: {}", entry_path.display(), e))?;
            } else {
                fs::remove_file(&entry_path)
                    .map_err(|e| format!("failed to remove {}: {}", entry_path.display(), e))?;
            }
        }
    }

    projects.remove(confirm.source_index);
    if projects.is_empty() {
        app.project_index = 0;
    } else {
        app.project_index = confirm.source_index.min(projects.len() - 1);
        action_set_selected(projects, app.project_index);
    }
    app.changed = true;
    action_reset_parallel_runtime(app);
    app.status_line = format!("project deleted: {}", confirm.project_name);
    Ok(())
}

fn action_render_projects_tab(
    f: &mut ratatui::Frame,
    area: Rect,
    projects: &[ProjectRecord],
    selected_index: usize,
    active: bool,
    overlay_modal: bool,
    parallel_running: bool,
    palette: BorderPalette,
) {
    let panel = Block::default()
        .title("Project Select")
        .borders(Borders::ALL)
        .border_style(if overlay_modal {
            Style::default().fg(palette.inactive).add_modifier(Modifier::DIM)
        } else if active {
            calc_pane_border_style(true, palette).add_modifier(Modifier::BOLD)
        } else {
            calc_pane_border_style(false, palette).add_modifier(Modifier::DIM)
        });
    let inner = panel.inner(area);
    f.render_widget(panel, area);

    if projects.is_empty() {
        f.render_widget(Paragraph::new("no projects"), inner);
        return;
    }

    const MAX_CARDS: usize = 9;
    const COLUMNS: usize = 3;
    const ROWS: usize = 3;
    let visible = &projects[..projects.len().min(MAX_CARDS)];
    let row_constraints = vec![Constraint::Ratio(1, ROWS as u32); ROWS];
    let row_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(inner);

    let name_style = Style::default()
        .fg(Color::White)
        .bg(Color::Black)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default()
        .fg(Color::Rgb(130, 130, 130))
        .add_modifier(Modifier::DIM);
    let path_style = Style::default()
        .fg(Color::Rgb(180, 180, 180))
        .add_modifier(Modifier::DIM);

    for (row_idx, row_area) in row_layout.iter().enumerate() {
        let col_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(34),
                Constraint::Percentage(33),
                Constraint::Percentage(33),
            ])
            .split(*row_area);

        for (col_idx, card_area) in col_layout.iter().enumerate() {
            let idx = row_idx * COLUMNS + col_idx;
            if idx >= visible.len() {
                continue;
            }
            let p = &visible[idx];
            let is_selected = idx == selected_index;
            let card_border = if overlay_modal {
                Style::default().fg(palette.inactive).add_modifier(Modifier::DIM)
            } else if is_selected {
                calc_pane_border_style(true, palette)
            } else {
                calc_pane_border_style(false, palette)
            };
            let card = Paragraph::new(vec![
                Line::from(Span::styled(p.name.clone(), name_style)),
                Line::from(Span::styled(p.description.clone(), desc_style)),
                Line::from(Span::styled(p.path.clone(), path_style)),
            ])
            .block(Block::default().borders(Borders::ALL).border_style(card_border))
            .wrap(Wrap { trim: false });
            f.render_widget(card, *card_area);
            if parallel_running && is_selected {
                let badge = Rect {
                    x: card_area.x.saturating_add(1),
                    y: card_area.y,
                    width: card_area.width.saturating_sub(2),
                    height: 1,
                };
                f.render_widget(
                    Paragraph::new(Span::styled(
                        "작업중",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .alignment(Alignment::Right),
                    badge,
                );
            }
        }
    }
}

fn action_render_details_tab(
    f: &mut ratatui::Frame,
    area: Rect,
    app: &UiApp,
    projects: &[ProjectRecord],
    _features: &[String],
    _menu_active: bool,
    palette: BorderPalette,
) {
    let col_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70),
            Constraint::Percentage(30),
        ])
        .split(area);
    let left_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(42),
            Constraint::Percentage(28),
        ])
        .split(col_layout[0]);
    let middle_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(left_rows[1]);

    let selected_project = projects.get(app.project_index);
    let project_md = selected_project.and_then(action_read_project_md);
    let parsed = project_md.as_deref().map(action_parse_project_md);
    let (name_value, desc_value, spec_value, goal_value): (String, String, String, String) =
        if let Some(doc) = &parsed {
            (
                doc.name.clone(),
                doc.description.clone(),
                doc.spec.clone(),
                doc.goal.clone(),
            )
        } else if let Some(project) = selected_project {
            (
                project.name.clone(),
                project.description.clone(),
                project.path.clone(),
                "project.md not found".to_string(),
            )
        } else {
            (
                "no selected project".to_string(),
                "no selected project".to_string(),
                "no selected project".to_string(),
                "no selected project".to_string(),
            )
        };
    let project_title = selected_project
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "Project".to_string());

    let project_area = calc_inset_rect(left_rows[0], calc_active_pane_margin(app, 0));
    let project_block = Block::default()
        .title(project_title)
        .borders(Borders::ALL)
        .border_style(calc_tweened_pane_border_style(app, 0, palette));
    let project_inner = project_block.inner(project_area);
    f.render_widget(project_block, project_area);
    let max_w = project_inner.width.saturating_sub(2).max(8);
    let separator = "─".repeat(max_w as usize);
    let project_lines = vec![
        Line::from(calc_truncate_to_width_ellipsis(
            &format!("Name: {}", name_value),
            max_w,
        )),
        Line::from(Span::styled(
            separator.clone(),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(calc_truncate_to_width_ellipsis(
            &format!("Description: {}", desc_value),
            max_w,
        )),
        Line::from(Span::styled(
            separator.clone(),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(calc_truncate_to_width_ellipsis(
            &format!("Spec: {}", spec_value),
            max_w,
        )),
        Line::from(Span::styled(
            separator.clone(),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(calc_truncate_to_width_ellipsis(
            &format!("Goal: {}", goal_value),
            max_w,
        )),
    ];
    f.render_widget(
        Paragraph::new(project_lines)
            .style(Style::default().fg(Color::Black))
            .wrap(Wrap { trim: false }),
        project_inner,
    );

    let rule_lines: Vec<Line> = if let Some(doc) = &parsed {
        if doc.rules.is_empty() {
            vec![Line::from("no rule")]
        } else {
            let max_w = middle_cols[0].width.saturating_sub(6).max(8);
            doc.rules
                .iter()
                .enumerate()
                .map(|(idx, item)| {
                    let raw = format!("{}. {}", idx + 1, item);
                    Line::from(calc_truncate_to_width_ellipsis(&raw, max_w))
                })
                .collect()
        }
    } else {
        vec![Line::from("no selected project")]
    };
    let rule_area = calc_inset_rect(middle_cols[0], calc_active_pane_margin(app, 1));
    let rule_block = Block::default()
        .title("Rule")
        .borders(Borders::ALL)
        .border_style(calc_tweened_pane_border_style(app, 1, palette));
    f.render_widget(
        Paragraph::new(rule_lines)
            .block(rule_block)
            .wrap(Wrap { trim: false }),
        rule_area,
    );

    let constraint_lines: Vec<Line> = if let Some(doc) = &parsed {
        if doc.constraints.is_empty() {
            vec![Line::from("no constraint")]
        } else {
            let max_w = middle_cols[1].width.saturating_sub(6).max(8);
            doc.constraints
                .iter()
                .enumerate()
                .map(|(idx, item)| {
                    let raw = format!("{}. {}", idx + 1, item);
                    Line::from(calc_truncate_to_width_ellipsis(&raw, max_w))
                })
                .collect()
        }
    } else {
        vec![Line::from("no selected project")]
    };
    let constraint_area = calc_inset_rect(middle_cols[1], calc_active_pane_margin(app, 2));
    let constraint_block = Block::default()
        .title("Constraint")
        .borders(Borders::ALL)
        .border_style(calc_tweened_pane_border_style(app, 2, palette));
    f.render_widget(
        Paragraph::new(constraint_lines)
            .block(constraint_block)
            .wrap(Wrap { trim: false }),
        constraint_area,
    );

    let feature_lines: Vec<Line> = selected_project
        .map(action_collect_feature_items_from_drafts)
        .map(|features| {
            if features.is_empty() {
                vec![Line::from("no feature")]
            } else {
                let max_w = left_rows[2].width.saturating_sub(6).max(8);
                features
                    .iter()
                    .enumerate()
                    .map(|(idx, item)| {
                        let raw = format!("{}. {}", idx + 1, item);
                        Line::from(calc_truncate_to_width_ellipsis(&raw, max_w))
                    })
                    .collect()
            }
        })
        .unwrap_or_else(|| vec![Line::from("no selected project")]);
    let feature_area = calc_inset_rect(left_rows[2], calc_active_pane_margin(app, 3));
    let feature_block = Block::default()
        .title("Features")
        .borders(Borders::ALL)
        .border_style(calc_tweened_pane_border_style(app, 3, palette));
    f.render_widget(
        Paragraph::new(feature_lines)
            .block(feature_block)
            .wrap(Wrap { trim: false }),
        feature_area,
    );

    let planned = selected_project
        .map(action_collect_planned_drafts_from_project)
        .unwrap_or_default();
    let draft_area = calc_inset_rect(col_layout[1], calc_active_pane_margin(app, 4));
    let draft_title = if app.parallel_running {
        "Drafts | 작업중"
    } else {
        "Drafts"
    };
    let draft_selected = app.menu_active
        && app.tab_index == 1
        && app.pane_focus == 4
        && !calc_has_overlay_modal(app);
    let draft_border_style = if app.parallel_running || draft_selected {
        calc_tweened_pane_border_style(app, 4, palette)
    } else if planned.is_empty() {
        Style::default().fg(palette.inactive)
    } else {
        Style::default().fg(palette.normal)
    };
    let draft_block = Block::default()
        .title(draft_title)
        .borders(Borders::ALL)
        .border_style(draft_border_style);
    if app.parallel_running && !app.parallel_statuses.is_empty() {
        let max_w = draft_area.width.saturating_sub(6).max(8);
        let lines: Vec<Line> = app
            .parallel_statuses
            .iter()
            .map(|(task, state)| {
                let status = match state {
                    TaskRuntimeState::Inactive => "대기",
                    TaskRuntimeState::Active => "작업중",
                    TaskRuntimeState::Clear => "완료",
                };
                let raw = format!("{} : {}", task, status);
                let color = match state {
                    TaskRuntimeState::Inactive => palette.inactive,
                    TaskRuntimeState::Active => palette.active,
                    TaskRuntimeState::Clear => palette.normal,
                };
                Line::from(Span::styled(
                    calc_truncate_to_width_ellipsis(&raw, max_w),
                    Style::default().fg(color),
                ))
            })
            .collect();
        f.render_widget(
            Paragraph::new(lines)
                .block(draft_block)
                .wrap(Wrap { trim: false }),
            draft_area,
        );
    } else if planned.is_empty() {
        let inner = draft_block.inner(draft_area);
        f.render_widget(draft_block, draft_area);
        let line = vec![Line::from(Span::styled(
            "no draft item",
            Style::default().fg(palette.inactive),
        ))];
        let body_area = Rect {
            x: inner.x,
            y: inner.y.saturating_add(inner.height.saturating_sub(1) / 2),
            width: inner.width,
            height: 1,
        };
        f.render_widget(
            Paragraph::new(line).alignment(Alignment::Center),
            body_area,
        );
    } else {
        let draft_lines: Vec<Line> = planned
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                Line::from(Span::styled(
                    format!("{}. {}", idx + 1, item),
                    Style::default().fg(palette.normal),
                ))
            })
            .collect();
        f.render_widget(
            Paragraph::new(draft_lines)
                .block(draft_block)
                .wrap(Wrap { trim: false }),
            draft_area,
        );
    }
}

fn action_read_project_md(project: &ProjectRecord) -> Option<String> {
    let path = Path::new(&project.path).join(".project").join("project.md");
    fs::read_to_string(path).ok()
}

#[derive(Debug, Clone, Default)]
struct ProjectMdDoc {
    name: String,
    description: String,
    spec: String,
    goal: String,
    rules: Vec<String>,
    constraints: Vec<String>,
}

fn action_parse_project_md(project_md: &str) -> ProjectMdDoc {
    let mut doc = ProjectMdDoc::default();
    let mut in_rule = false;
    let mut in_constraints = false;
    for line in project_md.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("## rule") {
            in_rule = true;
            in_constraints = false;
            continue;
        }
        if trimmed.eq_ignore_ascii_case("# constraints") || trimmed.eq_ignore_ascii_case("## constraints") {
            in_rule = false;
            in_constraints = true;
            continue;
        }
        if trimmed.starts_with('#') && !trimmed.eq_ignore_ascii_case("## rule") {
            in_rule = false;
            if !trimmed.eq_ignore_ascii_case("# constraints")
                && !trimmed.eq_ignore_ascii_case("## constraints")
            {
                in_constraints = false;
            }
        }
        if in_rule && trimmed.starts_with("- ") {
            doc.rules
                .push(trimmed.trim_start_matches("- ").trim().to_string());
        }
        if in_constraints && trimmed.starts_with("- ") {
            doc.constraints
                .push(trimmed.trim_start_matches("- ").trim().to_string());
        }
        if let Some(v) = trimmed.strip_prefix("- name:") {
            doc.name = v.trim().to_string();
        } else if let Some(v) = trimmed.strip_prefix("- description:") {
            doc.description = v.trim().to_string();
        } else if let Some(v) = trimmed.strip_prefix("- spec:") {
            doc.spec = v.trim().to_string();
        } else if let Some(v) = trimmed.strip_prefix("- goal:") {
            doc.goal = v.trim().to_string();
        }
    }
    doc
}

fn action_collect_planned_drafts_from_project(project: &ProjectRecord) -> Vec<String> {
    let path = Path::new(&project.path).join(".project").join("drafts_list.yaml");
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(doc) = serde_yaml::from_str::<DraftsListDoc>(&raw) else {
        return Vec::new();
    };
    doc.planned
}

fn action_open_list_edit_modal(
    app: &mut UiApp,
    projects: &[ProjectRecord],
    target: ListEditTarget,
) {
    let Some(project) = projects.get(app.project_index) else {
        app.status_line = "no selected project".to_string();
        return;
    };
    let md = action_read_project_md(project).unwrap_or_default();
    let parsed = action_parse_project_md(&md);
    let items = match target {
        ListEditTarget::Rule => parsed.rules,
        ListEditTarget::Constraint => parsed.constraints,
        ListEditTarget::Feature => action_collect_feature_items_from_drafts(project),
    };
    app.list_edit_modal = Some(ListEditModal {
        project_index: app.project_index,
        target,
        items,
        selected_index: 0,
        input_mode: None,
        input: String::new(),
        confirm_selected: true,
    });
    app.status_line = "list edit modal opened".to_string();
}

fn action_save_project_md_list(
    projects: &[ProjectRecord],
    project_index: usize,
    target: ListEditTarget,
    items: &[String],
) -> Result<(), String> {
    if matches!(target, ListEditTarget::Feature) {
        return action_save_drafts_feature_list(projects, project_index, items);
    }
    let Some(project) = projects.get(project_index) else {
        return Err("selected project index out of range".to_string());
    };
    let path = Path::new(&project.path).join(".project").join("project.md");
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let mut lines: Vec<String> = raw.lines().map(|v| v.to_string()).collect();
    let header = match target {
        ListEditTarget::Rule => "## rule",
        ListEditTarget::Constraint => "# Constraints",
        ListEditTarget::Feature => "## features",
    };
    let header_idx = lines
        .iter()
        .position(|line| line.trim().eq_ignore_ascii_case(header));
    let idx = if let Some(i) = header_idx {
        i
    } else {
        lines.push(String::new());
        lines.push(header.to_string());
        lines.push(String::new());
        lines.len() - 2
    };
    let mut end = idx + 1;
    while end < lines.len() {
        let t = lines[end].trim();
        if t.starts_with('#') {
            break;
        }
        end += 1;
    }
    let mut replacement: Vec<String> = items.iter().map(|v| format!("- {}", v)).collect();
    if replacement.is_empty() {
        replacement.push("- ".to_string());
    }
    lines.splice((idx + 1)..end, replacement);
    fs::write(&path, lines.join("\n") + "\n")
        .map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn action_collect_feature_items_from_drafts(project: &ProjectRecord) -> Vec<String> {
    let path = Path::new(&project.path).join(".project").join("drafts_list.yaml");
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(doc) = serde_yaml::from_str::<DraftsListDoc>(&raw) else {
        return Vec::new();
    };
    doc.feature
}

fn calc_normalize_feature_item(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("empty feature item".to_string());
    }
    let Some((name, description)) = trimmed.split_once(':') else {
        return Err("feature format: 기능명 : 설명".to_string());
    };
    let name = name.trim();
    let description = description.trim();
    if name.is_empty() || description.is_empty() {
        return Err("feature format: 기능명 : 설명".to_string());
    }
    Ok(format!("{} : {}", name, description))
}

fn action_save_drafts_feature_list(
    projects: &[ProjectRecord],
    project_index: usize,
    items: &[String],
) -> Result<(), String> {
    let Some(project) = projects.get(project_index) else {
        return Err("selected project index out of range".to_string());
    };
    let path = Path::new(&project.path).join(".project").join("drafts_list.yaml");
    let raw = fs::read_to_string(&path).unwrap_or_default();
    let mut doc = serde_yaml::from_str::<DraftsListDoc>(&raw).unwrap_or_default();
    let mut normalized = Vec::new();
    for item in items {
        normalized.push(calc_normalize_feature_item(item)?);
    }
    doc.feature = normalized;
    let encoded =
        serde_yaml::to_string(&doc).map_err(|e| format!("failed to encode drafts_list yaml: {}", e))?;
    fs::write(&path, encoded).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn calc_truncate_to_width_ellipsis(value: &str, width: u16) -> String {
    if width <= 3 {
        return ".".repeat(width as usize);
    }
    if UnicodeWidthStr::width(value) as u16 <= width {
        return value.to_string();
    }
    let mut out = String::new();
    let keep_w = width.saturating_sub(3);
    let mut used = 0u16;
    for ch in value.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
        if w == 0 {
            continue;
        }
        if used.saturating_add(w) > keep_w {
            break;
        }
        out.push(ch);
        used = used.saturating_add(w);
    }
    out.push_str("...");
    out
}

fn action_render_list_edit_modal(
    f: &mut ratatui::Frame,
    area: Rect,
    modal: &ListEditModal,
) -> Option<(u16, u16)> {
    f.render_widget(Clear, area);
    let title = match modal.target {
        ListEditTarget::Rule => "Edit Rule",
        ListEditTarget::Constraint => "Edit Constraint",
        ListEditTarget::Feature => "Edit Features",
    };
    let block = Block::default().title(title).borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(2)])
        .split(inner);
    let list_w = chunks[0].width.saturating_sub(6).max(8);
    let row_capacity = chunks[0]
        .height
        .saturating_sub(2)
        .saturating_div(2)
        .max(1) as usize;
    let list_start = if modal.selected_index >= row_capacity {
        modal
            .selected_index
            .saturating_add(1)
            .saturating_sub(row_capacity)
    } else {
        0
    };
    let list_end = list_start.saturating_add(row_capacity).min(modal.items.len());
    let lines: Vec<Line> = if modal.items.is_empty() {
        vec![Line::from("(empty)")]
    } else {
        let mut out = Vec::new();
        for idx in list_start..list_end {
            let prefix = if idx == modal.selected_index { "> " } else { "  " };
            let value = calc_truncate_to_width_ellipsis(&modal.items[idx], list_w.saturating_sub(2));
            let base = format!("{}{}", prefix, value);
            if idx == modal.selected_index {
                out.push(Line::from(Span::styled(
                    base,
                    Style::default().add_modifier(Modifier::BOLD),
                )));
            } else {
                out.push(Line::from(base));
            }
            out.push(Line::from("-".repeat(list_w as usize)));
        }
        if list_end < modal.items.len() {
            out.push(Line::from("..."));
        }
        out
    };
    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Items"))
            .wrap(Wrap { trim: false }),
        chunks[0],
    );

    action_render_confirm_buttons_bottom_right(
        f,
        inner,
        "Confirm",
        "Cancel",
        modal.confirm_selected,
    );

    if modal.input_mode.is_some() {
        let editor_area = calc_centered_rect(72, 24, area);
        f.render_widget(Clear, editor_area);
        let editor_title = match modal.input_mode {
            Some(ListEditInputMode::Add) => "New Item",
            Some(ListEditInputMode::Edit) => "Edit Item",
            None => "Edit Item",
        };
        let editor_block = Block::default().title(editor_title).borders(Borders::ALL);
        let editor_inner = editor_block.inner(editor_area);
        f.render_widget(editor_block, editor_area);
        let input_area = Rect {
            x: editor_inner.x,
            y: editor_inner.y,
            width: editor_inner.width,
            height: editor_inner.height.min(3),
        };
        let hint = if matches!(modal.target, ListEditTarget::Feature) {
            "feature format: 기능명 : 설명"
        } else {
            "enter apply | esc cancel"
        };
        f.render_widget(
            Paragraph::new(modal.input.clone())
                .block(Block::default().borders(Borders::ALL).title(hint))
                .wrap(Wrap { trim: false }),
            input_area,
        );
        Some(calc_cursor_in_input(input_area, &modal.input))
    } else {
        None
    }
}

fn calc_centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1]);
    horizontal[1]
}

fn calc_input_value_style(is_default: bool) -> Style {
    if is_default {
        Style::default().fg(Color::Black)
    } else {
        Style::default()
    }
}

fn calc_modal_field_value_style(modal: &CreateProjectModal, field_index: usize, is_default: bool) -> Style {
    if modal.field_index != field_index {
        return Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM);
    }
    calc_input_value_style(is_default)
}

fn calc_modal_input_border_style(active: bool) -> Style {
    if active {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM)
    }
}

fn calc_modal_label_style(active: bool) -> Style {
    if active {
        Style::default()
            .bg(Color::Black)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

fn action_render_create_modal(
    f: &mut ratatui::Frame,
    area: Rect,
    modal: &CreateProjectModal,
) -> Option<(u16, u16)> {
    f.render_widget(Clear, area);
    let title = if modal.mode == ProjectModalMode::Create {
        "Create Project"
    } else {
        "Edit Project"
    };
    let container = Block::default().title(title).borders(Borders::ALL);
    let inner = container.inner(area);
    f.render_widget(container, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(5),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(inner);

    let name_label = Paragraph::new(Line::from(Span::styled(
        "Name",
        calc_modal_label_style(modal.field_index == 0),
    )));
    f.render_widget(name_label, layout[0]);
    let name_block = Block::default()
        .borders(Borders::ALL)
        .border_style(calc_modal_input_border_style(modal.field_index == 0));
    f.render_widget(
        Paragraph::new(modal.name.clone())
            .style(calc_modal_field_value_style(modal, 0, modal.name_is_default))
            .block(name_block),
        layout[1],
    );

    let desc_label = Paragraph::new(Line::from(Span::styled(
        "Description",
        calc_modal_label_style(modal.field_index == 1),
    )));
    f.render_widget(desc_label, layout[2]);
    let desc_block = Block::default()
        .borders(Borders::ALL)
        .border_style(calc_modal_input_border_style(modal.field_index == 1));
    f.render_widget(
        Paragraph::new(modal.description.clone())
            .style(calc_modal_field_value_style(modal, 1, modal.description_is_default))
            .wrap(Wrap { trim: false })
            .block(desc_block),
        layout[3],
    );

    let spec_label = Paragraph::new(Line::from(Span::styled(
        "Spec",
        calc_modal_label_style(modal.field_index == 2),
    )));
    f.render_widget(spec_label, layout[4]);
    let spec_block = Block::default()
        .borders(Borders::ALL)
        .border_style(calc_modal_input_border_style(modal.field_index == 2));
    f.render_widget(
        Paragraph::new(modal.spec.clone())
            .style(calc_modal_field_value_style(modal, 2, modal.spec_is_default))
            .block(spec_block),
        layout[5],
    );

    let path_label = Paragraph::new(Line::from(Span::styled(
        "Project Path",
        calc_modal_label_style(modal.field_index == 3),
    )));
    f.render_widget(path_label, layout[6]);
    let path_block = Block::default()
        .borders(Borders::ALL)
        .border_style(calc_modal_input_border_style(modal.field_index == 3));
    f.render_widget(
        Paragraph::new(modal.path.clone())
            .style(calc_modal_field_value_style(modal, 3, modal.path_is_default))
            .block(path_block),
        layout[7],
    );

    action_render_confirm_buttons_bottom_right(
        f,
        inner,
        "Confirm",
        "Cancel",
        modal.confirm_selected,
    );

    action_calc_modal_cursor(modal, layout[1], layout[3], layout[5], layout[7])
}

fn action_calc_modal_cursor(
    modal: &CreateProjectModal,
    name_area: Rect,
    desc_area: Rect,
    spec_area: Rect,
    path_area: Rect,
) -> Option<(u16, u16)> {
    match modal.field_index {
        0 => Some(calc_cursor_in_input(name_area, &modal.name)),
        1 => Some(calc_cursor_in_input(desc_area, &modal.description)),
        2 => Some(calc_cursor_in_input(spec_area, &modal.spec)),
        3 => Some(calc_cursor_in_input(path_area, &modal.path)),
        _ => None,
    }
}

fn calc_cursor_in_input(area: Rect, value: &str) -> (u16, u16) {
    let inner_w = area.width.saturating_sub(2).max(1);
    let inner_h = area.height.saturating_sub(2).max(1);

    let mut row: u16 = 0;
    let mut col: u16 = 0;
    for ch in value.chars() {
        if ch == '\n' {
            row = row.saturating_add(1);
            col = 0;
            continue;
        }
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
        if ch_width == 0 {
            continue;
        }
        if col.saturating_add(ch_width) > inner_w {
            row = row.saturating_add(1);
            col = 0;
        }
        col = col.saturating_add(ch_width);
        if col == inner_w {
            row = row.saturating_add(1);
            col = 0;
        }
    }

    let clamped_row = row.min(inner_h.saturating_sub(1));
    let clamped_col = col.min(inner_w.saturating_sub(1));
    (
        area.x.saturating_add(1).saturating_add(clamped_col),
        area.y.saturating_add(1).saturating_add(clamped_row),
    )
}

fn action_handle_modal_input(
    _projects: &mut Vec<ProjectRecord>,
    app: &mut UiApp,
    key: KeyCode,
) -> Result<bool, String> {
    let Some(mut modal) = app.create_modal.take() else {
        return Ok(false);
    };
    let mut close_modal = false;

    match key {
        KeyCode::Char('q') => {
            app.status_line = "focus closed (inactive)".to_string();
            app.menu_active = false;
            close_modal = true;
        }
        KeyCode::Esc => {
            app.status_line = "create project canceled".to_string();
            close_modal = true;
        }
        KeyCode::Tab | KeyCode::Down => {
            modal.field_index = (modal.field_index + 1) % 5;
        }
        KeyCode::Up => {
            modal.field_index = if modal.field_index == 0 {
                4
            } else {
                modal.field_index - 1
            };
        }
        KeyCode::Left | KeyCode::Right if modal.field_index == 4 => {
            modal.confirm_selected = !modal.confirm_selected;
        }
        KeyCode::Backspace if modal.field_index == 0 => {
            if modal.name_is_default {
                modal.name_is_default = false;
            }
            modal.name.pop();
        }
        KeyCode::Backspace if modal.field_index == 1 => {
            if modal.description_is_default {
                modal.description.clear();
                modal.description_is_default = false;
            }
            modal.description.pop();
        }
        KeyCode::Backspace if modal.field_index == 2 => {
            if modal.spec_is_default {
                modal.spec.clear();
                modal.spec_is_default = false;
            }
            modal.spec.pop();
        }
        KeyCode::Backspace if modal.field_index == 3 => {
            if modal.path_is_default {
                modal.path_is_default = false;
            }
            modal.path.pop();
        }
        KeyCode::Char(c) if modal.field_index == 0 => {
            if modal.name_is_default {
                modal.name_is_default = false;
            }
            modal.name.push(c);
        }
        KeyCode::Char(c) if modal.field_index == 1 => {
            if modal.description_is_default {
                modal.description.clear();
                modal.description_is_default = false;
            }
            modal.description.push(c);
        }
        KeyCode::Char(c) if modal.field_index == 2 => {
            if modal.spec_is_default {
                modal.spec.clear();
                modal.spec_is_default = false;
            }
            modal.spec.push(c);
        }
        KeyCode::Char(c) if modal.field_index == 3 => {
            if modal.path_is_default {
                modal.path_is_default = false;
            }
            modal.path.push(c);
        }
        KeyCode::Enter if modal.field_index == 0 => modal.field_index = 1,
        KeyCode::Enter if modal.field_index == 1 => {
            if modal.description_is_default {
                modal.description.clear();
                modal.description_is_default = false;
            }
            modal.description.push('\n');
        }
        KeyCode::Enter if modal.field_index == 2 => modal.field_index = 3,
        KeyCode::Enter if modal.field_index == 3 => modal.field_index = 4,
        KeyCode::Enter => {
            if modal.confirm_selected {
                app.pending_action = Some(PendingUiAction::SubmitProjectModal(modal.clone()));
                app.busy_message = Some(if modal.mode == ProjectModalMode::Create {
                    "프로젝트 생성 초기화 실행 중".to_string()
                } else {
                    "프로젝트 수정 반영 중".to_string()
                });
            } else {
                app.status_line = "create project canceled".to_string();
            }
            close_modal = true;
        }
        _ => {}
    }

    if !close_modal {
        app.create_modal = Some(modal);
    }
    Ok(close_modal)
}

fn action_render_path_change_confirm_modal(
    f: &mut ratatui::Frame,
    area: Rect,
    confirm: &PathChangeConfirm,
) {
    let lines = vec![
        Line::from("Path changed. Move project directory?"),
        Line::from(format!("from: {}", confirm.old_path)),
        Line::from(format!("to: {}", confirm.new_path)),
    ];
    action_render_confirm_cancel_wrapper(
        f,
        area,
        "Move Project Path",
        &lines,
        "Move",
        "Keep",
        confirm.confirm_selected,
    );
}

fn action_render_delete_confirm_modal(
    f: &mut ratatui::Frame,
    area: Rect,
    confirm: &DeleteProjectConfirm,
) {
    let lines = vec![
        Line::from(format!("Delete project `{}`?", confirm.project_name)),
        Line::from(format!("path: {}", confirm.project_path)),
        Line::from("This removes all files/folders inside `<path>/.project`."),
    ];
    action_render_confirm_cancel_wrapper(
        f,
        area,
        "Delete Project",
        &lines,
        "Delete",
        "Cancel",
        confirm.confirm_selected,
    );
}

fn action_render_detail_fill_confirm_modal(
    f: &mut ratatui::Frame,
    area: Rect,
    project: &ProjectRecord,
    confirm: &DetailFillConfirm,
) {
    let lines = vec![
        Line::from(format!("project created: {}", project.name)),
        Line::from(format!("description: {}", project.description)),
        Line::from("project.md의 나머지 항목을 지금 채우시겠습니까?"),
    ];
    action_render_confirm_cancel_wrapper(
        f,
        area,
        "Fill Project Detail",
        &lines,
        "Open",
        "Skip",
        confirm.confirm_selected,
    );
}

fn action_render_draft_create_confirm_modal(
    f: &mut ratatui::Frame,
    area: Rect,
    confirm: &DraftCreateConfirm,
) {
    let lines = vec![
        Line::from("Drafts pane selected."),
        Line::from("Run `create-draft` now?"),
        Line::from("This triggers plan-drafts-code from current project."),
    ];
    action_render_confirm_cancel_wrapper(
        f,
        area,
        "Create Draft",
        &lines,
        "Run",
        "Cancel",
        confirm.confirm_selected,
    );
}

fn action_render_bootstrap_confirm_modal(
    f: &mut ratatui::Frame,
    area: Rect,
    confirm: &BootstrapConfirm,
) {
    let lines = vec![
        Line::from("상세 기획 반영이 완료되었습니다."),
        Line::from("spec 기준으로 프로젝트 bootstrap을 실행할까요?"),
        Line::from(format!("spec: {}", confirm.spec)),
    ];
    action_render_confirm_cancel_wrapper(
        f,
        area,
        "Project Bootstrap",
        &lines,
        "Bootstrap",
        "Skip",
        confirm.confirm_selected,
    );
}

fn action_render_confirm_buttons_bottom_right(
    f: &mut ratatui::Frame,
    inner: Rect,
    confirm_label: &str,
    cancel_label: &str,
    confirm_selected: bool,
) {
    let confirm_style = if confirm_selected {
        Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else {
        Style::default()
    };
    let cancel_style = if !confirm_selected {
        Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else {
        Style::default()
    };
    let button_area = Rect {
        x: inner.x,
        y: inner.y.saturating_add(inner.height.saturating_sub(1)),
        width: inner.width,
        height: 1,
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("[{}]", confirm_label), confirm_style),
            Span::raw("  "),
            Span::styled(format!("[{}]", cancel_label), cancel_style),
            Span::raw(" "),
        ]))
        .alignment(Alignment::Right),
        button_area,
    );
}

fn calc_ai_detail_input_border_style(modal: &AiChatModal) -> Style {
    if modal.focus == AiDetailFocus::Input && modal.input_active {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else if modal.focus == AiDetailFocus::Input {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM)
    }
}

fn action_render_confirm_cancel_wrapper(
    f: &mut ratatui::Frame,
    area: Rect,
    title: &str,
    lines: &[Line],
    confirm_label: &str,
    cancel_label: &str,
    confirm_selected: bool,
) {
    f.render_widget(Clear, area);
    let block = Block::default().title(title).borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let max_body_h = inner.height.saturating_sub(1).max(1);
    let body_h = (lines.len() as u16).min(max_body_h);
    let body_area = Rect {
        x: inner.x,
        y: inner
            .y
            .saturating_add(max_body_h.saturating_sub(body_h) / 2),
        width: inner.width,
        height: body_h,
    };
    f.render_widget(
        Paragraph::new(lines.to_vec())
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false }),
        body_area,
    );
    action_render_confirm_buttons_bottom_right(
        f,
        inner,
        confirm_label,
        cancel_label,
        confirm_selected,
    );
}

fn action_render_ai_chat_modal(
    f: &mut ratatui::Frame,
    area: Rect,
    modal: &AiChatModal,
) -> Option<(u16, u16)> {
    f.render_widget(Clear, area);
    let block = Block::default()
        .title(format!("AI Detail - {}", modal.project_name))
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(68),
            Constraint::Percentage(26),
            Constraint::Length(2),
        ])
        .split(inner);

    let mut lines: Vec<Line> = modal
        .history
        .iter()
        .flat_map(|msg| {
            let mut out = Vec::new();
            out.push(Line::from(msg.clone()));
            out.push(Line::from(""));
            out
        })
        .collect();
    if modal.streaming && !modal.warmup_inflight {
        lines.push(Line::from("AI 응답 생성중..."));
    }
    if lines.is_empty() {
        lines.push(Line::from(""));
    }
    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().title("Response").borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        split[0],
    );

    let hint = if modal.warmup_inflight {
        "초기 컨텍스트 전송중..."
    } else if modal.streaming {
        "AI 응답 수신중..."
    } else if modal.focus == AiDetailFocus::Input && !modal.input_active {
        "입력 비활성 | Enter: 입력 활성화 | ↓: 종료 버튼"
    } else if modal.focus == AiDetailFocus::CloseButton {
        "종료 버튼 포커스 | ↑: Input 포커스 | Enter: 종료"
    } else {
        "입력: Enter 줄바꿈, Enter 두번 전송 | Esc: 입력 비활성"
    };
    f.render_widget(
        Paragraph::new(modal.input.clone())
            .block(
                Block::default()
                    .title(format!("Input | {}", hint))
                    .borders(Borders::ALL)
                    .border_style(calc_ai_detail_input_border_style(modal)),
            )
            .wrap(Wrap { trim: false }),
        split[1],
    );

    let button_style = if modal.focus == AiDetailFocus::CloseButton {
        Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else {
        Style::default()
    };
    let button_label = "[ 대화 종료 ]";
    let button_text = Span::styled(button_label, button_style);
    let button_w = UnicodeWidthStr::width(button_label) as u16;
    let button_area = Rect {
        x: split[2]
            .x
            .saturating_add(split[2].width.saturating_sub(button_w.saturating_add(1))),
        y: split[2].y,
        width: button_w,
        height: 1,
    };
    f.render_widget(Paragraph::new(Line::from(button_text)), button_area);

    if modal.focus != AiDetailFocus::Input || !modal.input_active {
        None
    } else {
        Some(calc_cursor_in_input(split[1], &modal.input))
    }
}

fn action_render_busy_modal(f: &mut ratatui::Frame, area: Rect, message: &str) {
    f.render_widget(Clear, area);
    let block = Block::default().title("Working").borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let lines: Vec<Line> = vec![
        Line::from("작업중입니다. 잠시만 기다려주세요..."),
        Line::from(""),
        Line::from(message),
    ];
    let max_body_h = inner.height.saturating_sub(1).max(1);
    let body_h = (lines.len() as u16).min(max_body_h);
    let body_area = Rect {
        x: inner.x,
        y: inner
            .y
            .saturating_add(max_body_h.saturating_sub(body_h) / 2),
        width: inner.width,
        height: body_h,
    };
    f.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false }),
        body_area,
    );
}

pub fn flow_run_ui(
    projects: &mut Vec<ProjectRecord>,
    recent_active_pane: &mut Option<String>,
) -> Result<UiRunResult, String> {
    let palette = action_load_border_palette();
    let ids_changed = action_assign_missing_project_ids(projects);
    action_promote_recent_project_to_front(projects, recent_active_pane.as_deref());
    let mut app = UiApp {
        tab_index: 0,
        project_index: action_pick_selected_project_index(projects),
        pane_focus: 0,
        parallel_statuses: Vec::new(),
        parallel_running: false,
        last_tick: Instant::now(),
        status_line: "ready".to_string(),
        create_modal: None,
        detail_fill_confirm: None,
        draft_create_confirm: None,
        draft_bulk_add_modal: None,
        list_edit_modal: None,
        bootstrap_confirm: None,
        ai_chat_modal: None,
        path_change_confirm: None,
        delete_confirm: None,
        pending_action: None,
        busy_message: None,
        menu_active: true,
        changed: ids_changed,
        pane_activate_started_at: None,
        pane_activate_index: 0,
    };
    if !projects.is_empty() {
        action_set_selected(projects, app.project_index);
    }

    enable_raw_mode().map_err(|e| format!("failed to enable raw mode: {}", e))?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)
        .map_err(|e| format!("failed to enter alternate screen: {}", e))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|e| format!("failed to create terminal backend: {}", e))?;

    let mut run_result = Ok(UiRunResult {
        changed: false,
        message: "ui mode closed".to_string(),
        auto_mode_project: None,
    });

    'app_loop: loop {
        let _features = action_collect_feature_names(projects.get(app.project_index));

        if let Err(e) = terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(1),
                    Constraint::Length(4),
                ])
                .split(f.area());

            let header = Line::from(vec![
                ratatui::text::Span::styled(
                    "Project",
                    if app.tab_index == 0 {
                        Style::default()
                            .fg(palette.active)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(palette.inactive)
                    },
                ),
                " | ".into(),
                ratatui::text::Span::styled(
                    "Detail",
                    if app.tab_index == 1 {
                        Style::default()
                            .fg(palette.active)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(palette.inactive)
                    },
                ),
            ]);
            let header_block = Block::default()
                .title("Current Pane")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(palette.active));
            let header_inner = header_block.inner(chunks[0]);
            f.render_widget(header_block, chunks[0]);
            let header_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                .split(header_inner);
            f.render_widget(Paragraph::new(header), header_layout[0]);
            f.render_widget(
                Paragraph::new("switch : tab").alignment(Alignment::Right),
                header_layout[1],
            );

            if app.tab_index == 0 {
                let overlay_modal = calc_has_overlay_modal(&app);
                action_render_projects_tab(
                    f,
                    chunks[1],
                    projects,
                    app.project_index,
                    app.menu_active,
                    overlay_modal,
                    app.parallel_running,
                    palette,
                );
            } else {
                action_render_details_tab(
                    f,
                    chunks[1],
                    &app,
                    projects,
                    &_features,
                    app.menu_active,
                    palette,
                );
            }

            let running = if app.parallel_running { "running" } else { "idle" };
            let shared_help = "q: exit | tab: switch | a: create-project | m: edit-project | d: delete-project";
            let modal_help = "tab: move field | type/backspace: edit | esc: close";
            let footer = if app.create_modal.is_some() {
                format!(
                    "{} | {} | status: {} ({})",
                    shared_help, modal_help, app.status_line, running
                )
            } else if app.path_change_confirm.is_some() {
                format!(
                    "{} | y/n apply | esc cancel | status: {} ({})",
                    shared_help, app.status_line, running
                )
            } else if app.delete_confirm.is_some() {
                format!(
                    "{} | y/n apply | esc cancel | status: {} ({})",
                    shared_help, app.status_line, running
                )
            } else if app.detail_fill_confirm.is_some() {
                format!(
                    "{} | y/n apply | esc cancel | status: {} ({})",
                    shared_help, app.status_line, running
                )
            } else if app.ai_chat_modal.is_some() {
                format!(
                    "{} | ai-modal: send message | esc close | status: {} ({})",
                    shared_help, app.status_line, running
                )
            } else if app.draft_bulk_add_modal.is_some() {
                format!(
                    "{} | drafts-add: type(enter=newline) | tab: input/button | ←/→ choose | esc close | status: {} ({})",
                    shared_help, app.status_line, running
                )
            } else if let Some(modal) = &app.list_edit_modal {
                if modal.input_mode.is_some() {
                    format!(
                        "{} | list-edit: type | esc cancel-item | status: {} ({})",
                        shared_help, app.status_line, running
                    )
                } else {
                    format!(
                        "{} | list-edit: a/n add | e edit | d delete | esc cancel | status: {} ({})",
                        shared_help, app.status_line, running
                    )
                }
            } else if app.draft_create_confirm.is_some() {
                format!(
                    "{} | y/n apply | esc cancel | status: {} ({})",
                    shared_help, app.status_line, running
                )
            } else if app.bootstrap_confirm.is_some() {
                format!(
                    "{} | y/n apply | esc cancel | status: {} ({})",
                    shared_help, app.status_line, running
                )
            } else if app.menu_active && app.tab_index == 1 && app.pane_focus == 4 {
                format!(
                    "{} | drafts: a add-drafts | b build-draft/run-parallel | status: {} ({})",
                    shared_help, app.status_line, running
                )
            } else {
                format!("{} | status: {} ({})", shared_help, app.status_line, running)
            };
            let footer_widget = Paragraph::new(footer).block(
                Block::default()
                    .title("bar_status")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(palette.normal)),
            );
            f.render_widget(footer_widget, chunks[2]);

            if let Some(modal) = &app.create_modal {
                let modal_rect = calc_centered_rect(70, 55, f.area());
                if let Some((x, y)) = action_render_create_modal(f, modal_rect, modal) {
                    f.set_cursor_position((x, y));
                }
            } else if let Some(confirm) = &app.path_change_confirm {
                let modal_rect = calc_centered_rect(70, 35, f.area());
                action_render_path_change_confirm_modal(f, modal_rect, confirm);
            } else if let Some(confirm) = &app.delete_confirm {
                let modal_rect = calc_centered_rect(70, 35, f.area());
                action_render_delete_confirm_modal(f, modal_rect, confirm);
            } else if let Some(modal) = &app.draft_bulk_add_modal {
                let modal_rect = calc_centered_rect(82, 65, f.area());
                if let Some((x, y)) = action_render_draft_bulk_add_modal(f, modal_rect, modal) {
                    f.set_cursor_position((x, y));
                }
            } else if let Some(confirm) = &app.detail_fill_confirm {
                if let Some(project) = projects.get(confirm.project_index) {
                    let modal_rect = calc_centered_rect(70, 35, f.area());
                    action_render_detail_fill_confirm_modal(f, modal_rect, project, confirm);
                }
            } else if let Some(modal) = &app.list_edit_modal {
                let modal_rect = calc_centered_rect(92, 88, f.area());
                if let Some((x, y)) = action_render_list_edit_modal(f, modal_rect, modal) {
                    f.set_cursor_position((x, y));
                }
            } else if let Some(confirm) = &app.draft_create_confirm {
                let modal_rect = calc_centered_rect(70, 35, f.area());
                action_render_draft_create_confirm_modal(f, modal_rect, confirm);
            } else if let Some(confirm) = &app.bootstrap_confirm {
                let modal_rect = calc_centered_rect(70, 35, f.area());
                action_render_bootstrap_confirm_modal(f, modal_rect, confirm);
            } else if let Some(modal) = &app.ai_chat_modal {
                let modal_rect = calc_centered_rect(85, 80, f.area());
                if let Some((x, y)) = action_render_ai_chat_modal(f, modal_rect, modal) {
                    f.set_cursor_position((x, y));
                }
            }
            if let Some(message) = &app.busy_message {
                let modal_rect = calc_centered_rect(55, 25, f.area());
                action_render_busy_modal(f, modal_rect, message);
            }
        }) {
            run_result = Err(format!("ui draw failed: {}", e));
            break 'app_loop;
        }

        if let Some(action) = app.pending_action.take() {
            let result = match action {
                PendingUiAction::SubmitProjectModal(modal) => {
                    if modal.mode == ProjectModalMode::Create {
                        action_apply_project_create(projects, &mut app, &modal)
                    } else {
                        action_try_submit_edit_project(projects, &mut app, &modal)
                    }
                }
                PendingUiAction::ApplyPathChange { confirm, move_dir } => {
                    action_apply_path_change_confirm(projects, &mut app, confirm, move_dir)
                }
                PendingUiAction::ApplyDelete { confirm, accepted } => {
                    action_apply_delete_confirm(projects, &mut app, confirm, accepted)
                }
                PendingUiAction::ApplyCreateDraft { project_index } => {
                    action_apply_draft_create_via_cli(projects, &mut app, project_index)
                }
                PendingUiAction::ApplyDraftBulkAdd {
                    project_index,
                    raw_input,
                } => action_apply_draft_bulk_add_via_cli(projects, &mut app, project_index, &raw_input),
            };
            app.busy_message = None;
            if let Err(e) = result {
                app.status_line = e;
            }
            continue;
        }

        if app.parallel_running && app.last_tick.elapsed() >= Duration::from_millis(350) {
            action_advance_parallel_runtime(&mut app, projects);
            app.last_tick = Instant::now();
        }

        if let Some(modal) = app.ai_chat_modal.as_mut() {
            if let Some(rx) = modal.stream_rx.as_ref() {
                loop {
                    match rx.try_recv() {
                        Ok(AiStreamEvent::Chunk(chunk)) => {
                            if !modal.warmup_inflight {
                                modal.streaming_buffer.push_str(&chunk);
                            }
                        }
                        Ok(AiStreamEvent::Done) => {
                            modal.streaming = false;
                            if modal.warmup_inflight {
                                modal.warmup_inflight = false;
                                modal.streaming_buffer.clear();
                                modal.stream_rx = None;
                                modal.stream_cancel = None;
                                app.status_line = "ai detail ready".to_string();
                                break;
                            }
                            let raw_response = modal.streaming_buffer.trim().to_string();
                            let response = if !modal.allow_full_md_response
                                && calc_is_project_md_dump(&raw_response)
                            {
                                "전체 project.md 출력이 감지되어 화면 표시를 제한했습니다.\n필요하면 `project.md 전체 업데이트`라고 입력해 주세요."
                                    .to_string()
                            } else {
                                raw_response.clone()
                            };
                            modal.history.push(format!("AI:\n{}", response));
                            if let Some(md) = calc_extract_markdown_block(&raw_response) {
                                let path = Path::new(&modal.project_path).join(".project").join("project.md");
                                if fs::write(&path, md).is_ok() {
                                    app.status_line =
                                        "ai response applied to .project/project.md".to_string();
                                }
                            }
                            modal.streaming_buffer.clear();
                            modal.stream_rx = None;
                            modal.stream_cancel = None;
                            break;
                        }
                        Ok(AiStreamEvent::Error(err)) => {
                            modal.streaming = false;
                            if modal.warmup_inflight {
                                modal.warmup_inflight = false;
                                app.status_line = "ai detail warmup failed".to_string();
                            } else {
                                modal.history.push(format!("AI error:\n{}", err));
                                app.status_line = "ai response failed".to_string();
                            }
                            modal.streaming_buffer.clear();
                            modal.stream_rx = None;
                            modal.stream_cancel = None;
                            break;
                        }
                        Ok(AiStreamEvent::Cancelled) => {
                            modal.streaming = false;
                            modal.warmup_inflight = false;
                            modal.streaming_buffer.clear();
                            modal.stream_rx = None;
                            modal.stream_cancel = None;
                            app.status_line = "ai request canceled".to_string();
                            break;
                        }
                        Err(mpsc::TryRecvError::Empty) => break,
                        Err(mpsc::TryRecvError::Disconnected) => {
                            modal.streaming = false;
                            modal.stream_rx = None;
                            modal.stream_cancel = None;
                            break;
                        }
                    }
                }
            }
        }

        let has_event =
            event::poll(Duration::from_millis(80)).map_err(|e| format!("ui event poll failed: {}", e))?;
        if !has_event {
            continue;
        }

        let ev = event::read().map_err(|e| format!("ui event read failed: {}", e))?;
        if let Event::Key(key_event) = ev {
            if !matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                continue;
            }
            if let Some(modal) = app.draft_bulk_add_modal.as_mut() {
                match key_event.code {
                    KeyCode::Esc => {
                        app.draft_bulk_add_modal = None;
                        app.status_line = "draft add canceled".to_string();
                    }
                    KeyCode::Tab => {
                        modal.input_focus = !modal.input_focus;
                    }
                    KeyCode::Left | KeyCode::Right if !modal.input_focus => {
                        modal.confirm_selected = !modal.confirm_selected;
                    }
                    KeyCode::Up if !modal.input_focus => {
                        modal.input_focus = true;
                    }
                    KeyCode::Down if modal.input_focus => {
                        modal.input_focus = false;
                    }
                    KeyCode::Backspace if modal.input_focus => {
                        modal.input.pop();
                    }
                    KeyCode::Enter if modal.input_focus => {
                        modal.input.push('\n');
                    }
                    KeyCode::Char(c) if modal.input_focus => {
                        modal.input.push(c);
                    }
                    KeyCode::Enter => {
                        if modal.confirm_selected {
                            app.pending_action = Some(PendingUiAction::ApplyDraftBulkAdd {
                                project_index: modal.project_index,
                                raw_input: modal.input.clone(),
                            });
                            app.busy_message = Some("draft 추가 요청 실행 중".to_string());
                        } else {
                            app.status_line = "draft add canceled".to_string();
                        }
                        app.draft_bulk_add_modal = None;
                    }
                    _ => {}
                }
                continue;
            }
            if let Some(modal) = app.list_edit_modal.as_mut() {
                if let Some(mode) = modal.input_mode {
                    match key_event.code {
                        KeyCode::Esc => {
                            modal.input_mode = None;
                            modal.input.clear();
                        }
                        KeyCode::Backspace => {
                            modal.input.pop();
                        }
                        KeyCode::Char(c) => {
                            modal.input.push(c);
                        }
                        KeyCode::Enter => {
                            let item = modal.input.trim().to_string();
                            if !item.is_empty() {
                                if matches!(modal.target, ListEditTarget::Feature)
                                    && calc_normalize_feature_item(&item).is_err()
                                {
                                    app.status_line = "feature format: 기능명 : 설명".to_string();
                                    continue;
                                }
                                match mode {
                                    ListEditInputMode::Add => {
                                        modal.items.push(item.clone());
                                        if !modal.items.is_empty() {
                                            modal.selected_index = modal.items.len() - 1;
                                        }
                                        app.status_line = "list item added".to_string();
                                    }
                                    ListEditInputMode::Edit => {
                                        if !modal.items.is_empty() && modal.selected_index < modal.items.len() {
                                            modal.items[modal.selected_index] = item.clone();
                                            app.status_line = "list item updated".to_string();
                                        }
                                    }
                                }
                            }
                            modal.input_mode = None;
                            modal.input.clear();
                        }
                        _ => {}
                    }
                    continue;
                }
                match key_event.code {
                    KeyCode::Esc => {
                        app.list_edit_modal = None;
                        app.status_line = "list modal canceled".to_string();
                    }
                    KeyCode::Char('a') => {
                        modal.input_mode = Some(ListEditInputMode::Add);
                        modal.input.clear();
                    }
                    KeyCode::Char('n') => {
                        modal.input_mode = Some(ListEditInputMode::Add);
                        modal.input.clear();
                    }
                    KeyCode::Char('e') => {
                        if !modal.items.is_empty() && modal.selected_index < modal.items.len() {
                            modal.input_mode = Some(ListEditInputMode::Edit);
                            modal.input = modal.items[modal.selected_index].clone();
                        }
                    }
                    KeyCode::Char('d') => {
                        if !modal.items.is_empty() && modal.selected_index < modal.items.len() {
                            modal.items.remove(modal.selected_index);
                            if modal.selected_index > 0 && modal.selected_index >= modal.items.len() {
                                modal.selected_index -= 1;
                            }
                            app.status_line = "list item deleted".to_string();
                        }
                    }
                    KeyCode::Left | KeyCode::Right => {
                        modal.confirm_selected = !modal.confirm_selected;
                    }
                    KeyCode::Enter => {
                        if modal.confirm_selected {
                            action_save_project_md_list(
                                projects,
                                modal.project_index,
                                modal.target,
                                &modal.items,
                            )?;
                            app.changed = true;
                            app.list_edit_modal = None;
                            app.status_line = "list items applied".to_string();
                        } else {
                            app.list_edit_modal = None;
                            app.status_line = "list modal canceled".to_string();
                        }
                    }
                    KeyCode::Up => {
                        if modal.selected_index > 0 {
                            modal.selected_index -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if modal.selected_index + 1 < modal.items.len() {
                            modal.selected_index += 1;
                        }
                    }
                    _ => {}
                }
                continue;
            }
            if app.bootstrap_confirm.is_some() {
                let mut confirm = app.bootstrap_confirm.take().unwrap();
                match key_event.code {
                    KeyCode::Left | KeyCode::Right => {
                        confirm.confirm_selected = !confirm.confirm_selected;
                        app.bootstrap_confirm = Some(confirm);
                    }
                    KeyCode::Enter => {
                        if confirm.confirm_selected {
                            if let Err(e) = action_apply_bootstrap(projects, &mut app, &confirm) {
                                app.status_line = e;
                            } else {
                                app.changed = true;
                            }
                        } else {
                            app.status_line = "bootstrap skipped".to_string();
                        }
                    }
                    KeyCode::Esc => {
                        app.status_line = "bootstrap skipped".to_string();
                    }
                    _ => {
                        app.bootstrap_confirm = Some(confirm);
                    }
                }
                continue;
            }
            if let Some(modal) = app.ai_chat_modal.as_mut() {
                match key_event.code {
                    KeyCode::Esc => {
                        if modal.focus == AiDetailFocus::Input && modal.input_active && !modal.streaming {
                            modal.input_active = false;
                            modal.input_enter_streak = 0;
                            app.status_line = "ai input inactive".to_string();
                        }
                    }
                    KeyCode::Down => {
                        if !modal.streaming
                            && modal.focus == AiDetailFocus::Input
                            && !modal.input_active
                        {
                            modal.focus = AiDetailFocus::CloseButton;
                        }
                    }
                    KeyCode::Up => {
                        if !modal.streaming && modal.focus == AiDetailFocus::CloseButton {
                            modal.focus = AiDetailFocus::Input;
                            modal.input_active = false;
                        }
                    }
                    KeyCode::Backspace => {
                        if !modal.streaming
                            && modal.focus == AiDetailFocus::Input
                            && modal.input_active
                        {
                            modal.input.pop();
                            modal.input_enter_streak = 0;
                        }
                    }
                    KeyCode::Char(c) => {
                        if !modal.streaming
                            && modal.focus == AiDetailFocus::Input
                            && modal.input_active
                        {
                            modal.input.push(c);
                            modal.input_enter_streak = 0;
                        }
                    }
                    KeyCode::Enter => {
                        if modal.streaming {
                            continue;
                        }
                        if modal.focus == AiDetailFocus::CloseButton {
                            let idx = modal.project_index;
                            action_close_ai_chat_modal_and_open_bootstrap(&mut app, projects, idx);
                            continue;
                        }
                        if modal.focus == AiDetailFocus::Input && !modal.input_active {
                            modal.input_active = true;
                            modal.input_enter_streak = 0;
                            app.status_line = "ai input active".to_string();
                            continue;
                        }
                        modal.input_enter_streak = modal.input_enter_streak.saturating_add(1);
                        if modal.input_enter_streak >= 2 {
                            let msg = modal.input.trim().to_string();
                            modal.input.clear();
                            modal.input_enter_streak = 0;
                            if msg.is_empty() {
                                continue;
                            }
                            modal.allow_full_md_response = calc_is_full_project_md_request(&msg);
                            let user_line = format!("You:\n{}", msg);
                            modal.history.push(user_line.clone());
                            let prompt = action_build_ai_chat_prompt(modal, &msg);
                            modal.streaming = true;
                            modal.streaming_buffer.clear();
                            let (rx, cancel) = action_spawn_ai_stream(&modal.model_bin, prompt);
                            modal.stream_rx = Some(rx);
                            modal.stream_cancel = Some(cancel);
                            app.status_line = "ai request sent".to_string();
                        } else {
                            modal.input.push('\n');
                        }
                    }
                    _ => {}
                }
                continue;
            }
            if app.detail_fill_confirm.is_some() {
                let mut confirm = app.detail_fill_confirm.take().unwrap();
                match key_event.code {
                    KeyCode::Left | KeyCode::Right => {
                        confirm.confirm_selected = !confirm.confirm_selected;
                        app.detail_fill_confirm = Some(confirm);
                    }
                    KeyCode::Enter => {
                        if confirm.confirm_selected {
                            action_open_ai_chat_modal(&mut app, projects, confirm.project_index);
                        } else {
                            app.status_line = "skip detail fill".to_string();
                            action_open_bootstrap_confirm(&mut app, projects, confirm.project_index);
                        }
                    }
                    KeyCode::Char('y') | KeyCode::Char('Y') if confirm.confirm_selected => {
                        action_open_ai_chat_modal(&mut app, projects, confirm.project_index);
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        app.status_line = "skip detail fill".to_string();
                        action_open_bootstrap_confirm(&mut app, projects, confirm.project_index);
                    }
                    _ => {
                        app.detail_fill_confirm = Some(confirm);
                    }
                }
                continue;
            }
            if app.draft_create_confirm.is_some() {
                let mut confirm = app.draft_create_confirm.take().unwrap();
                match key_event.code {
                    KeyCode::Left | KeyCode::Right => {
                        confirm.confirm_selected = !confirm.confirm_selected;
                        app.draft_create_confirm = Some(confirm);
                    }
                    KeyCode::Enter => {
                        if confirm.confirm_selected {
                            app.pending_action = Some(PendingUiAction::ApplyCreateDraft {
                                project_index: confirm.project_index,
                            });
                            app.busy_message = Some("draft 생성 요청 실행 중".to_string());
                        } else {
                            app.status_line = "draft create canceled".to_string();
                        }
                    }
                    KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                        app.status_line = "draft create canceled".to_string();
                    }
                    KeyCode::Char('y') | KeyCode::Char('Y') if confirm.confirm_selected => {
                        app.pending_action = Some(PendingUiAction::ApplyCreateDraft {
                            project_index: confirm.project_index,
                        });
                        app.busy_message = Some("draft 생성 요청 실행 중".to_string());
                    }
                    _ => {
                        app.draft_create_confirm = Some(confirm);
                    }
                }
                continue;
            }
            if app.delete_confirm.is_some() {
                let mut confirm = app.delete_confirm.take().unwrap();
                match key_event.code {
                    KeyCode::Left | KeyCode::Right => {
                        confirm.confirm_selected = !confirm.confirm_selected;
                        app.delete_confirm = Some(confirm);
                    }
                    KeyCode::Enter => {
                        if confirm.confirm_selected {
                            app.pending_action = Some(PendingUiAction::ApplyDelete {
                                confirm,
                                accepted: true,
                            });
                            app.busy_message = Some("프로젝트 삭제 처리 중".to_string());
                        } else {
                            action_apply_delete_confirm(projects, &mut app, confirm, false)?;
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        action_apply_delete_confirm(projects, &mut app, confirm, false)?;
                    }
                    _ => {
                        app.delete_confirm = Some(confirm);
                    }
                }
                continue;
            }
            if app.path_change_confirm.is_some() {
                let mut confirm = app.path_change_confirm.take().unwrap();
                match key_event.code {
                    KeyCode::Left | KeyCode::Right => {
                        confirm.confirm_selected = !confirm.confirm_selected;
                        app.path_change_confirm = Some(confirm);
                    }
                    KeyCode::Enter => {
                        if confirm.confirm_selected {
                            app.pending_action = Some(PendingUiAction::ApplyPathChange {
                                confirm,
                                move_dir: true,
                            });
                            app.busy_message = Some("프로젝트 경로 이동 처리 중".to_string());
                        } else {
                            action_apply_path_change_confirm(projects, &mut app, confirm, false)?;
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        action_apply_path_change_confirm(projects, &mut app, confirm, false)?;
                    }
                    KeyCode::Esc => {
                        app.status_line = "path change canceled".to_string();
                    }
                    _ => {
                        app.path_change_confirm = Some(confirm);
                    }
                }
                continue;
            }
            if app.create_modal.is_some() {
                let _ = action_handle_modal_input(projects, &mut app, key_event.code)?;
                continue;
            }
            match key_event.code {
                KeyCode::Char('q') => {
                    if app.menu_active {
                        app.menu_active = false;
                        app.status_line = "focus closed (inactive)".to_string();
                    } else {
                        action_cancel_ai_stream(&mut app);
                        break 'app_loop;
                    }
                }
                KeyCode::Enter => {
                    if !app.menu_active {
                        app.menu_active = true;
                        app.status_line = "focus active".to_string();
                        action_start_pane_activate_tween(&mut app);
                    } else if app.tab_index == 0 {
                        app.tab_index = 1;
                        if let Some(project) = projects.get(app.project_index) {
                            *recent_active_pane = Some(project.id.clone());
                            app.changed = true;
                        }
                        app.status_line = "moved to detail tab".to_string();
                        action_start_pane_activate_tween(&mut app);
                    } else if app.tab_index == 1 && app.pane_focus == 1 {
                        action_open_list_edit_modal(&mut app, projects, ListEditTarget::Rule);
                    } else if app.tab_index == 1 && app.pane_focus == 2 {
                        action_open_list_edit_modal(
                            &mut app,
                            projects,
                            ListEditTarget::Constraint,
                        );
                    } else if app.tab_index == 1 && app.pane_focus == 3 {
                        action_open_list_edit_modal(&mut app, projects, ListEditTarget::Feature);
                    } else if app.tab_index == 1 && app.pane_focus == 4 {
                        if let Some(project) = projects.get(app.project_index) {
                            let planned = action_collect_planned_drafts_from_project(project);
                            if planned.is_empty() {
                                app.pending_action = Some(PendingUiAction::ApplyCreateDraft {
                                    project_index: app.project_index,
                                });
                                app.busy_message = Some("draft 생성 요청 실행 중".to_string());
                            } else {
                                action_start_parallel_runtime(&mut app, &planned);
                            }
                        } else {
                            app.status_line = "no selected project".to_string();
                        }
                    } else {
                        app.status_line = "focus active".to_string();
                        action_start_pane_activate_tween(&mut app);
                    }
                }
                KeyCode::Char('a')
                    if app.menu_active && app.tab_index == 1 && app.pane_focus == 4 =>
                {
                    let project_index = app.project_index;
                    action_open_draft_bulk_add_modal(&mut app, project_index);
                }
                KeyCode::Char('a') if app.menu_active => action_open_create_modal(&mut app),
                KeyCode::Char('m') if app.menu_active && app.tab_index == 0 => {
                    action_open_edit_modal(&mut app, projects);
                }
                KeyCode::Char('d') if app.menu_active && app.tab_index == 0 => {
                    action_open_delete_confirm(&mut app, projects);
                }
                KeyCode::Char('b') if app.menu_active && app.tab_index == 1 && app.pane_focus == 4 => {
                    if let Some(project) = projects.get(app.project_index) {
                        let planned = action_collect_planned_drafts_from_project(project);
                        if planned.is_empty() {
                            app.pending_action = Some(PendingUiAction::ApplyCreateDraft {
                                project_index: app.project_index,
                            });
                            app.busy_message = Some("draft 생성 요청 실행 중".to_string());
                        } else {
                            action_start_parallel_runtime(&mut app, &planned);
                        }
                    } else {
                        app.status_line = "no selected project".to_string();
                    }
                }
                KeyCode::Tab => {
                    if !app.menu_active {
                        continue;
                    }
                    app.tab_index = (app.tab_index + 1) % 2;
                    app.status_line = format!("tab changed to {}", app.tab_index + 1);
                }
                KeyCode::Char('1') if app.menu_active => app.tab_index = 0,
                KeyCode::Char('2') if app.menu_active => app.tab_index = 1,
                KeyCode::Char('k') => action_move_project_grid_selection(projects, &mut app, -3),
                KeyCode::Char('j') => action_move_project_grid_selection(projects, &mut app, 3),
                KeyCode::Up if app.tab_index == 0 => {
                    action_move_project_grid_selection(projects, &mut app, -3);
                }
                KeyCode::Down if app.tab_index == 0 => {
                    action_move_project_grid_selection(projects, &mut app, 3);
                }
                KeyCode::Left if app.tab_index == 0 => {
                    action_move_project_grid_selection(projects, &mut app, -1);
                }
                KeyCode::Right if app.tab_index == 0 => {
                    action_move_project_grid_selection(projects, &mut app, 1);
                }
                KeyCode::Left if app.tab_index == 1 => action_move_detail_pane_focus(&mut app, KeyCode::Left),
                KeyCode::Right if app.tab_index == 1 => action_move_detail_pane_focus(&mut app, KeyCode::Right),
                KeyCode::Up if app.tab_index == 1 => action_move_detail_pane_focus(&mut app, KeyCode::Up),
                KeyCode::Down if app.tab_index == 1 => action_move_detail_pane_focus(&mut app, KeyCode::Down),
                _ => {}
            }
        }
    }

    action_cancel_ai_stream(&mut app);

    let leave_screen_result = execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .map_err(|e| format!("failed to leave alternate screen: {}", e));
    let raw_off_result =
        disable_raw_mode().map_err(|e| format!("failed to disable raw mode: {}", e));
    let cursor_result = terminal
        .show_cursor()
        .map_err(|e| format!("failed to show cursor: {}", e));

    if let Err(e) = leave_screen_result {
        return Err(e);
    }
    if let Err(e) = raw_off_result {
        return Err(e);
    }
    if let Err(e) = cursor_result {
        return Err(e);
    }

    if run_result.is_ok() {
        run_result = Ok(UiRunResult {
            changed: app.changed,
            message: "ui mode closed".to_string(),
            auto_mode_project: None,
        });
    }
    run_result
}
