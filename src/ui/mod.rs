mod component;

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
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const UI_REGISTRY_PATH: &str = "configs/project.yaml";

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct UiProjectRegistry {
    #[serde(default, rename = "recentActivepane")]
    recent_active_pane: Option<String>,
    #[serde(default)]
    projects: Vec<ProjectRecord>,
}

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

pub fn resolve_feature_draft_path(feature_name: &str) -> PathBuf {
    PathBuf::from(".project")
        .join("feature")
        .join(feature_name)
        .join("drafts.yaml")
}

pub fn apply_draft_create_update_delete(
    command: DraftCommand,
    feature_name: &str,
    patch_content: Option<&str>,
) -> Result<PathBuf, String> {
    let draft_path = resolve_feature_draft_path(feature_name);
    let parent = draft_path
        .parent()
        .ok_or_else(|| "failed to resolve draft parent path".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;

    match command {
        DraftCommand::Create => {
            let template_path = resolve_draft_template_path()?;
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

#[derive(Debug, Clone, Deserialize)]
struct DetailLayoutGridDoc {
    columns: u16,
    rows: u16,
}

impl Default for DetailLayoutGridDoc {
    fn default() -> Self {
        Self {
            columns: 10,
            rows: 10,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct DetailLayoutPanelDoc {
    id: String,
    name: String,
    #[serde(rename = "type")]
    panel_type: String,
    #[serde(default)]
    selected_view: String,
    #[serde(default)]
    shortcut: String,
    cell_start: u16,
    cell_end: u16,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct DetailLayoutDoc {
    #[serde(default)]
    grid: DetailLayoutGridDoc,
    #[serde(default)]
    panels: Vec<DetailLayoutPanelDoc>,
}

#[derive(Debug, Clone)]
struct DetailLayoutGrid {
    columns: u16,
    rows: u16,
}

#[derive(Debug, Clone)]
struct DetailLayoutPanel {
    id: String,
    name: String,
    panel_type: String,
    selected_view: String,
    shortcut: String,
    cell_start: u16,
    cell_end: u16,
}

#[derive(Debug, Clone)]
struct DetailLayoutPreset {
    preset: String,
    grid: DetailLayoutGrid,
    panels: Vec<DetailLayoutPanel>,
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
    domains: Vec<String>,
    #[serde(default)]
    flows: Vec<String>,
    #[serde(default)]
        features: Vec<String>,
    #[serde(default)]
    planned: Vec<String>,
    #[serde(default)]
    planned_items: Vec<PlannedItemDoc>,
    #[serde(default)]
    draft_state: DraftStateDoc,
    #[serde(default)]
    sync_initialized: bool,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
struct PlannedItemDoc {
    name: String,
    value: String,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
struct DraftStateDoc {
    #[serde(default)]
    generated: Vec<String>,
    #[serde(default)]
    pending: Vec<String>,
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

#[derive(Debug, Default, Clone, Deserialize)]
struct ProjectPresetDoc {
    #[serde(default)]
    presets: Vec<ProjectPresetItem>,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct ProjectPresetItem {
    #[serde(default)]
    name: String,
    #[serde(default)]
    libraries: Vec<String>,
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

#[derive(Debug, Clone)]
struct AlarmModal {
    message: String,
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
    alarm_modal: Option<AlarmModal>,
    pending_action: Option<PendingUiAction>,
    busy_message: Option<String>,
    parallel_build_rx: Option<Receiver<Result<String, String>>>,
    menu_active: bool,
    changed: bool,
    pane_activate_started_at: Option<Instant>,
    pane_activate_index: usize,
    detail_layout: DetailLayoutPreset,
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
    ApplyBootstrap {
        confirm: BootstrapConfirm,
    },
    ApplyCreateDraft {
        project_index: usize,
    },
    ApplyBuildParallel {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AiChatMode {
    DetailProject,
    AddPlan,
}

#[derive(Debug)]
struct AiChatModal {
    project_index: usize,
    project_path: String,
    project_name: String,
    project_description: String,
    initial_spec: String,
    mode: AiChatMode,
    model_bin: String,
    warmup_inflight: bool,
    input: String,
    input_enter_streak: u8,
    focus: AiDetailFocus,
    input_active: bool,
    allow_full_md_response: bool,
    add_plan_apply_requested: bool,
    history: Vec<String>,
    streaming: bool,
    streaming_buffer: String,
    stream_rx: Option<Receiver<AiStreamEvent>>,
    stream_cancel: Option<Arc<AtomicBool>>,
}

pub struct UiRunResult {
    pub changed: bool,
    pub message: String,
}

fn parse_color(name: Option<&str>, fallback: Color) -> Color {
    match name.unwrap_or("").to_ascii_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "orange" => Color::Rgb(255, 165, 0),
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "lightmagenta" | "light_magenta" | "brightmagenta" | "bright_magenta" => {
            Color::LightMagenta
        }
        "cyan" => Color::Cyan,
        "white" => Color::White,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "dark_gray" => Color::DarkGray,
        _ => fallback,
    }
}

fn load_border_palette() -> BorderPalette {
    let root = binary_root();
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
                let active = parse_color(
                    doc.active.as_ref().and_then(|v| v.border.as_deref()),
                    Color::Rgb(255, 165, 0),
                );
                let normal = parse_color(
                    doc.normal.as_ref().and_then(|v| v.border.as_deref()),
                    Color::Black,
                );
                let inactive = parse_color(
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
        active: Color::Rgb(255, 165, 0),
        normal: Color::Black,
        inactive: Color::Gray,
    }
}

fn binary_root() -> PathBuf {
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            return parent.to_path_buf();
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn ui_registry_path() -> PathBuf {
    crate::source_root().join(UI_REGISTRY_PATH)
}

fn reload_projects_from_registry(
    projects: &mut Vec<ProjectRecord>,
    recent_active_pane: &mut Option<String>,
    app: &mut UiApp,
) -> Result<(), String> {
    let path = ui_registry_path();
    if !path.exists() {
        return Ok(());
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let parsed: UiProjectRegistry = serde_yaml::from_str(&raw)
        .map_err(|e| format!("failed to parse {}: {}", path.display(), e))?;
    *projects = parsed.projects;
    *recent_active_pane = parsed.recent_active_pane;
    if !projects.is_empty() {
        app.project_index = pick_selected_project_index(projects);
        promote_recent_project_to_front(projects, recent_active_pane.as_deref());
        app.project_index = pick_selected_project_index(projects);
        set_selected(projects, app.project_index);
    } else {
        app.project_index = 0;
    }
    Ok(())
}

fn save_projects_to_registry(
    projects: &[ProjectRecord],
    recent_active_pane: &Option<String>,
) -> Result<(), String> {
    let path = ui_registry_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
    }
    let doc = UiProjectRegistry {
        recent_active_pane: recent_active_pane.clone(),
        projects: projects.to_vec(),
    };
    let raw = serde_yaml::to_string(&doc)
        .map_err(|e| format!("failed to encode {}: {}", path.display(), e))?;
    fs::write(&path, raw).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn resolve_draft_template_path() -> Result<PathBuf, String> {
    let root = binary_root();
    let manifest_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest_root
            .join("assets")
            .join("code")
            .join("templates")
            .join("drafts.yaml"),
        manifest_root.join("assets").join("templates").join("drafts.yaml"),
        root.join("assets").join("code").join("templates").join("drafts.yaml"),
        PathBuf::from("assets")
            .join("code")
            .join("templates")
            .join("drafts.yaml"),
        root.join("assets").join("templates").join("drafts.yaml"),
        PathBuf::from("assets").join("templates").join("drafts.yaml"),
        root.join("src").join("assets").join("templates").join("drafts.yaml"),
        PathBuf::from("src").join("assets").join("templates").join("drafts.yaml"),
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

fn resolve_project_preset_path() -> Result<PathBuf, String> {
    let root = binary_root();
    let candidates = [
        root.join("assets").join("presets").join("project.yaml"),
        root.join("assets").join("presets").join("project.yml"),
        PathBuf::from("assets").join("presets").join("project.yaml"),
        PathBuf::from("assets").join("presets").join("project.yml"),
        root.join("src").join("assets").join("presets").join("project.yaml"),
        root.join("src").join("assets").join("presets").join("project.yml"),
    ];
    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err("project preset not found".to_string())
}

fn is_allowed_spec_library(value_lc: &str) -> bool {
    matches!(
        value_lc,
        "react"
            | "react-dom"
            | "next"
            | "vite"
            | "typescript"
            | "javascript"
            | "axios"
            | "zod"
            | "zustand"
            | "@tanstack/react-query"
            | "tailwindcss"
            | "three"
            | "@react-three/fiber"
            | "@react-three/drei"
            | "react-native"
            | "expo"
            | "rust"
            | "tokio"
            | "serde"
            | "serde_json"
            | "reqwest"
            | "axum"
    )
}

fn filter_allowed_preset_libraries(libraries: &[String]) -> Vec<String> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out = Vec::new();
    for lib in libraries {
        let trimmed = lib.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_ascii_lowercase();
        if !is_allowed_spec_library(&key) {
            continue;
        }
        if seen.insert(key) {
            out.push(trimmed.to_string());
        }
    }
    out
}

fn load_project_presets() -> Vec<ProjectPresetItem> {
    let Ok(path) = resolve_project_preset_path() else {
        return Vec::new();
    };
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(doc) = serde_yaml::from_str::<ProjectPresetDoc>(&raw) else {
        return Vec::new();
    };
    doc.presets
        .into_iter()
        .filter_map(|mut p| {
            p.libraries = filter_allowed_preset_libraries(&p.libraries);
            if p.libraries.is_empty() {
                None
            } else {
                Some(p)
            }
        })
        .collect()
}

fn apply_first_project_preset_to_create_modal(app: &mut UiApp) {
    let presets = load_project_presets();
    if presets.is_empty() {
        app.status_line = "project preset not found at assets/presets/project.yaml".to_string();
        return;
    }
    let selected = &presets[0];
    let spec = selected
        .libraries
        .iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    let Some(modal) = app.create_modal.as_mut() else {
        return;
    };
    modal.spec = spec;
    modal.spec_is_default = false;
    app.status_line = if selected.name.trim().is_empty() {
        "project preset loaded".to_string()
    } else {
        format!("project preset loaded: {}", selected.name.trim())
    };
}

fn default_detail_layout() -> DetailLayoutPreset {
    DetailLayoutPreset {
        preset: "code".to_string(),
        grid: DetailLayoutGrid {
            columns: 10,
            rows: 10,
        },
        panels: vec![
            DetailLayoutPanel {
                id: "project".to_string(),
                name: "Project".to_string(),
                panel_type: "info".to_string(),
                selected_view: "project_meta".to_string(),
                shortcut: "enter: move-detail".to_string(),
                cell_start: 1,
                cell_end: 27,
            },
            DetailLayoutPanel {
                id: "rule".to_string(),
                name: "Rule".to_string(),
                panel_type: "list".to_string(),
                selected_view: "rule_list".to_string(),
                shortcut: "enter: edit-rule".to_string(),
                cell_start: 31,
                cell_end: 64,
            },
            DetailLayoutPanel {
                id: "constraint".to_string(),
                name: "Constraint".to_string(),
                panel_type: "list".to_string(),
                selected_view: "constraint_list".to_string(),
                shortcut: "enter: edit-constraint".to_string(),
                cell_start: 35,
                cell_end: 67,
            },
            DetailLayoutPanel {
                id: "features".to_string(),
                name: "Features".to_string(),
                panel_type: "list".to_string(),
                selected_view: "feature_list".to_string(),
                shortcut: "enter: edit-feature".to_string(),
                cell_start: 71,
                cell_end: 97,
            },
            DetailLayoutPanel {
                id: "drafts".to_string(),
                name: "Drafts".to_string(),
                panel_type: "runtime".to_string(),
                selected_view: "parallel_status".to_string(),
                shortcut: "b: create_code_draft/enter-parallel".to_string(),
                cell_start: 8,
                cell_end: 100,
            },
        ],
    }
}

fn resolve_detail_layout_path(preset: &str) -> Result<PathBuf, String> {
    let root = binary_root();
    let file = format!("{}.yaml", preset);
    let candidates = [
        root.join("assets").join("layouts").join(&file),
        PathBuf::from("assets").join("layouts").join(&file),
        root.join("src").join("assets").join("layouts").join(&file),
        PathBuf::from("src").join("assets").join("layouts").join(&file),
    ];
    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "detail layout not found: {} (binary root: {})",
        file,
        root.display()
    ))
}

fn compile_detail_layout(preset: &str, doc: DetailLayoutDoc) -> Result<DetailLayoutPreset, String> {
    if doc.grid.columns == 0 || doc.grid.rows == 0 {
        return Err("detail layout grid columns/rows must be >= 1".to_string());
    }
    let max_index = (doc.grid.columns as u32) * (doc.grid.rows as u32);
    let mut panels = Vec::new();
    for panel in doc.panels {
        if panel.cell_start == 0 || panel.cell_end == 0 {
            return Err(format!("detail layout panel `{}` cell index must be >= 1", panel.id));
        }
        if panel.cell_start as u32 > max_index || panel.cell_end as u32 > max_index {
            return Err(format!(
                "detail layout panel `{}` cell index out of range (max={})",
                panel.id, max_index
            ));
        }
        if panel.cell_start > panel.cell_end {
            return Err(format!(
                "detail layout panel `{}` must satisfy cell_start <= cell_end",
                panel.id
            ));
        }
        panels.push(DetailLayoutPanel {
            id: panel.id,
            name: panel.name,
            panel_type: panel.panel_type,
            selected_view: panel.selected_view,
            shortcut: panel.shortcut,
            cell_start: panel.cell_start,
            cell_end: panel.cell_end,
        });
    }
    Ok(DetailLayoutPreset {
        preset: preset.to_string(),
        grid: DetailLayoutGrid {
            columns: doc.grid.columns,
            rows: doc.grid.rows,
        },
        panels,
    })
}

fn layout_load(preset: &str) -> DetailLayoutPreset {
    let Ok(path) = resolve_detail_layout_path(preset) else {
        return default_detail_layout();
    };
    let Ok(raw) = fs::read_to_string(path) else {
        return default_detail_layout();
    };
    let Ok(doc) = serde_yaml::from_str::<DetailLayoutDoc>(&raw) else {
        return default_detail_layout();
    };
    compile_detail_layout(preset, doc).unwrap_or_else(|_| default_detail_layout())
}

fn layout_cell_to_row_col(cell: u16, cols: u16) -> (u16, u16) {
    let idx = cell.saturating_sub(1);
    let row = idx / cols;
    let col = idx % cols;
    (row, col)
}

fn layout_rect_from_cells(
    area: Rect,
    cols: u16,
    rows: u16,
    cell_start: u16,
    cell_end: u16,
) -> Rect {
    if area.width == 0 || area.height == 0 || cols == 0 || rows == 0 {
        return area;
    }
    let (start_row, start_col) = layout_cell_to_row_col(cell_start, cols);
    let (end_row, end_col) = layout_cell_to_row_col(cell_end, cols);
    let left_col = start_col.min(end_col) as u32;
    let right_col = start_col.max(end_col) as u32 + 1;
    let top_row = start_row.min(end_row) as u32;
    let bottom_row = start_row.max(end_row) as u32 + 1;

    let width = area.width as u32;
    let height = area.height as u32;
    let cols_u32 = cols as u32;
    let rows_u32 = rows as u32;

    let x0 = (width * left_col) / cols_u32;
    let x1 = (width * right_col) / cols_u32;
    let y0 = (height * top_row) / rows_u32;
    let y1 = (height * bottom_row) / rows_u32;

    let rect_x = area.x.saturating_add(x0 as u16);
    let rect_y = area.y.saturating_add(y0 as u16);
    let rect_w = ((x1.saturating_sub(x0)) as u16).max(1);
    let rect_h = ((y1.saturating_sub(y0)) as u16).max(1);

    Rect {
        x: rect_x,
        y: rect_y,
        width: rect_w.min(area.width.saturating_sub(rect_x.saturating_sub(area.x))),
        height: rect_h.min(area.height.saturating_sub(rect_y.saturating_sub(area.y))),
    }
}

fn layout_panel<'a>(layout: &'a DetailLayoutPreset, id: &str) -> Option<&'a DetailLayoutPanel> {
    layout.panels.iter().find(|panel| panel.id == id)
}

fn layout_panel_rect(layout: &DetailLayoutPreset, area: Rect, id: &str) -> Option<Rect> {
    let panel = layout_panel(layout, id)?;
    Some(layout_rect_from_cells(
        area,
        layout.grid.columns,
        layout.grid.rows,
        panel.cell_start,
        panel.cell_end,
    ))
}

fn layout_panel_name(layout: &DetailLayoutPreset, id: &str, fallback: &str) -> String {
    layout_panel(layout, id)
        .map(|p| {
            let _ = (&p.panel_type, &p.selected_view, &p.shortcut);
            p.name.clone()
        })
        .unwrap_or_else(|| fallback.to_string())
}

fn selected_pane_shortcut(layout: &DetailLayoutPreset, tab_index: usize, pane_focus: usize) -> String {
    if tab_index != 1 {
        return String::new();
    }
    let panel_id = match pane_focus {
        0 => "project",
        1 => "rule",
        2 => "constraint",
        3 => "features",
        4 | 5 => "drafts",
        _ => return String::new(),
    };
    layout_panel(layout, panel_id)
        .map(|p| p.shortcut.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_default()
}

fn run_create_project_in_project_dir(
    project_dir: &Path,
    name: &str,
    description: &str,
) -> Result<String, String> {
    let abs_path = project_dir
        .canonicalize()
        .unwrap_or_else(|_| project_dir.to_path_buf());
    let exe = env::current_exe().map_err(|e| format!("failed to resolve current exe: {}", e))?;
    let output = Command::new(exe)
        .current_dir(project_dir)
        .arg("init_code_project")
        .arg("-n")
        .arg(name)
        .arg("-p")
        .arg(abs_path.display().to_string())
        .arg("-d")
        .arg(description)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("failed to run init_code_project: {}", e))?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Err(format!(
            "init_code_project failed (code={:?}) stderr=`{}` stdout=`{}`",
            output.status.code(),
            stderr,
            stdout
        ))
    }
}

fn has_any_project_md(project_root: &Path) -> bool {
    project_md_shadow_path(project_root).exists()
}

fn sync_project_md_files(project_root: &Path) -> Result<bool, String> {
    let shadow = project_md_shadow_path(project_root);
    if !shadow.exists() {
        return Ok(false);
    }
    let body = fs::read_to_string(&shadow)
        .map_err(|e| format!("failed to read {}: {}", shadow.display(), e))?;
    write_project_md_with_sync(project_root, &body)?;
    Ok(true)
}

fn ui_model_bin() -> String {
    let root = crate::source_root();
    let candidates = [
        root.join("configs.yaml"),
        root.join("config.yaml"),
        root.join("assets").join("config").join("config.yaml"),
        root.join("src").join("assets").join("config").join("config.yaml"),
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

fn extract_markdown_block(raw: &str) -> Option<String> {
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

fn validate_project_md_format(project_md: &str) -> Result<(), String> {
    let required_headers = ["# info", "# features", "# rules", "# constraints", "# domains"];
    for header in required_headers {
        if !project_md.lines().any(|line| line.trim().eq_ignore_ascii_case(header)) {
            return Err(format!("missing header `{}`", header));
        }
    }
    for banned in ["- 제안 도메인:", "- 근거:", "- 책임:"] {
        if project_md.contains(banned) {
            return Err(format!("banned domains summary style `{}`", banned));
        }
    }
    let domain_names = crate::extract_project_md_domain_names(project_md);
    if domain_names.is_empty() {
        return Err("missing `# domains -> ## <name>` block".to_string());
    }
    for required in ["### states", "### action", "### rules"] {
        if !project_md
            .lines()
            .any(|line| line.trim().eq_ignore_ascii_case(required))
        {
            return Err(format!("missing domain subsection `{}`", required));
        }
    }
    Ok(())
}

fn apply_draft_create_via_cli(
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
        .arg("create_code_draft")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("failed to run create_code_draft: {}", e))?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        app.status_line = if stdout.is_empty() {
            "draft create requested (create_code_draft)".to_string()
        } else {
            stdout
        };
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!(
            "create_code_draft failed (code={:?}) {}",
            output.status.code(),
            stderr
        ))
    }
}

fn apply_build_parallel_via_cli(
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
        .arg("impl_code_draft")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("failed to run impl_code_draft: {}", e))?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        app.status_line = if stdout.is_empty() {
            "impl_code_draft done".to_string()
        } else {
            stdout
        };
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!(
            "impl_code_draft failed (code={:?}) {}",
            output.status.code(),
            stderr
        ))
    }
}

fn start_build_parallel_via_cli_async(
    projects: &[ProjectRecord],
    app: &mut UiApp,
    project_index: usize,
) -> Result<(), String> {
    let Some(project) = projects.get(project_index) else {
        return Err("selected project index out of range".to_string());
    };
    if app.parallel_running {
        return Ok(());
    }
    let exe = env::current_exe().map_err(|e| format!("failed to resolve current exe: {}", e))?;
    let project_dir = project.path.clone();
    let (tx, rx) = mpsc::channel::<Result<String, String>>();
    thread::spawn(move || {
        let output = Command::new(exe)
            .current_dir(&project_dir)
            .arg("impl_code_draft")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();
        let result = match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if stdout.is_empty() {
                    Ok("impl_code_draft done".to_string())
                } else {
                    Ok(stdout)
                }
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                Err(format!(
                    "impl_code_draft failed (code={:?}) {}",
                    out.status.code(),
                    stderr
                ))
            }
            Err(e) => Err(format!("failed to run impl_code_draft: {}", e)),
        };
        let _ = tx.send(result);
    });
    let planned = collect_planned_drafts_from_project(project);
    app.parallel_statuses = if planned.is_empty() {
        vec![("buildParallelCode".to_string(), TaskRuntimeState::Active)]
    } else {
        planned
            .into_iter()
            .map(|name| (name, TaskRuntimeState::Inactive))
            .collect()
    };
    if let Some((_, state)) = app.parallel_statuses.first_mut() {
        *state = TaskRuntimeState::Active;
    }
    app.parallel_build_rx = Some(rx);
    app.parallel_running = true;
    app.last_tick = Instant::now();
    app.status_line = "parallel build started".to_string();
    Ok(())
}

fn has_planned_task_file(project: &ProjectRecord, feature_name: &str) -> bool {
    let feature_dir = Path::new(&project.path)
        .join(".project")
        .join("feature")
        .join(feature_name);
    [
        feature_dir.join("drafts.yaml"),
        feature_dir.join("tasks.yaml"),
        feature_dir.join("drafts.yaml"),
        feature_dir.join("drafts.yaml"),
    ]
    .iter()
    .any(|p| p.exists())
}

fn all_planned_task_files_exist(project: &ProjectRecord, planned: &[String]) -> bool {
    !planned.is_empty()
        && planned
            .iter()
            .all(|feature_name| has_planned_task_file(project, feature_name))
}

fn open_draft_bulk_add_modal(app: &mut UiApp, project_index: usize) {
    app.draft_bulk_add_modal = Some(DraftBulkAddModal {
        project_index,
        input: String::new(),
        input_focus: true,
        confirm_selected: true,
    });
    app.status_line = "draft bulk add modal opened".to_string();
}

fn open_draft_create_confirm(app: &mut UiApp, project_index: usize) {
    app.draft_create_confirm = Some(DraftCreateConfirm {
        project_index,
        confirm_selected: true,
    });
    app.status_line = "draft create confirm opened".to_string();
}

fn split_draft_bulk_add_requests(raw_input: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut current_name: Option<String> = None;
    let mut chunk: Vec<String> = Vec::new();
    for line in raw_input.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_prefix('#') {
            if !chunk.is_empty() {
                let request = chunk.join("\n").trim().to_string();
                if !request.is_empty() {
                    out.push((
                        current_name
                            .clone()
                            .filter(|v| !v.is_empty())
                            .unwrap_or_else(|| "new_feature".to_string()),
                        request,
                    ));
                }
                chunk.clear();
            }
            current_name = Some(name.trim().to_string());
        }
        chunk.push(line.to_string());
    }
    if !chunk.is_empty() {
        let request = chunk.join("\n").trim().to_string();
        if !request.is_empty() {
            out.push((
                current_name
                    .filter(|v| !v.is_empty())
                    .unwrap_or_else(|| "new_feature".to_string()),
                request,
            ));
        }
    }
    if out.is_empty() && !raw_input.trim().is_empty() {
        out.push(("new_feature".to_string(), raw_input.trim().to_string()));
    }
    out
}

fn apply_draft_bulk_add_via_cli(
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
    let requests = split_draft_bulk_add_requests(raw_input);
    if requests.is_empty() {
        return Err("draft add requires parseable input".to_string());
    }
    let mut applied = 0usize;
    for (feature_name, request) in requests {
        let output = Command::new(&exe)
            .current_dir(&project.path)
            .arg("add_code_draft")
            .arg("-m")
            .arg(format!("{}: {}", feature_name, request))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("failed to run add_code_draft: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(format!(
                "add_code_draft failed (code={:?}) {}",
                output.status.code(),
                stderr
            ));
        }
        applied += 1;
    }
    app.status_line = format!("draft add requested via add_code_draft ({})", applied);
    Ok(())
}

fn render_draft_bulk_add_modal(
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
        Style::default()
            .fg(Color::Rgb(255, 165, 0))
            .add_modifier(Modifier::BOLD)
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
    component::render_confirm_buttons_bottom_right(
        f,
        inner,
        "Confirm",
        "Cancel",
        modal.confirm_selected,
    );
    if modal.input_focus {
        Some(cursor_in_input(rows[0], &modal.input))
    } else {
        None
    }
}

fn build_ai_seed_prompt(project: &ProjectRecord, project_md: &str) -> String {
    format!(
        "System:\n`$plan-project-code` 스킬 워크플로우로 진행합니다.\n현재 project.md에는 name/description이 이미 포함되어 있습니다.\n\n\
Context:\nname={}\ndescription={}\npath={}\n\n\
Current project.md:\n{}\n\n\
위 컨텍스트를 내부 기준으로만 저장하고, 출력은 반드시 `READY` 한 단어만 반환하세요.",
        project.name, project.description, project.path, project_md
    )
}

fn new_ai_chat_modal_template(
    project: &ProjectRecord,
    project_index: usize,
    mode: AiChatMode,
    model_bin: String,
) -> AiChatModal {
    AiChatModal {
        project_index,
        project_path: project.path.clone(),
        project_name: project.name.clone(),
        project_description: project.description.clone(),
        initial_spec: String::new(),
        mode,
        model_bin,
        warmup_inflight: false,
        input: String::new(),
        input_enter_streak: 0,
        focus: AiDetailFocus::Input,
        input_active: false,
        allow_full_md_response: false,
        add_plan_apply_requested: false,
        history: Vec::new(),
        streaming: false,
        streaming_buffer: String::new(),
        stream_rx: None,
        stream_cancel: None,
    }
}

fn start_ai_chat_warmup(modal: &mut AiChatModal, seed_prompt: String) {
    append_project_chat_log(&modal.project_path, "LLM_PROMPT", &seed_prompt);
    let (seed_rx, seed_cancel) = spawn_ai_stream(&modal.model_bin, seed_prompt);
    modal.warmup_inflight = true;
    modal.streaming = true;
    modal.stream_rx = Some(seed_rx);
    modal.stream_cancel = Some(seed_cancel);
}

fn start_ai_chat_onboarding(modal: &mut AiChatModal, initial_spec: Option<&str>) {
    let normalized_initial_spec = initial_spec
        .map(str::trim)
        .filter(|v| !v.is_empty() && !v.eq_ignore_ascii_case("auto"))
        .unwrap_or_default()
        .to_string();
    modal.initial_spec = normalized_initial_spec.clone();
    let initial_spec_text = if normalized_initial_spec.is_empty() {
        String::new()
    } else {
        format!("- initial_spec: {}\n", normalized_initial_spec)
    };
    let prompt = format!(
        "너는 새 프로젝트 온보딩 도우미다.\n\
프로젝트:\n- name: {}\n- description: {}\n- path: {}\n\n\
{}\n\
지금부터 사용자에게 필요한 정보(spec, goal, rule, features)를 단계적으로 질문해 수집해.\n\
수집된 features를 바탕으로 `$build_domain` 스킬 기준의 domain 초안을 제시하고, 확정 또는 추가 요청을 받는다.\n\
spec과 domain이 모두 확정되면 `.project/project.md` 전체를 출력하지 말고 `둘다 완료되었습니다. 다음으로 진행하세요.` 한 줄만 출력한다.\n\
최종 확정 시 `## plan` 최소 5개 기능은 내부적으로 반영된 것으로 간주하고, 응답에는 완료 메시지만 출력한다.\n\
주의:\n\
- 템플릿 `/home/tree/ai/skills/plan-project-code/references/project.md` 형식 준수\n\
- 도메인 확정 전에는 전체 문서를 출력하지 말고 질문/확인만 진행\n\
- 지금은 첫 질문부터 시작",
        modal.project_name, modal.project_description, modal.project_path, initial_spec_text
    );
    append_project_chat_log(&modal.project_path, "LLM_PROMPT", &prompt);
    let (rx, cancel) = spawn_ai_stream(&modal.model_bin, prompt);
    modal.warmup_inflight = false;
    modal.streaming = true;
    modal.streaming_buffer.clear();
    modal.stream_rx = Some(rx);
    modal.stream_cancel = Some(cancel);
}

fn open_ai_chat_modal(app: &mut UiApp, projects: &[ProjectRecord], project_index: usize) {
    let Some(project) = projects.get(project_index) else {
        app.status_line = "no project selected".to_string();
        return;
    };
    let project_md = read_project_md(project).unwrap_or_default();
    let seed_prompt = build_ai_seed_prompt(project, &project_md);
    let model_bin = ui_model_bin();
    let mut modal =
        new_ai_chat_modal_template(project, project_index, AiChatMode::DetailProject, model_bin);
    modal.input_active = true;
    start_ai_chat_warmup(&mut modal, seed_prompt);
    app.ai_chat_modal = Some(modal);
    app.status_line = "ai detail warmup started".to_string();
}

fn open_ai_onboarding_modal(
    app: &mut UiApp,
    projects: &[ProjectRecord],
    project_index: usize,
    initial_spec: Option<&str>,
) {
    let Some(project) = projects.get(project_index) else {
        app.status_line = "no project selected".to_string();
        return;
    };
    let model_bin = ui_model_bin();
    let mut modal =
        new_ai_chat_modal_template(project, project_index, AiChatMode::DetailProject, model_bin);
    modal.input_active = true;
    start_ai_chat_onboarding(&mut modal, initial_spec);
    app.ai_chat_modal = Some(modal);
    app.status_line = "ai onboarding started".to_string();
}

fn open_add_plan_ai_chat_modal(
    app: &mut UiApp,
    projects: &[ProjectRecord],
    project_index: usize,
) {
    let Some(project) = projects.get(project_index) else {
        app.status_line = "no project selected".to_string();
        return;
    };
    let model_bin = ui_model_bin();
    let mut modal =
        new_ai_chat_modal_template(project, project_index, AiChatMode::AddPlan, model_bin);
    modal.input_active = true;
    let intro = "add_code_plan 모드입니다. 원하는 기능 방향을 말하면 질문으로 범위를 좁힌 뒤 적용 가능한 features/planned를 제안합니다.\n적용하려면 `적용`이라고 입력하세요.";
    modal.history.push(format!("AI:\n{}", intro));
    append_project_chat_log(&modal.project_path, "AI_RESPONSE", intro);
    app.ai_chat_modal = Some(modal);
    app.status_line = "ai add_code_plan modal opened".to_string();
}

fn open_bootstrap_confirm(app: &mut UiApp, projects: &[ProjectRecord], project_index: usize) {
    open_bootstrap_confirm_with_spec_hint(app, projects, project_index, None);
}

fn open_bootstrap_confirm_with_spec_hint(
    app: &mut UiApp,
    projects: &[ProjectRecord],
    project_index: usize,
    spec_hint: Option<&str>,
) {
    let Some(project) = projects.get(project_index) else {
        app.status_line = "no project selected".to_string();
        return;
    };
    let mut spec = read_project_md(project)
        .map(|md| parse_project_md(&md).spec)
        .unwrap_or_default();
    if spec.trim().is_empty() {
        if let Some(hint) = spec_hint {
            let normalized = hint.trim();
            if !normalized.is_empty() && !normalized.eq_ignore_ascii_case("auto") {
                spec = normalized.to_string();
            }
        }
    }
    app.bootstrap_confirm = Some(BootstrapConfirm {
        project_index,
        spec,
        confirm_selected: true,
    });
    app.status_line = "project bootstrap 실행 여부를 선택하세요".to_string();
}

fn extract_yaml_codeblock(raw: &str) -> Option<String> {
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

fn load_bootstrap_rule_for_spec(spec: &str) -> Option<BootstrapRule> {
    let root = binary_root();
    let candidates = [
        root.join("configs").join("bootstrap.md"),
        PathBuf::from("configs").join("bootstrap.md"),
    ];
    let spec_lc = spec.to_ascii_lowercase();
    for path in candidates {
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let Some(yaml_block) = extract_yaml_codeblock(&raw) else {
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

fn resolve_bootstrap_prompt_path() -> Result<PathBuf, String> {
    let root = crate::source_root();
    let candidates = [
        root.join("assets").join("code").join("prompts").join("bootstrap.txt"),
        PathBuf::from("assets").join("code").join("prompts").join("bootstrap.txt"),
        root.join("src")
            .join("assets")
            .join("code")
            .join("prompts")
            .join("bootstrap.txt"),
        PathBuf::from("src")
            .join("assets")
            .join("code")
            .join("prompts")
            .join("bootstrap.txt"),
    ];
    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "bootstrap prompt not found (source root: {})",
        root.display()
    ))
}

fn extract_spec_from_project_md(project_md: &str) -> Option<String> {
    for line in project_md.lines() {
        let trimmed = line.trim();
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        if key.trim().eq_ignore_ascii_case("spec") {
            let spec = value.trim().trim_matches('`').to_string();
            if !spec.is_empty() {
                return Some(spec);
            }
        }
    }
    None
}

fn extract_bootstrap_spec_from_project_md(project_root: &Path) -> Result<String, String> {
    let path = project_root.join(".project").join("project.md");
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    extract_spec_from_project_md(&raw)
        .ok_or_else(|| format!("bootstrap spec not found in {}", path.display()))
}

pub(crate) fn apply_bootstrap_by_spec(
    project_root: &Path,
    project_name: &str,
) -> Result<String, String> {
    if !is_bootstrap_target_empty(project_root)? {
        return Ok("bootstrap skipped: target folder is not empty".to_string());
    }
    let spec = extract_bootstrap_spec_from_project_md(project_root)?;
    let template_path = resolve_bootstrap_prompt_path()?;
    let template = fs::read_to_string(&template_path)
        .map_err(|e| format!("failed to read {}: {}", template_path.display(), e))?;
    let prompt = template
        .replace("{{project_name}}", project_name)
        .replace("{{project_root}}", &project_root.display().to_string())
        .replace("{{spec}}", &spec);
    let timeout_sec = crate::load_app_config()
        .as_ref()
        .map_or(300, crate::config::AppConfig::default_timeout_sec)
        .max(30);
    let raw =
        crate::run_codex_exec_capture_in_dir_with_timeout(project_root, &prompt, timeout_sec)?;
    let first_line = raw.lines().next().unwrap_or("").trim();
    if first_line.is_empty() {
        Ok("bootstrap completed via llm".to_string())
    } else {
        Ok(format!("bootstrap completed via llm: {}", first_line))
    }
}

fn is_bootstrap_target_empty(project_root: &Path) -> Result<bool, String> {
    let entries = fs::read_dir(project_root)
        .map_err(|e| format!("failed to read {}: {}", project_root.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read dir entry: {}", e))?;
        let name = entry.file_name().to_string_lossy().to_string();
        // Internal metadata/docs folders should not block initial bootstrap.
        if name == ".project" || name == "project" || name == ".agents" {
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

fn run_bootstrap_llm_prepare(
    project: &ProjectRecord,
    project_root: &Path,
    preset: &str,
    ) -> Result<(), String> {
    let spec = extract_bootstrap_spec_from_project_md(project_root)?;
    let model_bin = ui_model_bin();
    let template_path = resolve_bootstrap_prompt_path()?;
    let template = fs::read_to_string(&template_path)
        .map_err(|e| format!("failed to read {}: {}", template_path.display(), e))?;
    let prompt = template
        .replace("{{project_name}}", &project.name)
        .replace("{{project_root}}", &project_root.display().to_string())
        .replace("{{spec}}", &spec)
        .replace("{{preset}}", preset);
    append_project_chat_log(&project.path, "LLM_PROMPT", &prompt);
    let mut cmd = Command::new(&model_bin);
    cmd.arg("exec");
    if model_bin.eq_ignore_ascii_case("codex") {
        cmd.arg("--dangerously-bypass-approvals-and-sandbox");
    }
    let output = cmd
        .arg(prompt)
        .current_dir(project_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("failed to execute bootstrap llm prepare: {}", e))?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        append_project_chat_log(&project.path, "LLM_RESPONSE", &stdout);
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        append_project_chat_log(&project.path, "LLM_ERROR", &stderr);
        Err(format!("bootstrap llm prepare failed: {}", stderr))
    }
}

fn apply_bootstrap(
    projects: &[ProjectRecord],
    app: &mut UiApp,
    confirm: &BootstrapConfirm,
) -> Result<(), String> {
    let Some(project) = projects.get(confirm.project_index) else {
        return Err("selected project index out of range".to_string());
    };
    let finalize_ui = |app: &mut UiApp| {
        cancel_ai_stream(app);
        app.ai_chat_modal = None;
        app.tab_index = 1;
        app.pane_focus = 0;
        app.menu_active = true;
    };
    let project_root = Path::new(&project.path);
    let preset = if let Some(rule) = load_bootstrap_rule_for_spec(&confirm.spec) {
        match rule.template.trim().to_ascii_lowercase().as_str() {
            "react-native" | "react_native" | "expo" => Some("react-native"),
            "node-react" | "node" | "react" => Some("node-react"),
            "rust" => Some("rust"),
            _ => None,
        }
    } else {
        let spec_lc = confirm.spec.to_ascii_lowercase();
        if spec_lc.contains("react native") || spec_lc.contains("react-native") || spec_lc.contains("expo") {
            Some("react-native")
        } else if spec_lc.contains("react")
            || spec_lc.contains("next")
            || spec_lc.contains("node")
            || spec_lc.contains("typescript")
            || spec_lc.contains("javascript")
        {
            Some("node-react")
        } else if spec_lc.contains("rust") {
            Some("rust")
        } else {
            None
        }
    };
    if let Some(preset) = preset {
        let _ = run_bootstrap_llm_prepare(project, project_root, preset);
    }
    let status = apply_bootstrap_by_spec(project_root, &project.name)?;
    app.status_line = status;
    finalize_ui(app);
    Ok(())
}

fn build_ai_detail_chat_prompt(modal: &AiChatModal, user_message: &str) -> String {
    let full_md_requested = is_full_project_md_request(user_message);
    let (spec_ready, domain_ready, feature_count) =
        collect_onboarding_signals(modal, user_message);
    let completion_ready = spec_ready && domain_ready && feature_count >= 3;
    format!(
        "당신은 `$plan-project-code` 스킬을 따라 project.md를 완성하는 도우미다.\n\
`/home/tree/ai/skills/plan-project-code/references/project.md` 템플릿을 대상 폴더의 `.project/project.md`에 먼저 복사한 뒤, 주석/예시를 지우고 값만 수정한다.\n\
현재 project의 확정 정보(name/description)는 유지해야 한다.\n\
- name: {}\n- description: {}\n- initial_spec: {}\n\n\
초기 컨텍스트는 이미 전달되었다. 아래 대화만 기반으로 답변하라.\n\n\
대화 이력:\n{}\n\n\
사용자 최신 입력:\n{}\n\n\
전체 project.md 출력 명시 요청 여부: {}\n\n\
현재 수집 상태:\n- spec: {}\n- domain: {}\n- features_count: {}\n- completion_ready: {}\n\n\
응답 규칙:\n1) 규칙은 `$plan-project-code`, `$build_domain` 스킬을 사용한다.\n\
2) `둘다 완료되었습니다. 다음으로 진행하세요.`는 completion_ready=true 인 경우에만 출력한다.\n\
3) completion_ready=false 이면 누락 항목(spec/domain/features)을 채우는 질문만 1~2문장으로 출력한다.\n\
3) 기본 응답은 짧게 작성하고, 코드펜스/장문 덤프/다음 단계 전환 제안은 금지한다.",
        modal.project_name,
        modal.project_description,
        if modal.initial_spec.is_empty() {
            "(empty)"
        } else {
            modal.initial_spec.as_str()
        },
        modal.history.join("\n\n"),
        user_message,
        if full_md_requested { "yes" } else { "no" }
        ,
        if spec_ready { "ready" } else { "missing" },
        if domain_ready { "ready" } else { "missing" },
        feature_count,
        if completion_ready { "true" } else { "false" }
    )
}

fn strip_next_step_guidance(raw: &str) -> String {
    let mut kept = Vec::new();
    for line in raw.lines() {
        let lower = line.to_ascii_lowercase();
        let blocked = lower.contains("plan-drafts-code")
            || line.contains("다음 단계")
            || line.contains("전환할까요")
            || line.contains("바로 전환")
            || line.contains("진행할까요");
        if !blocked {
            kept.push(line);
        }
    }
    let joined = kept.join("\n").trim().to_string();
    if joined.is_empty() {
        "project.md 보완 완료. 다음 단계는 직접 선택해 주세요.".to_string()
    } else {
        joined
    }
}

fn has_project_md_complete_signal(raw: &str) -> bool {
    let compact = raw.replace(' ', "").to_ascii_lowercase();
    compact.contains("project.md생성을완료하겠습니다")
        || compact.contains("projet.md생성을완료하겠습니다")
}

fn has_onboarding_done_signal(raw: &str) -> bool {
    let compact = raw.replace([' ', '\n', '\t'], "").to_ascii_lowercase();
    compact.contains("둘다완료되었습니다.다음으로진행하세요.")
        || compact.contains("spec과domain이모두확정되었습니다.다음으로진행하세요.")
}

fn collect_onboarding_signals(
    modal: &AiChatModal,
    latest_user_message: &str,
) -> (bool, bool, usize) {
    let mut user_texts = Vec::new();
    for entry in &modal.history {
        if let Some(rest) = entry.strip_prefix("You:\n") {
            user_texts.push(rest.to_string());
        }
    }
    if !latest_user_message.trim().is_empty() {
        user_texts.push(latest_user_message.trim().to_string());
    }
    let mut joined = user_texts.join("\n");
    if !modal.initial_spec.trim().is_empty() {
        if !joined.is_empty() {
            joined.push('\n');
        }
        joined.push_str(&modal.initial_spec);
    }
    let joined = joined.to_ascii_lowercase();
    let spec_ready = ["react", "three", "fiber", "zustand", "tauri", "rust", "typescript", "next"]
        .iter()
        .any(|k| joined.contains(k))
        || joined.contains("spec");
    let domain_ready = joined.contains("도메인") || joined.contains("domain");
    let mut feature_count = 0usize;
    for text in &user_texts {
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("- ") {
                feature_count += 1;
            }
        }
    }
    (spec_ready, domain_ready, feature_count)
}

fn build_ai_finalize_project_md_prompt(modal: &AiChatModal) -> String {
    let current_md_path = Path::new(&modal.project_path).join(".project").join("project.md");
    let current_md = fs::read_to_string(current_md_path).unwrap_or_default();
    format!(
        "너는 project.md 생성기다.\n\
대화에서 확정된 내용을 반영해 `.project/project.md` 전체 본문을 생성한다.\n\
반드시 `$plan-project-code`, `$build_domain` 스킬 규칙을 따른다.\n\
반드시 템플릿(`/home/tree/ai/skills/plan-project-code/references/project.md`) 구조를 유지한다.\n\
규칙:\n\
1) # info의 name/description은 아래 고정값 유지\n\
2) 대화에서 확정된 spec/goal/rule/features/domain을 반영\n\
3) `## plan` 최소 5개\n\
4) 설명문/코드펜스 금지, markdown 본문만 출력\n\n\
고정 정보:\n- name: {}\n- description: {}\n- path: {}\n\n\
초기 spec 힌트:\n{}\n\n\
현재 project.md:\n{}\n\n\
대화 이력:\n{}",
        modal.project_name,
        modal.project_description,
        modal.project_path,
        if modal.initial_spec.trim().is_empty() {
            "(empty)"
        } else {
            modal.initial_spec.trim()
        },
        current_md,
        modal.history.join("\n\n")
    )
}

fn finalize_project_md_from_chat(modal: &AiChatModal) -> Result<(), String> {
    let prompt = build_ai_finalize_project_md_prompt(modal);
    append_project_chat_log(&modal.project_path, "LLM_PROMPT", &prompt);
    let mut cmd = Command::new(&modal.model_bin);
    cmd.arg("exec");
    if modal.model_bin.eq_ignore_ascii_case("codex") {
        cmd.arg("--dangerously-bypass-approvals-and-sandbox");
    }
    let output = cmd
        .arg(prompt)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("failed to execute finalize project.md llm: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        append_project_chat_log(&modal.project_path, "LLM_ERROR", &stderr);
        return Err(format!(
            "finalize project.md llm failed (code={:?}) {}",
            output.status.code(),
            stderr
        ));
    }
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    append_project_chat_log(&modal.project_path, "LLM_RESPONSE_RAW", &raw);
    let md = extract_markdown_block(&raw)
        .ok_or_else(|| "finalize project.md: markdown body not found".to_string())?;
    validate_project_md_format(&md)?;
    let root = Path::new(&modal.project_path);
    write_project_md_with_sync(root, &md)?;
    let _ = crate::sync_project_tasks_list_from_project_md(root);
    Ok(())
}

fn project_md_shadow_path(project_root: &Path) -> PathBuf {
    project_root.join(".project").join("project.md")
}

fn write_project_md_with_sync(project_root: &Path, body: &str) -> Result<(), String> {
    let shadow = project_md_shadow_path(project_root);
    if let Some(parent) = shadow.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
    }
    fs::write(&shadow, body).map_err(|e| format!("failed to write {}: {}", shadow.display(), e))
}

fn normalize_feature_key(value: &str) -> String {
    let mut out = String::new();
    let mut prev_is_alnum = false;
    for ch in value.chars() {
        if ch.is_ascii_uppercase() {
            if prev_is_alnum && !out.ends_with('_') {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            prev_is_alnum = true;
        } else if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            if out.is_empty() {
                if ch.is_ascii_lowercase() {
                    out.push(ch);
                    prev_is_alnum = true;
                }
            } else {
                out.push(ch.to_ascii_lowercase());
                prev_is_alnum = true;
            }
        } else {
            if !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }
            prev_is_alnum = false;
        }
    }
    out.trim_matches('_').to_string()
}

fn is_valid_snake_feature_key(value: &str) -> bool {
    if value.len() < 3 {
        return false;
    }
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return false;
    }
    value.contains('_') && !value.contains("__") && !value.ends_with('_')
}

fn is_add_plan_apply_request(user_message: &str) -> bool {
    let lower = user_message.to_ascii_lowercase();
    ["적용", "반영", "저장", "apply", "update", "commit"]
        .iter()
        .any(|kw| user_message.contains(kw) || lower.contains(kw))
}

fn build_ai_add_plan_prompt(modal: &AiChatModal, user_message: &str) -> String {
    let project_md_path = Path::new(&modal.project_path).join(".project").join("project.md");
    let project_md = fs::read_to_string(project_md_path).unwrap_or_default();
    let base = Path::new(&modal.project_path).join(".project");
    let tasks_doc = load_tasks_list_doc(&base).unwrap_or_default();
    let features = if tasks_doc.features.is_empty() {
        "- (none)".to_string()
    } else {
        tasks_doc
            .features
            .iter()
            .map(|v| format!("- {}", v))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let planned = if tasks_doc.planned.is_empty() {
        "- (none)".to_string()
    } else {
        tasks_doc
            .planned
            .iter()
            .map(|v| format!("- {}", v))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let apply_requested = is_add_plan_apply_request(user_message);
    format!(
        "너는 add_code_plan 전용 기획 도우미다.\n\
목표: project.md의 `## features`에 추가할 후보를 정리한다.\n\
항상 한국어로 짧게 답해라.\n\n\
현재 project.md:\n{}\n\n\
현재 drafts_list.yaml.features:\n{}\n\n\
현재 drafts_list.yaml.planned:\n{}\n\n\
대화 이력:\n{}\n\n\
사용자 입력:\n{}\n\n\
이번 입력에서 즉시 적용 요청 여부: {}\n\n\
응답 규칙:\n\
1) 정보가 부족하면 1~3개의 구체 질문만 출력한다.\n\
2) `즉시 적용 요청 여부=yes`가 아니면, 추천 기능 목록(최대 5개)과 확인 질문만 출력하고 YAML은 출력하지 않는다.\n\
3) `즉시 적용 요청 여부=yes`일 때만 짧은 설명 뒤에 아래 YAML codeblock을 반드시 포함한다.\n\
```yaml\n\
add_plan_update:\n\
  features:\n\
    - <verb_noun snake_case key>\n\
```\n\
4) key는 영문 소문자 `동사_명사` snake_case만 허용(예: `render_cube`), 중복 금지.\n\
5) `planned`는 시스템 함수가 자동 동기화하므로 YAML에 포함하지 않는다.\n\
6) `add_plan_update` 블록 외 추가 YAML 금지.",
        project_md,
        features,
        planned,
        modal.history.join("\n\n"),
        user_message,
        if apply_requested { "yes" } else { "no" }
    )
}

fn build_ai_chat_prompt(modal: &AiChatModal, user_message: &str) -> String {
    match modal.mode {
        AiChatMode::DetailProject => build_ai_detail_chat_prompt(modal, user_message),
        AiChatMode::AddPlan => build_ai_add_plan_prompt(modal, user_message),
    }
}

#[derive(Debug, Deserialize)]
struct AddPlanUpdateDoc {
    add_plan_update: AddPlanUpdateBody,
}

#[derive(Debug, Default, Deserialize)]
struct AddPlanUpdateBody {
    #[serde(default)]
    features: Vec<String>,
}

fn strip_keyword_suffix(token: &str) -> String {
    let lower = token.to_ascii_lowercase();
    for suffix in ["planned", "features"] {
        if lower.ends_with(suffix) && token.len() > suffix.len() {
            return token[..token.len() - suffix.len()].to_string();
        }
    }
    token.to_string()
}

fn extract_key_tokens_from_text(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut token = String::new();
    let push_token = |buf: &mut String, out: &mut Vec<String>| {
        if buf.is_empty() {
            return;
        }
        let cleaned = strip_keyword_suffix(buf.trim());
        let lower = cleaned.to_ascii_lowercase();
        let banned = [
            "features",
            "planned",
            "add",
            "plan",
            "update",
            "yaml",
            "codeblock",
            "key",
            "keys",
            "snake",
            "snakecase",
            "verb",
            "noun",
        ];
        if !cleaned.is_empty()
            && !cleaned.as_bytes()[0].is_ascii_digit()
            && !banned.iter().any(|v| *v == lower)
            && cleaned.chars().any(|c| c.is_ascii_alphabetic())
        {
            let key = normalize_feature_key(&cleaned);
            if is_valid_snake_feature_key(&key) && !out.iter().any(|v| v == &key) {
                out.push(key);
            }
        }
        buf.clear();
    };
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            token.push(ch);
        } else {
            push_token(&mut token, &mut out);
        }
    }
    push_token(&mut token, &mut out);
    out
}

fn extract_add_plan_update_from_raw_response(raw_response: &str) -> Option<AddPlanUpdateBody> {
    let raw = raw_response.replace('`', " ").replace(['│', '|'], " ");
    let lower = raw.to_ascii_lowercase();
    let features_idx = lower.find("features:");
    if features_idx.is_none() {
        return None;
    }
    let mut body = AddPlanUpdateBody::default();
    if let Some(fi) = features_idx {
        if fi < raw.len() {
            body.features = extract_key_tokens_from_text(&raw[fi..]);
        }
    }
    if body.features.is_empty() {
        None
    } else {
        Some(body)
    }
}

fn append_project_md_features_items(project_path: &Path, items: &[String]) -> Result<Vec<String>, String> {
    if items.is_empty() {
        return Ok(Vec::new());
    }
    let path = project_path.join(".project").join("project.md");
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let mut lines: Vec<String> = raw.lines().map(|v| v.to_string()).collect();
    let header = "# features";
    let header_idx = lines
        .iter()
        .position(|line| line.trim().eq_ignore_ascii_case(header));
    let idx = if let Some(i) = header_idx {
        i
    } else {
        lines.push(String::new());
        lines.push("# features".to_string());
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
    let mut existing = Vec::new();
    for line in &lines[(idx + 1)..end] {
        let trimmed = line.trim();
        if !trimmed.starts_with("- ") {
            continue;
        }
        let body = trimmed.trim_start_matches("- ").trim();
        if body.is_empty() {
            continue;
        }
        let key = body.split(':').next().unwrap_or(body).trim();
        let normalized = normalize_feature_key(key);
        if is_valid_snake_feature_key(&normalized) && !existing.iter().any(|v| v == &normalized) {
            existing.push(normalized);
        }
    }
    let mut added = Vec::new();
    for item in items {
        let key = normalize_feature_key(item);
        if !is_valid_snake_feature_key(&key) || existing.iter().any(|v| v == &key) {
            continue;
        }
        existing.push(key.clone());
        added.push(key);
    }
    if existing.is_empty() {
        existing.push("plan_feature".to_string());
    }
    let replacement: Vec<String> = existing.iter().map(|v| format!("- {}", v)).collect();
    lines.splice((idx + 1)..end, replacement);
    fs::write(&path, lines.join("\n") + "\n")
        .map_err(|e| format!("failed to write {}: {}", path.display(), e))?;
    Ok(added)
}

fn append_planned_from_add_plan_items(
    project_path: &Path,
    feature_keys: &[String],
) -> Result<usize, String> {
    if feature_keys.is_empty() {
        return Ok(0);
    }
    let base = project_path.join(".project");
    let tasks_path = base.join("drafts_list.yaml");
    let mut doc = load_tasks_list_doc(&base).unwrap_or_default();
    let mut changed = 0usize;
    for key in feature_keys {
        if doc.features.iter().any(|v| v == key) || doc.planned.iter().any(|v| v == key) {
            continue;
        }
        doc.planned.push(key.clone());
        if !doc.planned_items.iter().any(|v| v.name == *key) {
            doc.planned_items.push(PlannedItemDoc {
                name: key.clone(),
                value: key.clone(),
            });
        }
        changed += 1;
    }
    if changed == 0 {
        return Ok(0);
    }
    let encoded =
        serde_yaml::to_string(&doc).map_err(|e| format!("failed to encode tasks_list yaml: {}", e))?;
    fs::write(&tasks_path, encoded)
        .map_err(|e| format!("failed to write {}: {}", tasks_path.display(), e))?;
    Ok(changed)
}

fn run_add_plan_via_cli(project_path: &Path, hint: &str) -> Result<String, String> {
    let exe = env::current_exe().map_err(|e| format!("failed to resolve current exe: {}", e))?;
    let mut cmd = Command::new(exe);
    cmd.current_dir(project_path).arg("add_code_plan");
    if !hint.trim().is_empty() {
        cmd.arg("-m").arg(hint.trim());
    }
    let out = cmd
        .output()
        .map_err(|e| format!("failed to run add_code_plan: {}", e))?;
    if out.status.success() {
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if stdout.is_empty() {
            Ok("add_code_plan executed".to_string())
        } else {
            Ok(stdout)
        }
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(format!(
            "add_code_plan failed (code={:?}) {}",
            out.status.code(),
            stderr
        ))
    }
}

fn apply_add_plan_update_from_yaml(modal: &AiChatModal, raw_response: &str) -> Result<Option<String>, String> {
    let parsed_body = if let Some(yaml) = extract_yaml_codeblock(raw_response) {
        match serde_yaml::from_str::<AddPlanUpdateDoc>(&yaml) {
            Ok(v) => Some(v.add_plan_update),
            Err(_) => extract_add_plan_update_from_raw_response(raw_response),
        }
    } else {
        extract_add_plan_update_from_raw_response(raw_response)
    };
    let mut features = Vec::new();
    let Some(parsed_body) = parsed_body else {
        return Ok(None);
    };
    for item in parsed_body.features {
        let key = normalize_feature_key(&item);
        if !is_valid_snake_feature_key(&key) || features.iter().any(|v| v == &key) {
            continue;
        }
        features.push(key);
    }
    if features.is_empty() {
        return Ok(None);
    }
    let project_path = Path::new(&modal.project_path);
    let added_features = append_project_md_features_items(project_path, &features)?;
    let add_plan_hint = if added_features.is_empty() {
        String::new()
    } else {
        added_features.join(", ")
    };
    let add_plan_msg = run_add_plan_via_cli(project_path, &add_plan_hint)?;
    let planned_added = append_planned_from_add_plan_items(project_path, &added_features)?;
    Ok(Some(format!(
        "add_code_plan applied: project.md features +{} / tasks_list planned +{} | {}",
        added_features.len(), planned_added, add_plan_msg
    )))
}

fn is_full_project_md_request(user_message: &str) -> bool {
    let compact = user_message
        .to_ascii_lowercase()
        .replace([' ', '\n', '\t'], "");
    compact.contains("project.md전체업데이트")
        || compact.contains("project.md전체출력")
        || compact.contains("fullproject.md")
        || compact.contains("full-project-md")
}

fn is_project_md_dump(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    (lower.contains("# info") && lower.contains("# rules"))
        || lower.contains("# features")
        || (lower.contains("project.md") && text.len() > 700)
}

fn spawn_ai_stream(model_bin: &str, prompt: String) -> (Receiver<AiStreamEvent>, Arc<AtomicBool>) {
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

fn cancel_ai_stream(app: &mut UiApp) {
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

fn append_project_chat_log(project_path: &str, role: &str, message: &str) {
    let log_path = Path::new(project_path).join(".project").join("chat.log");
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let _ = writeln!(file, "[{}] {}", ts, role);
        let _ = writeln!(file, "{}", message);
        let _ = writeln!(file);
    }
}

fn close_ai_chat_modal_and_open_bootstrap(
    app: &mut UiApp,
    projects: &[ProjectRecord],
    project_index: usize,
) {
    let spec_hint_owned = app
        .ai_chat_modal
        .as_ref()
        .map(|modal| modal.initial_spec.trim().to_string())
        .filter(|v| !v.is_empty() && !v.eq_ignore_ascii_case("auto"));
    cancel_ai_stream(app);
    app.ai_chat_modal = None;
    app.status_line = "ai modal closed".to_string();
    open_bootstrap_confirm_with_spec_hint(
        app,
        projects,
        project_index,
        spec_hint_owned.as_deref(),
    );
}

fn now_unix() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_string()
}

fn generate_project_id(existing: &BTreeSet<String>) -> String {
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

fn assign_missing_project_ids(projects: &mut [ProjectRecord]) -> bool {
    let mut changed = false;
    let mut existing: BTreeSet<String> = projects
        .iter()
        .filter_map(|p| if p.id.is_empty() { None } else { Some(p.id.clone()) })
        .collect();
    for project in projects {
        if project.id.is_empty() {
            let id = generate_project_id(&existing);
            existing.insert(id.clone());
            project.id = id;
            changed = true;
        }
    }
    changed
}

fn promote_recent_project_to_front(projects: &mut Vec<ProjectRecord>, recent_id: Option<&str>) {
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

fn pick_selected_project_index(projects: &[ProjectRecord]) -> usize {
    projects
        .iter()
        .position(|p| p.selected)
        .unwrap_or(0)
        .min(projects.len().saturating_sub(1))
}

fn set_selected(projects: &mut [ProjectRecord], selected_index: usize) {
    for (idx, p) in projects.iter_mut().enumerate() {
        p.selected = idx == selected_index;
    }
}

fn collect_feature_names(project: Option<&ProjectRecord>) -> Vec<String> {
    let Some(project) = project else {
        return Vec::new();
    };

    let base = Path::new(&project.path).join(".project");
    if let Some(doc) = load_tasks_list_doc(&base) {
        let mut set: BTreeSet<String> = BTreeSet::new();
        for feature in doc.features {
            set.insert(feature);
        }
        for planned in doc.planned {
            set.insert(planned);
        }
        if !set.is_empty() {
            return set.into_iter().collect();
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

fn load_tasks_list_doc(base: &Path) -> Option<DraftsListDoc> {
    if base
        .file_name()
        .and_then(|v| v.to_str())
        .map(|v| v == ".project")
        .unwrap_or(false)
    {
        if let Some(project_root) = base.parent() {
            let _ = crate::sync_project_tasks_list_from_project_md(project_root);
        }
    }
    for name in ["drafts_list.yaml"] {
        let path = base.join(name);
        let Ok(raw) = fs::read_to_string(path) else {
            continue;
        };
        let Ok(doc) = serde_yaml::from_str::<DraftsListDoc>(&raw) else {
            continue;
        };
        return Some(doc);
    }
    None
}

fn pane_border_style(active: bool, palette: BorderPalette) -> Style {
    let color = if active {
        palette.active
    } else {
        palette.normal
    };
    Style::default().fg(color)
}

fn has_overlay_modal(app: &UiApp) -> bool {
    app.create_modal.is_some()
        || app.path_change_confirm.is_some()
        || app.delete_confirm.is_some()
        || app.detail_fill_confirm.is_some()
        || app.draft_bulk_add_modal.is_some()
        || app.list_edit_modal.is_some()
        || app.draft_create_confirm.is_some()
        || app.bootstrap_confirm.is_some()
        || app.ai_chat_modal.is_some()
        || app.alarm_modal.is_some()
        || app.busy_message.is_some()
}

fn color_rgb(color: Color) -> Option<(u8, u8, u8)> {
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

fn lerp_u8(from: u8, to: u8, t: f32) -> u8 {
    let start = from as f32;
    let end = to as f32;
    (start + (end - start) * t).round().clamp(0.0, 255.0) as u8
}

fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    let ratio = t.clamp(0.0, 1.0);
    let Some((fr, fg, fb)) = color_rgb(from) else {
        return if ratio < 1.0 { from } else { to };
    };
    let Some((tr, tg, tb)) = color_rgb(to) else {
        return if ratio < 1.0 { from } else { to };
    };
    Color::Rgb(
        lerp_u8(fr, tr, ratio),
        lerp_u8(fg, tg, ratio),
        lerp_u8(fb, tb, ratio),
    )
}

fn active_pane_tween_progress(app: &UiApp, pane_index: usize) -> Option<f32> {
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

fn tweened_pane_border_style(app: &UiApp, pane_index: usize, palette: BorderPalette) -> Style {
    if has_overlay_modal(app) {
        return Style::default().fg(palette.inactive);
    }
    let Some(progress) = active_pane_tween_progress(app, pane_index) else {
        return Style::default().fg(palette.normal);
    };
    Style::default().fg(lerp_color(palette.normal, palette.active, progress))
}

fn inset_rect(area: Rect, margin: u16) -> Rect {
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

fn active_pane_margin(_app: &UiApp, _pane_index: usize) -> u16 {
    0
}

fn start_pane_activate_tween(app: &mut UiApp) {
    app.pane_activate_started_at = Some(Instant::now());
    app.pane_activate_index = app.pane_focus;
}

fn reset_parallel_runtime(app: &mut UiApp) {
    app.parallel_running = false;
    app.parallel_statuses.clear();
}

fn move_project_grid_selection(projects: &mut [ProjectRecord], app: &mut UiApp, delta: isize) {
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
    set_selected(projects, app.project_index);
    app.changed = true;
    app.status_line = format!("selected project: {}", projects[app.project_index].name);
}

fn move_detail_pane_focus(app: &mut UiApp, key: KeyCode) {
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
        (3, KeyCode::Right) => 5,
        (4, KeyCode::Left) => 2,
        (4, KeyCode::Up) => 0,
        (4, KeyCode::Down) => 5,
        (5, KeyCode::Left) => 3,
        (5, KeyCode::Up) => 4,
        _ => app.pane_focus,
    };
    start_pane_activate_tween(app);
}

fn start_parallel_runtime(app: &mut UiApp, features: &[String]) {
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

fn advance_parallel_runtime(app: &mut UiApp, projects: &[ProjectRecord]) {
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
        let planned = collect_planned_drafts_from_project(project);
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

fn default_project_name_from_parent() -> String {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    cwd.file_name()
        .and_then(|v| v.to_str())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "new-project".to_string())
}

fn open_create_modal(app: &mut UiApp) {
    let default_name = default_project_name_from_parent();
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

fn open_edit_modal(app: &mut UiApp, projects: &[ProjectRecord]) {
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

fn resolve_project_path(raw_path: &str) -> Result<PathBuf, String> {
    let trimmed = raw_path.trim();
    let mut path = if trimmed.is_empty() {
        PathBuf::from(".")
    } else if trimmed == "~" {
        env::var("HOME")
            .map(PathBuf::from)
            .map_err(|e| format!("failed to resolve HOME: {}", e))?
    } else if let Some(rest) = trimmed.strip_prefix("~/") {
        let home = env::var("HOME").map_err(|e| format!("failed to resolve HOME: {}", e))?;
        PathBuf::from(home).join(rest)
    } else {
        PathBuf::from(trimmed)
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

fn apply_project_create(
    projects: &mut Vec<ProjectRecord>,
    app: &mut UiApp,
    modal: &CreateProjectModal,
) -> Result<(), String> {
    let name = modal.name.trim();
    if name.is_empty() {
        return Err("name is required".to_string());
    }
    let path = resolve_project_path(modal.path.trim())?;
    fs::create_dir_all(&path)
        .map_err(|e| format!("failed to create project dir {}: {}", path.display(), e))?;
    fs::create_dir_all(path.join(".project"))
        .map_err(|e| format!("failed to create project meta dir: {}", e))?;

    let now = now_unix();
    let mut created_new = false;
    let selected_index = if let Some((idx, p)) = projects
        .iter_mut()
        .enumerate()
        .find(|(_, p)| p.name == name)
    {
        p.path = path.display().to_string();
        p.description = modal.description.trim().to_string();
        p.updated_at = now;
        if !has_any_project_md(&path) {
            let _ = run_create_project_in_project_dir(
                &path,
                name,
                modal.description.trim(),
            )?;
            created_new = true;
        }
        let _ = sync_project_md_files(&path)?;
        app.status_line = format!("project updated: {}", name);
        idx
    } else {
        let existing_ids: BTreeSet<String> = projects
            .iter()
            .filter_map(|p| if p.id.is_empty() { None } else { Some(p.id.clone()) })
            .collect();
        let create_project_msg =
            run_create_project_in_project_dir(&path, name, modal.description.trim())?;
        created_new = true;
        projects.push(ProjectRecord {
            id: generate_project_id(&existing_ids),
            name: name.to_string(),
            path: path.display().to_string(),
            description: modal.description.trim().to_string(),
            created_at: now.clone(),
            updated_at: now,
            selected: false,
        });
        let _ = sync_project_md_files(&path)?;
        app.status_line = if create_project_msg.is_empty() {
            format!("project created: {}", name)
        } else {
            format!("project created: {} | {}", name, create_project_msg)
        };
        projects.len().saturating_sub(1)
    };
    set_selected(projects, selected_index);
    app.project_index = selected_index;
    app.changed = true;
    reset_parallel_runtime(app);
    if created_new {
        app.tab_index = 1;
        app.pane_focus = 0;
        app.menu_active = true;
        let initial_spec = modal.spec.trim();
        let initial_spec = if modal.spec_is_default || initial_spec.is_empty() {
            None
        } else {
            Some(initial_spec)
        };
        open_ai_onboarding_modal(app, projects, selected_index, initial_spec);
    }
    Ok(())
}

fn try_submit_edit_project(
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
    let path = resolve_project_path(modal.path.trim())?;
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

    let now = now_unix();
    {
        let target = &mut projects[source_index];
        target.name = name.to_string();
        target.description = modal.description.trim().to_string();
        target.updated_at = now;
    }
    set_selected(projects, source_index);
    app.project_index = source_index;
    app.changed = true;
    reset_parallel_runtime(app);
    app.status_line = format!("project updated: {}", projects[source_index].name);
    Ok(())
}

fn apply_path_change_confirm(
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

    let now = now_unix();
    {
        let target = &mut projects[confirm.source_index];
        target.name = confirm.new_name;
        target.description = confirm.new_description;
        if move_dir {
            target.path = confirm.new_path;
        }
        target.updated_at = now;
    }
    set_selected(projects, confirm.source_index);
    app.project_index = confirm.source_index;
    app.changed = true;
    reset_parallel_runtime(app);
    let updated_name = projects[confirm.source_index].name.clone();
    app.status_line = if move_dir {
        format!("project updated and moved: {}", updated_name)
    } else {
        format!("project updated without path move: {}", updated_name)
    };
    Ok(())
}

fn open_delete_confirm(app: &mut UiApp, projects: &[ProjectRecord]) {
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

fn apply_delete_confirm(
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
        set_selected(projects, app.project_index);
    }
    app.changed = true;
    reset_parallel_runtime(app);
    app.status_line = format!("project deleted: {}", confirm.project_name);
    Ok(())
}

fn render_projects_tab(
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
            pane_border_style(true, palette).add_modifier(Modifier::BOLD)
        } else {
            pane_border_style(false, palette).add_modifier(Modifier::DIM)
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
                pane_border_style(true, palette)
            } else {
                pane_border_style(false, palette)
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

fn render_details_tab(
    f: &mut ratatui::Frame,
    area: Rect,
    app: &UiApp,
    projects: &[ProjectRecord],
    _features: &[String],
    _menu_active: bool,
    palette: BorderPalette,
) {
    let _ = &app.detail_layout.preset;
    let project_slot = layout_panel_rect(&app.detail_layout, area, "project").unwrap_or(area);
    let rule_slot = layout_panel_rect(&app.detail_layout, area, "rule").unwrap_or(area);
    let constraint_slot = layout_panel_rect(&app.detail_layout, area, "constraint").unwrap_or(area);
    let feature_slot = layout_panel_rect(&app.detail_layout, area, "features").unwrap_or(area);
    let draft_slot = layout_panel_rect(&app.detail_layout, area, "drafts").unwrap_or(area);
    let selected_project = projects.get(app.project_index);
    let project_md = selected_project.and_then(read_project_md);
    let parsed = project_md.as_deref().map(parse_project_md);
    let parsed_has_core_info = parsed.as_ref().map_or(false, |doc| {
        !doc.name.trim().is_empty()
            || !doc.description.trim().is_empty()
            || !doc.spec.trim().is_empty()
            || !doc.goal.trim().is_empty()
    });
    let (name_value, desc_value, spec_value, goal_value): (String, String, String, String) =
        if parsed_has_core_info {
            let doc = parsed.as_ref().expect("parsed exists when parsed_has_core_info");
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
                "spec not set".to_string(),
                "goal not set".to_string(),
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

    let project_area = inset_rect(project_slot, active_pane_margin(app, 0));
    let project_block = Block::default()
        .title(project_title)
        .borders(Borders::ALL)
        .border_style(tweened_pane_border_style(app, 0, palette));
    let project_inner = project_block.inner(project_area);
    f.render_widget(project_block, project_area);
    let max_w = project_inner.width.saturating_sub(2).max(8);
    let separator = "─".repeat(max_w as usize);
    let project_lines = vec![
        Line::from(truncate_to_width_ellipsis(
            &format!("Name: {}", name_value),
            max_w,
        )),
        Line::from(Span::styled(
            separator.clone(),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(truncate_to_width_ellipsis(
            &format!("Description: {}", desc_value),
            max_w,
        )),
        Line::from(Span::styled(
            separator.clone(),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(truncate_to_width_ellipsis(
            &format!("Spec: {}", spec_value),
            max_w,
        )),
        Line::from(Span::styled(
            separator.clone(),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(truncate_to_width_ellipsis(
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
            let max_w = rule_slot.width.saturating_sub(6).max(8);
            doc.rules
                .iter()
                .enumerate()
                .map(|(idx, item)| {
                    let raw = format!("{}. {}", idx + 1, item);
                    Line::from(truncate_to_width_ellipsis(&raw, max_w))
                })
                .collect()
        }
    } else {
        vec![Line::from("no selected project")]
    };
    let rule_area = inset_rect(rule_slot, active_pane_margin(app, 1));
    let rule_block = Block::default()
        .title(layout_panel_name(&app.detail_layout, "rule", "Rule"))
        .borders(Borders::ALL)
        .border_style(tweened_pane_border_style(app, 1, palette));
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
            let max_w = constraint_slot.width.saturating_sub(6).max(8);
            doc.constraints
                .iter()
                .enumerate()
                .map(|(idx, item)| {
                    let raw = format!("{}. {}", idx + 1, item);
                    Line::from(truncate_to_width_ellipsis(&raw, max_w))
                })
                .collect()
        }
    } else {
        vec![Line::from("no selected project")]
    };
    let constraint_area = inset_rect(constraint_slot, active_pane_margin(app, 2));
    let constraint_block = Block::default()
        .title(layout_panel_name(&app.detail_layout, "constraint", "Constraint"))
        .borders(Borders::ALL)
        .border_style(tweened_pane_border_style(app, 2, palette));
    f.render_widget(
        Paragraph::new(constraint_lines)
            .block(constraint_block)
            .wrap(Wrap { trim: false }),
        constraint_area,
    );

    let feature_lines: Vec<Line> = selected_project
        .map(collect_feature_items_from_drafts)
        .map(|features| {
            if features.is_empty() {
                vec![Line::from("no feature")]
            } else {
                let max_w = feature_slot.width.saturating_sub(6).max(8);
                features
                    .iter()
                    .enumerate()
                    .map(|(idx, item)| {
                        let raw = format!("{}. {}", idx + 1, item);
                        Line::from(truncate_to_width_ellipsis(&raw, max_w))
                    })
                    .collect()
            }
        })
        .unwrap_or_else(|| vec![Line::from("no selected project")]);
    let feature_area = inset_rect(feature_slot, active_pane_margin(app, 3));
    let feature_block = Block::default()
        .title("Support Features")
        .borders(Borders::ALL)
        .border_style(tweened_pane_border_style(app, 3, palette));
    f.render_widget(
        Paragraph::new(feature_lines)
            .block(feature_block)
            .wrap(Wrap { trim: false }),
        feature_area,
    );

    let planned = selected_project
        .map(collect_planned_drafts_from_project)
        .unwrap_or_default();
    let planned_display = selected_project
        .map(collect_planned_display_items_from_project)
        .unwrap_or_default();
    let generated = selected_project
        .map(collect_generated_draft_items_from_project)
        .unwrap_or_default();
    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(draft_slot);

    let plan_area = inset_rect(right_rows[0], active_pane_margin(app, 4));
    let plan_selected = app.menu_active
        && app.tab_index == 1
        && app.pane_focus == 4
        && !has_overlay_modal(app);
    let plan_border_style = if plan_selected {
        tweened_pane_border_style(app, 4, palette)
    } else if planned.is_empty() {
        Style::default().fg(palette.inactive)
    } else {
        Style::default().fg(palette.normal)
    };
    let plan_block = Block::default()
        .title("Plan")
        .borders(Borders::ALL)
        .border_style(plan_border_style);
    if planned.is_empty() {
        let inner = plan_block.inner(plan_area);
        f.render_widget(plan_block, plan_area);
        let body_area = Rect {
            x: inner.x,
            y: inner.y.saturating_add(inner.height.saturating_sub(1) / 2),
            width: inner.width,
            height: 1,
        };
        f.render_widget(
            Paragraph::new(vec![Line::from(Span::styled(
                "no planned item",
                Style::default().fg(palette.inactive),
            ))])
            .alignment(Alignment::Center),
            body_area,
        );
    } else {
        let plan_lines: Vec<Line> = planned_display
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
            Paragraph::new(plan_lines)
                .block(plan_block)
                .wrap(Wrap { trim: false }),
            plan_area,
        );
    }

    let draft_area = inset_rect(right_rows[1], active_pane_margin(app, 5));
    let draft_title = if app.parallel_running {
        "Drafts | 작업중".to_string()
    } else {
        "Drafts".to_string()
    };
    let draft_selected = app.menu_active
        && app.tab_index == 1
        && app.pane_focus == 5
        && !has_overlay_modal(app);
    let draft_border_style = if app.parallel_running || draft_selected {
        tweened_pane_border_style(app, 5, palette)
    } else if generated.is_empty() {
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
                    truncate_to_width_ellipsis(&raw, max_w),
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
    } else if generated.is_empty() {
        let inner = draft_block.inner(draft_area);
        f.render_widget(draft_block, draft_area);
        let body_area = Rect {
            x: inner.x,
            y: inner.y.saturating_add(inner.height.saturating_sub(1) / 2),
            width: inner.width,
            height: 1,
        };
        f.render_widget(
            Paragraph::new(vec![Line::from(Span::styled(
                "no draft item",
                Style::default().fg(palette.inactive),
            ))])
            .alignment(Alignment::Center),
            body_area,
        );
    } else {
        let draft_lines: Vec<Line> = generated
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

fn read_project_md(project: &ProjectRecord) -> Option<String> {
    let root = Path::new(&project.path);
    let shadow = project_md_shadow_path(root);
    fs::read_to_string(shadow).ok()
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

fn parse_project_md(project_md: &str) -> ProjectMdDoc {
    let mut doc = ProjectMdDoc::default();
    let mut in_rule = false;
    let mut in_constraints = false;
    for line in project_md.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("# rules") {
            in_rule = true;
            in_constraints = false;
            continue;
        }
        if trimmed.eq_ignore_ascii_case("# constraints") {
            in_rule = false;
            in_constraints = true;
            continue;
        }
        if trimmed.starts_with('#') && !trimmed.eq_ignore_ascii_case("# rules") {
            in_rule = false;
            if !trimmed.eq_ignore_ascii_case("# constraints") {
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
        if let Some((key, value)) = trimmed.split_once(':') {
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim().to_string();
            match key.as_str() {
                "name" => doc.name = value,
                "description" => doc.description = value,
                "spec" => doc.spec = value,
                "goal" => doc.goal = value,
                _ => {}
            }
        }
    }
    doc
}

#[cfg(test)]
mod tests {
    use super::*;

    struct VirtualPaneInput {
        project_md: String,
        tasks_doc: DraftsListDoc,
        generated_files: Vec<(String, String)>,
    }

    struct DisplayPaneValues {
        name: String,
        description: String,
        spec: String,
        goal: String,
        rules: Vec<String>,
        constraints: Vec<String>,
        features: Vec<String>,
        planned: Vec<String>,
        planned_display: Vec<String>,
        generated: Vec<String>,
    }

    fn make_temp_dir(prefix: &str) -> PathBuf {
        let base = std::env::temp_dir();
        let uniq = format!(
            "{}_{}_{}",
            prefix,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let dir = base.join(uniq);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn collect_display_values_from_virtual_input(input: &VirtualPaneInput) -> DisplayPaneValues {
        let dir = make_temp_dir("orc_ui_pane_mapping");
        let project_meta = dir.join(".project");
        fs::create_dir_all(&project_meta).expect("create .project");
        fs::write(project_meta.join("project.md"), &input.project_md).expect("write project.md");
        let tasks_raw = serde_yaml::to_string(&input.tasks_doc).expect("encode tasks_list");
        fs::write(project_meta.join("drafts_list.yaml"), tasks_raw).expect("write drafts_list");

        for (feature_name, file_name) in &input.generated_files {
            let feature_dir = project_meta.join("feature").join(feature_name);
            fs::create_dir_all(&feature_dir).expect("create feature dir");
            fs::write(feature_dir.join(file_name), "task:\n- name: stub\n").expect("write task");
        }

        let project = crate::ProjectRecord {
            id: "p1".to_string(),
            name: "temp".to_string(),
            path: dir.display().to_string(),
            description: "desc".to_string(),
            created_at: "0".to_string(),
            updated_at: "0".to_string(),
            selected: true,
        };

        let parsed = parse_project_md(&input.project_md);
        let values = DisplayPaneValues {
            name: parsed.name,
            description: parsed.description,
            spec: parsed.spec,
            goal: parsed.goal,
            rules: parsed.rules,
            constraints: parsed.constraints,
            features: collect_feature_items_from_drafts(&project),
            planned: collect_planned_drafts_from_project(&project),
            planned_display: collect_planned_display_items_from_project(&project),
            generated: collect_generated_draft_items_from_project(&project),
        };

        let _ = fs::remove_dir_all(dir);
        values
    }

    #[test]
    fn parse_project_md_accepts_spec_with_space_before_colon() {
        let md = "# info\nname : sample\nspec : typescript react axios\n";
        let parsed = parse_project_md(md);
        assert_eq!(parsed.spec, "typescript react axios");
    }

    #[test]
    fn parse_project_md_accepts_spec_with_hyphen_and_comma() {
        let md = "# info\nname : sample\nspec : react, @react-three/fiber, three-fiber, zustand\n";
        let parsed = parse_project_md(md);
        assert_eq!(
            parsed.spec,
            "react, @react-three/fiber, three-fiber, zustand"
        );
    }

    #[test]
    fn onboarding_signal_uses_initial_spec_hint() {
        let project = crate::ProjectRecord {
            id: "p1".to_string(),
            name: "temp".to_string(),
            path: "/tmp".to_string(),
            description: "desc".to_string(),
            created_at: "0".to_string(),
            updated_at: "0".to_string(),
            selected: true,
        };
        let mut modal = new_ai_chat_modal_template(
            &project,
            0,
            AiChatMode::DetailProject,
            "codex".to_string(),
        );
        modal.initial_spec = "react,zustand,three-fiber".to_string();
        let (spec_ready, domain_ready, feature_count) =
            collect_onboarding_signals(&modal, "원하는 도메인 : player, character, system");
        assert!(spec_ready);
        assert!(domain_ready);
        assert_eq!(feature_count, 0);
    }

    #[test]
    fn bootstrap_prompt_template_exists() {
        let path = resolve_bootstrap_prompt_path().expect("bootstrap prompt path");
        let raw = fs::read_to_string(path).expect("read bootstrap prompt");
        assert!(raw.contains("{{project_name}}"));
        assert!(raw.contains("{{spec}}"));
        assert!(!raw.contains("{{project_md}}"));
    }

    #[test]
    fn preset_libraries_allowlist_filters_unknown_values() {
        let filtered = filter_allowed_preset_libraries(&[
            "three".to_string(),
            "@react-three/fiber".to_string(),
            "@react-three/drei".to_string(),
            "unknown-lib".to_string(),
            "three".to_string(),
        ]);
        assert_eq!(
            filtered,
            vec![
                "three".to_string(),
                "@react-three/fiber".to_string(),
                "@react-three/drei".to_string()
            ]
        );
    }

    #[test]
    fn cursor_no_wrap_stays_on_same_line_for_mixed_width_text() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 8,
            height: 3,
        };
        let (_, y) = cursor_in_input_with_wrap(area, "한a한a한a", false);
        assert_eq!(y, 1);
    }

    #[test]
    fn detail_panes_data_mapping_is_consistent() {
        let input = VirtualPaneInput {
            project_md: r#"# info
name : temp
description : zustand, react, threefiber를 이용한 점프 게임
spec : react, zustand, three-fiber
goal : 100번 점프 달성 시 승리

# rules
- 점프 카운트는 1회 입력당 1 증가
- UI 전환은 easing 애니메이션을 사용

# constraints
- 점프 카운트는 음수가 될 수 없다
- 승리 조건은 100회 이상으로 고정
            "#
            .to_string(),
            tasks_doc: DraftsListDoc {
                features: vec![
                    "jump_action : cube를 누르면 점프".to_string(),
                    "victory_rule : 100회 점프 시 승리".to_string(),
                ],
                planned: vec!["jump_action".to_string(), "victory_rule".to_string()],
                planned_items: vec![
                    PlannedItemDoc {
                        name: "jump_action".to_string(),
                        value: "cube를 누르면 점프한다".to_string(),
                    },
                    PlannedItemDoc {
                        name: "victory_rule".to_string(),
                        value: "100번 이상 점프하면 승리한다".to_string(),
                    },
                ],
                sync_initialized: true,
                ..Default::default()
            },
            generated_files: vec![
                ("jump".to_string(), "drafts.yaml".to_string()),
                ("win".to_string(), "drafts.yaml".to_string()),
            ],
        };

        let values = collect_display_values_from_virtual_input(&input);
        assert_eq!(values.name, "temp");
        assert_eq!(
            values.description,
            "zustand, react, threefiber를 이용한 점프 게임"
        );
        assert_eq!(values.spec, "react, zustand, three-fiber");
        assert_eq!(values.goal, "100번 점프 달성 시 승리");
        assert_eq!(values.rules.len(), 2);
        assert_eq!(values.constraints.len(), 2);

        assert_eq!(values.features.len(), 2);
        assert!(
            values
                .features
                .iter()
                .any(|v| v == "jump_action : cube를 누르면 점프")
        );
        assert_eq!(
            values.planned,
            vec!["jump_action".to_string(), "victory_rule".to_string()]
        );
        assert_eq!(
            values.planned_display,
            vec![
                "cube를 누르면 점프한다".to_string(),
                "100번 이상 점프하면 승리한다".to_string()
            ]
        );
        assert_eq!(values.generated, vec!["jump".to_string(), "win".to_string()]);
    }

    #[test]
    fn detail_layout_panel_shortcut_is_compiled_and_selected() {
        let doc = DetailLayoutDoc {
            grid: DetailLayoutGridDoc {
                columns: 10,
                rows: 10,
            },
            panels: vec![
                DetailLayoutPanelDoc {
                    id: "rule".to_string(),
                    name: "Rule".to_string(),
                    panel_type: "list".to_string(),
                    selected_view: "rule_list".to_string(),
                    shortcut: "enter: edit-rule".to_string(),
                    cell_start: 1,
                    cell_end: 1,
                },
                DetailLayoutPanelDoc {
                    id: "drafts".to_string(),
                    name: "Drafts".to_string(),
                    panel_type: "runtime".to_string(),
                    selected_view: "parallel_status".to_string(),
                    shortcut: "b: create_code_draft/enter-parallel".to_string(),
                    cell_start: 2,
                    cell_end: 2,
                },
            ],
        };
        let layout = compile_detail_layout("test", doc).expect("compile layout");
        assert_eq!(
            selected_pane_shortcut(&layout, 1, 1),
            "enter: edit-rule".to_string()
        );
        assert_eq!(
            selected_pane_shortcut(&layout, 1, 5),
            "b: create_code_draft/enter-parallel".to_string()
        );
    }
}

fn collect_planned_drafts_from_project(project: &ProjectRecord) -> Vec<String> {
    let base = Path::new(&project.path).join(".project");
    let Some(doc) = load_tasks_list_doc(&base) else {
        return Vec::new();
    };
    doc.planned
}

fn collect_planned_display_items_from_project(project: &ProjectRecord) -> Vec<String> {
    let base = Path::new(&project.path).join(".project");
    let Some(doc) = load_tasks_list_doc(&base) else {
        return Vec::new();
    };
    doc.planned
        .iter()
        .map(|key| {
            doc.planned_items
                .iter()
                .find(|item| item.name == *key)
                .map(|item| item.value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| key.clone())
        })
        .collect()
}

fn collect_generated_draft_items_from_project(project: &ProjectRecord) -> Vec<String> {
    let root = Path::new(&project.path).join(".project").join("feature");
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let dir = entry.path();
        let has_task = [
            dir.join("drafts.yaml"),
            dir.join("tasks.yaml"),
            dir.join("drafts.yaml"),
            dir.join("drafts.yaml"),
        ]
        .iter()
        .any(|p| p.exists());
        if !has_task {
            continue;
        }
        if let Some(name) = entry.file_name().to_str() {
            out.push(name.to_string());
        }
    }
    out.sort();
    out
}

fn open_list_edit_modal(
    app: &mut UiApp,
    projects: &[ProjectRecord],
    target: ListEditTarget,
) {
    let Some(project) = projects.get(app.project_index) else {
        app.status_line = "no selected project".to_string();
        return;
    };
    let md = read_project_md(project).unwrap_or_default();
    let parsed = parse_project_md(&md);
    let items = match target {
        ListEditTarget::Rule => parsed.rules,
        ListEditTarget::Constraint => parsed.constraints,
        ListEditTarget::Feature => collect_feature_items_from_drafts(project),
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

fn save_project_md_list(
    projects: &[ProjectRecord],
    project_index: usize,
    target: ListEditTarget,
    items: &[String],
) -> Result<(), String> {
    if matches!(target, ListEditTarget::Feature) {
        return save_drafts_feature_list(projects, project_index, items);
    }
    let Some(project) = projects.get(project_index) else {
        return Err("selected project index out of range".to_string());
    };
    let project_root = Path::new(&project.path);
    let raw = read_project_md(project)
        .ok_or_else(|| format!("failed to read project.md at {}", project.path))?;
    let mut lines: Vec<String> = raw.lines().map(|v| v.to_string()).collect();
    let header = match target {
        ListEditTarget::Rule => "# rules",
        ListEditTarget::Constraint => "# constraints",
        ListEditTarget::Feature => "# features",
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
    write_project_md_with_sync(project_root, &(lines.join("\n") + "\n"))
}

fn collect_feature_items_from_drafts(project: &ProjectRecord) -> Vec<String> {
    let base = Path::new(&project.path).join(".project");
    let Some(doc) = load_tasks_list_doc(&base) else {
        return Vec::new();
    };
    doc.features
}

fn normalize_feature_item(value: &str) -> Result<String, String> {
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

fn save_drafts_feature_list(
    projects: &[ProjectRecord],
    project_index: usize,
    items: &[String],
) -> Result<(), String> {
    let Some(project) = projects.get(project_index) else {
        return Err("selected project index out of range".to_string());
    };
    let base = Path::new(&project.path).join(".project");
    let path = base.join("drafts_list.yaml");
    let mut doc = load_tasks_list_doc(&base).unwrap_or_default();
    let mut normalized = Vec::new();
    for item in items {
        normalized.push(normalize_feature_item(item)?);
    }
    doc.features = normalized;
    let encoded =
        serde_yaml::to_string(&doc).map_err(|e| format!("failed to encode tasks_list yaml: {}", e))?;
    fs::write(&path, encoded).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn truncate_to_width_ellipsis(value: &str, width: u16) -> String {
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

fn render_list_edit_modal(
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
            let value = truncate_to_width_ellipsis(&modal.items[idx], list_w.saturating_sub(2));
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

    component::render_confirm_buttons_bottom_right(
        f,
        inner,
        "Confirm",
        "Cancel",
        modal.confirm_selected,
    );

    if modal.input_mode.is_some() {
        let editor_area = centered_rect(72, 24, area);
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
        Some(cursor_in_input(input_area, &modal.input))
    } else {
        None
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
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

fn input_value_style(is_default: bool) -> Style {
    if is_default {
        Style::default().fg(Color::Black)
    } else {
        Style::default()
    }
}

fn modal_field_value_style(modal: &CreateProjectModal, field_index: usize, is_default: bool) -> Style {
    if modal.field_index != field_index {
        return Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM);
    }
    input_value_style(is_default)
}

fn modal_input_border_style(active: bool) -> Style {
    if active {
        Style::default()
            .fg(Color::Rgb(255, 165, 0))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM)
    }
}

fn modal_label_style(active: bool) -> Style {
    if active {
        Style::default()
            .bg(Color::Black)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

fn render_create_modal(
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
        modal_label_style(modal.field_index == 0),
    )));
    f.render_widget(name_label, layout[0]);
    let name_block = Block::default()
        .borders(Borders::ALL)
        .border_style(modal_input_border_style(modal.field_index == 0));
    f.render_widget(
        Paragraph::new(modal.name.clone())
            .style(modal_field_value_style(modal, 0, modal.name_is_default))
            .block(name_block),
        layout[1],
    );

    let desc_label = Paragraph::new(Line::from(Span::styled(
        "Description",
        modal_label_style(modal.field_index == 1),
    )));
    f.render_widget(desc_label, layout[2]);
    let desc_block = Block::default()
        .borders(Borders::ALL)
        .border_style(modal_input_border_style(modal.field_index == 1));
    f.render_widget(
        Paragraph::new(modal.description.clone())
            .style(modal_field_value_style(modal, 1, modal.description_is_default))
            .wrap(Wrap { trim: false })
            .block(desc_block),
        layout[3],
    );

    let spec_label = Paragraph::new(Line::from(Span::styled(
        "Spec",
        modal_label_style(modal.field_index == 2),
    )));
    f.render_widget(spec_label, layout[4]);
    let spec_block = Block::default()
        .borders(Borders::ALL)
        .border_style(modal_input_border_style(modal.field_index == 2));
    f.render_widget(
        Paragraph::new(modal.spec.clone())
            .style(modal_field_value_style(modal, 2, modal.spec_is_default))
            .block(spec_block),
        layout[5],
    );

    let path_label = Paragraph::new(Line::from(Span::styled(
        "Project Path",
        modal_label_style(modal.field_index == 3),
    )));
    f.render_widget(path_label, layout[6]);
    let path_block = Block::default()
        .borders(Borders::ALL)
        .border_style(modal_input_border_style(modal.field_index == 3));
    f.render_widget(
        Paragraph::new(modal.path.clone())
            .style(modal_field_value_style(modal, 3, modal.path_is_default))
            .block(path_block),
        layout[7],
    );

    component::render_confirm_buttons_bottom_right(
        f,
        inner,
        "Confirm",
        "Cancel",
        modal.confirm_selected,
    );

    modal_cursor(modal, layout[1], layout[3], layout[5], layout[7])
}

fn modal_cursor(
    modal: &CreateProjectModal,
    name_area: Rect,
    desc_area: Rect,
    spec_area: Rect,
    path_area: Rect,
) -> Option<(u16, u16)> {
    match modal.field_index {
        0 => Some(cursor_in_input_with_wrap(name_area, &modal.name, false)),
        1 => Some(cursor_in_input(desc_area, &modal.description)),
        2 => Some(cursor_in_input_with_wrap(spec_area, &modal.spec, false)),
        3 => Some(cursor_in_input_with_wrap(path_area, &modal.path, false)),
        _ => None,
    }
}

fn cursor_in_input(area: Rect, value: &str) -> (u16, u16) {
    cursor_in_input_with_wrap(area, value, true)
}

fn cursor_in_input_with_wrap(area: Rect, value: &str, wrap: bool) -> (u16, u16) {
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
        if wrap && col.saturating_add(ch_width) > inner_w {
            row = row.saturating_add(1);
            col = 0;
        }
        col = col.saturating_add(ch_width).min(inner_w);
    }

    let clamped_row = row.min(inner_h.saturating_sub(1));
    let clamped_col = col.min(inner_w.saturating_sub(1));
    (
        area.x.saturating_add(1).saturating_add(clamped_col),
        area.y.saturating_add(1).saturating_add(clamped_row),
    )
}

fn handle_modal_input(
    _projects: &mut Vec<ProjectRecord>,
    app: &mut UiApp,
    key: KeyCode,
) -> Result<bool, String> {
    let Some(mut modal) = app.create_modal.take() else {
        return Ok(false);
    };
    let mut close_modal = false;

    match key {
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

fn render_path_change_confirm_modal(
    f: &mut ratatui::Frame,
    area: Rect,
    confirm: &PathChangeConfirm,
) {
    let lines = vec![
        Line::from("Path changed. Move project directory?"),
        Line::from(format!("from: {}", confirm.old_path)),
        Line::from(format!("to: {}", confirm.new_path)),
    ];
    component::render_confirm_cancel_wrapper(
        f,
        area,
        "Move Project Path",
        &lines,
        "Move",
        "Keep",
        confirm.confirm_selected,
    );
}

fn render_delete_confirm_modal(
    f: &mut ratatui::Frame,
    area: Rect,
    confirm: &DeleteProjectConfirm,
) {
    let lines = vec![
        Line::from(format!("Delete project `{}`?", confirm.project_name)),
        Line::from(format!("path: {}", confirm.project_path)),
        Line::from("This removes all files/folders inside `<path>/.project`."),
    ];
    component::render_confirm_cancel_wrapper(
        f,
        area,
        "Delete Project",
        &lines,
        "Delete",
        "Cancel",
        confirm.confirm_selected,
    );
}

fn render_detail_fill_confirm_modal(
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
    component::render_confirm_cancel_wrapper(
        f,
        area,
        "Fill Project Detail",
        &lines,
        "Open",
        "Skip",
        confirm.confirm_selected,
    );
}

fn render_draft_create_confirm_modal(
    f: &mut ratatui::Frame,
    area: Rect,
    confirm: &DraftCreateConfirm,
) {
    let lines = vec![
        Line::from("Drafts pane selected."),
        Line::from("Run `create_code_draft` now?"),
        Line::from("This triggers plan-drafts-code from current project."),
    ];
    component::render_confirm_cancel_wrapper(
        f,
        area,
        "Create Draft",
        &lines,
        "Run",
        "Cancel",
        confirm.confirm_selected,
    );
}

fn render_bootstrap_confirm_modal(
    f: &mut ratatui::Frame,
    area: Rect,
    confirm: &BootstrapConfirm,
) {
    let lines = vec![
        Line::from("상세 기획 반영이 완료되었습니다."),
        Line::from("spec 기준으로 프로젝트 bootstrap을 실행할까요?"),
        Line::from(format!("spec: {}", confirm.spec)),
    ];
    component::render_confirm_cancel_wrapper(
        f,
        area,
        "Project Bootstrap",
        &lines,
        "Bootstrap",
        "Skip",
        confirm.confirm_selected,
    );
}

fn ai_detail_input_border_style(modal: &AiChatModal) -> Style {
    if modal.focus == AiDetailFocus::Input && modal.input_active {
        Style::default()
            .fg(Color::Rgb(255, 165, 0))
            .add_modifier(Modifier::BOLD)
    } else if modal.focus == AiDetailFocus::Input {
        Style::default().fg(Color::Rgb(255, 165, 0))
    } else {
        Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM)
    }
}

fn ai_response_scroll(modal: &AiChatModal, response_area_height: u16) -> u16 {
    let viewport_rows = response_area_height.saturating_sub(2) as usize;
    if viewport_rows == 0 {
        return 0;
    }
    let mut total_rows: usize = 0;
    for msg in &modal.history {
        total_rows = total_rows
            .saturating_add(msg.lines().count().max(1))
            .saturating_add(1);
    }
    if modal.streaming && !modal.warmup_inflight {
        total_rows = total_rows.saturating_add(1);
    }
    total_rows
        .saturating_sub(viewport_rows)
        .min(u16::MAX as usize) as u16
}

fn render_ai_chat_modal(
    f: &mut ratatui::Frame,
    area: Rect,
    modal: &AiChatModal,
) -> Option<(u16, u16)> {
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
    let response_scroll = ai_response_scroll(modal, area.height.saturating_mul(68) / 100);
    let input_rect = component::render_llm_chat_pane(
        f,
        area,
        &component::LlmChatPaneView {
            project_name: &modal.project_name,
            history: &modal.history,
            streaming: modal.streaming,
            warmup_inflight: modal.warmup_inflight,
            response_scroll,
            hint,
            input: &modal.input,
            input_border_style: ai_detail_input_border_style(modal),
            close_button_focused: modal.focus == AiDetailFocus::CloseButton,
            input_active_for_cursor: modal.focus == AiDetailFocus::Input && modal.input_active,
        },
    );
    input_rect.map(|rect| cursor_in_input(rect, &modal.input))
}

fn render_busy_modal(f: &mut ratatui::Frame, area: Rect, message: &str) {
    component::render_busy_modal(f, area, message);
}

fn render_alarm_modal(f: &mut ratatui::Frame, area: Rect, modal: &AlarmModal) {
    component::render_alarm_modal(f, area, &modal.message);
}

pub fn run_ui(
    projects: &mut Vec<ProjectRecord>,
    recent_active_pane: &mut Option<String>,
) -> Result<UiRunResult, String> {
    let palette = load_border_palette();
    let ids_changed = assign_missing_project_ids(projects);
    promote_recent_project_to_front(projects, recent_active_pane.as_deref());
    let mut app = UiApp {
        tab_index: 0,
        project_index: pick_selected_project_index(projects),
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
        alarm_modal: None,
        pending_action: None,
        busy_message: None,
        parallel_build_rx: None,
        menu_active: true,
        changed: ids_changed,
        pane_activate_started_at: None,
        pane_activate_index: 0,
        detail_layout: layout_load("code"),
    };
    if !projects.is_empty() {
        set_selected(projects, app.project_index);
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
    });

    'app_loop: loop {
        let _features = collect_feature_names(projects.get(app.project_index));

        if let Err(e) = terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(1),
                    Constraint::Length(4),
                ])
                .split(f.area());

            component::render_tab_header(
                f,
                chunks[0],
                app.tab_index,
                palette.active,
                palette.inactive,
                palette.normal,
                "switch : tab",
            );

            if app.tab_index == 0 {
                let overlay_modal = has_overlay_modal(&app);
                render_projects_tab(
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
                render_details_tab(
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
            let shared_help = if app.tab_index == 0 {
                "q: exit | tab: switch | a: init-project | l: load-preset | m: edit | d: delete"
            } else {
                "q: exit | tab: switch | m: edit | d: delete"
            };
            let modal_help = "tab: move field | type/backspace: edit | esc: close";
            let pane_shortcut = if app.menu_active && app.tab_index == 1 {
                selected_pane_shortcut(&app.detail_layout, app.tab_index, app.pane_focus)
            } else {
                String::new()
            };
            let pane_shortcut_text = if pane_shortcut.is_empty() {
                String::new()
            } else {
                format!(" | pane-shortcut: {}", pane_shortcut)
            };
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
                    "{} | plan: b create_code_draft{} | status: {} ({})",
                    shared_help, pane_shortcut_text, app.status_line, running
                )
            } else if app.menu_active && app.tab_index == 1 && app.pane_focus == 5 {
                let can_add_draft = projects
                    .get(app.project_index)
                    .map(|project| !collect_generated_draft_items_from_project(project).is_empty())
                    .unwrap_or(false);
                let draft_help = if can_add_draft {
                    "drafts(stage_draft): a add_draft, b enter_parallel"
                } else {
                    "drafts(stage_draft): b enter_parallel(빈 draft면 create_code_draft 선실행)"
                };
                format!(
                    "{} | {}{} | status: {} ({})",
                    shared_help, draft_help, pane_shortcut_text, app.status_line, running
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
                let modal_rect = centered_rect(70, 55, f.area());
                if let Some((x, y)) = render_create_modal(f, modal_rect, modal) {
                    f.set_cursor_position((x, y));
                }
            } else if let Some(confirm) = &app.path_change_confirm {
                let modal_rect = centered_rect(70, 35, f.area());
                render_path_change_confirm_modal(f, modal_rect, confirm);
            } else if let Some(confirm) = &app.delete_confirm {
                let modal_rect = centered_rect(70, 35, f.area());
                render_delete_confirm_modal(f, modal_rect, confirm);
            } else if let Some(modal) = &app.draft_bulk_add_modal {
                let modal_rect = centered_rect(82, 65, f.area());
                if let Some((x, y)) = render_draft_bulk_add_modal(f, modal_rect, modal) {
                    f.set_cursor_position((x, y));
                }
            } else if let Some(confirm) = &app.detail_fill_confirm {
                if let Some(project) = projects.get(confirm.project_index) {
                    let modal_rect = centered_rect(70, 35, f.area());
                    render_detail_fill_confirm_modal(f, modal_rect, project, confirm);
                }
            } else if let Some(modal) = &app.list_edit_modal {
                let modal_rect = centered_rect(92, 88, f.area());
                if let Some((x, y)) = render_list_edit_modal(f, modal_rect, modal) {
                    f.set_cursor_position((x, y));
                }
            } else if let Some(confirm) = &app.draft_create_confirm {
                let modal_rect = centered_rect(70, 35, f.area());
                render_draft_create_confirm_modal(f, modal_rect, confirm);
            } else if let Some(confirm) = &app.bootstrap_confirm {
                let modal_rect = centered_rect(70, 35, f.area());
                render_bootstrap_confirm_modal(f, modal_rect, confirm);
            } else if let Some(modal) = &app.ai_chat_modal {
                let modal_rect = centered_rect(85, 80, f.area());
                if let Some((x, y)) = render_ai_chat_modal(f, modal_rect, modal) {
                    f.set_cursor_position((x, y));
                }
            } else if let Some(modal) = &app.alarm_modal {
                let modal_rect = centered_rect(64, 28, f.area());
                render_alarm_modal(f, modal_rect, modal);
            }
            if let Some(message) = &app.busy_message {
                let modal_rect = centered_rect(55, 25, f.area());
                render_busy_modal(f, modal_rect, message);
            }
        }) {
            run_result = Err(format!("ui draw failed: {}", e));
            break 'app_loop;
        }

        if let Some(action) = app.pending_action.take() {
            let result = match action {
                PendingUiAction::SubmitProjectModal(modal) => {
                    if modal.mode == ProjectModalMode::Create {
                        apply_project_create(projects, &mut app, &modal)
                    } else {
                        try_submit_edit_project(projects, &mut app, &modal)
                    }
                }
                PendingUiAction::ApplyPathChange { confirm, move_dir } => {
                    apply_path_change_confirm(projects, &mut app, confirm, move_dir)
                }
                PendingUiAction::ApplyDelete { confirm, accepted } => {
                    apply_delete_confirm(projects, &mut app, confirm, accepted)
                }
                PendingUiAction::ApplyBootstrap { confirm } => {
                    apply_bootstrap(projects, &mut app, &confirm)
                }
                PendingUiAction::ApplyCreateDraft { project_index } => {
                    apply_draft_create_via_cli(projects, &mut app, project_index)
                }
                PendingUiAction::ApplyBuildParallel { project_index } => {
                    start_build_parallel_via_cli_async(projects, &mut app, project_index)
                }
                PendingUiAction::ApplyDraftBulkAdd {
                    project_index,
                    raw_input,
                } => apply_draft_bulk_add_via_cli(projects, &mut app, project_index, &raw_input),
            };
            app.busy_message = None;
            if let Err(e) = result {
                app.status_line = e;
            }
            continue;
        }

        if let Some(rx) = app.parallel_build_rx.as_ref() {
            match rx.try_recv() {
                Ok(Ok(msg)) => {
                    app.parallel_running = false;
                    app.parallel_build_rx = None;
                    for (_, state) in &mut app.parallel_statuses {
                        *state = TaskRuntimeState::Clear;
                    }
                    app.status_line = msg;
                }
                Ok(Err(e)) => {
                    app.parallel_running = false;
                    app.parallel_build_rx = None;
                    app.parallel_statuses.clear();
                    app.status_line = e;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    app.parallel_running = false;
                    app.parallel_build_rx = None;
                    app.parallel_statuses.clear();
                    app.status_line = "parallel build channel disconnected".to_string();
                }
            }
        }

        let mut auto_next_bootstrap: Option<usize> = None;
        if let Some(modal) = app.ai_chat_modal.as_mut() {
            if let Some(rx) = modal.stream_rx.as_ref() {
                loop {
                    match rx.try_recv() {
                        Ok(AiStreamEvent::Chunk(chunk)) => {
                            if !modal.warmup_inflight {
                                modal.streaming_buffer.push_str(&chunk);
                                if has_onboarding_done_signal(&modal.streaming_buffer) {
                                    let (spec_ready, domain_ready, feature_count) =
                                        collect_onboarding_signals(modal, "");
                                    if spec_ready && domain_ready && feature_count >= 3 {
                                        let raw_response = modal.streaming_buffer.trim().to_string();
                                        append_project_chat_log(
                                            &modal.project_path,
                                            "LLM_RESPONSE_RAW",
                                            &raw_response,
                                        );
                                        let response = strip_next_step_guidance(&raw_response);
                                        modal.history.push(format!("AI:\n{}", response));
                                        if let Some(cancel) = modal.stream_cancel.take() {
                                            cancel.store(true, Ordering::Relaxed);
                                        }
                                        modal.streaming = false;
                                        modal.streaming_buffer.clear();
                                        modal.stream_rx = None;
                                        modal.stream_cancel = None;
                                        match finalize_project_md_from_chat(modal) {
                                            Ok(()) => {
                                                app.status_line =
                                                    "onboarding finalized: .project/project.md + drafts_list.yaml".to_string();
                                                auto_next_bootstrap = Some(modal.project_index);
                                            }
                                            Err(e) => {
                                                app.status_line =
                                                    format!("onboarding finalize failed: {}", e);
                                            }
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                        Ok(AiStreamEvent::Done) => {
                            modal.streaming = false;
                            if modal.warmup_inflight {
                                modal.warmup_inflight = false;
                                modal.streaming_buffer.clear();
                                modal.stream_rx = None;
                                modal.stream_cancel = None;
                                append_project_chat_log(
                                    &modal.project_path,
                                    "LLM_WARMUP_DONE",
                                    "warmup completed",
                                );
                                app.status_line = "ai detail ready".to_string();
                                break;
                            }
                            let raw_response = modal.streaming_buffer.trim().to_string();
                            append_project_chat_log(
                                &modal.project_path,
                                "LLM_RESPONSE_RAW",
                                &raw_response,
                            );
                            let response = if modal.mode == AiChatMode::DetailProject
                                && !modal.allow_full_md_response
                                && is_project_md_dump(&raw_response)
                            {
                                "전체 project.md 출력이 감지되어 화면 표시를 제한했습니다.\n필요하면 `project.md 전체 업데이트`라고 입력해 주세요."
                                    .to_string()
                            } else if modal.mode == AiChatMode::DetailProject {
                                strip_next_step_guidance(&raw_response)
                            } else {
                                raw_response.clone()
                            };
                            modal.history.push(format!("AI:\n{}", response));
                            match modal.mode {
                                AiChatMode::DetailProject => {
                                    let blocked_full_dump = !modal.allow_full_md_response
                                        && is_project_md_dump(&raw_response);
                                    if blocked_full_dump {
                                        app.status_line =
                                            "project.md 전체 출력 응답은 적용하지 않았습니다".to_string();
                                        modal.streaming_buffer.clear();
                                        modal.stream_rx = None;
                                        modal.stream_cancel = None;
                                        break;
                                    }
                                    if let Some(md) = extract_markdown_block(&raw_response) {
                                        let root = Path::new(&modal.project_path);
                                        match validate_project_md_format(&md) {
                                            Ok(()) => {
                                                if write_project_md_with_sync(root, &md).is_ok() {
                                                    let _ = crate::sync_project_tasks_list_from_project_md(root);
                                                    app.status_line =
                                                        "ai response applied: .project/project.md + drafts_list.yaml".to_string();
                                                }
                                            }
                                            Err(reason) => {
                                                app.status_line = format!(
                                                    "project.md format check failed: {}",
                                                    reason
                                                );
                                            }
                                        }
                                    } else if has_onboarding_done_signal(&raw_response) {
                                        let (spec_ready, domain_ready, feature_count) =
                                            collect_onboarding_signals(modal, "");
                                        if spec_ready && domain_ready && feature_count >= 3 {
                                            match finalize_project_md_from_chat(modal) {
                                                Ok(()) => {
                                                    app.status_line =
                                                        "onboarding finalized: .project/project.md + drafts_list.yaml".to_string();
                                                    auto_next_bootstrap = Some(modal.project_index);
                                                }
                                                Err(e) => {
                                                    app.status_line = format!(
                                                        "onboarding finalize failed: {}",
                                                        e
                                                    );
                                                }
                                            }
                                        } else {
                                            app.status_line = format!(
                                                "onboarding not ready: spec={} domain={} features={}",
                                                if spec_ready { "ok" } else { "missing" },
                                                if domain_ready { "ok" } else { "missing" },
                                                feature_count
                                            );
                                        }
                                    }
                                }
                                AiChatMode::AddPlan => {
                                    if modal.add_plan_apply_requested {
                                        match apply_add_plan_update_from_yaml(modal, &raw_response) {
                                            Ok(Some(msg)) => {
                                                app.status_line = msg;
                                            }
                                            Ok(None) => {
                                                app.status_line =
                                                    "add_code_plan 적용 요청이었지만 유효한 update 블록이 없습니다".to_string();
                                            }
                                            Err(e) => {
                                                app.status_line = e;
                                            }
                                        }
                                    } else {
                                        app.status_line = "add_code_plan 추천안 응답 수신".to_string();
                                    }
                                    modal.add_plan_apply_requested = false;
                                }
                            }
                            modal.streaming_buffer.clear();
                            modal.stream_rx = None;
                            modal.stream_cancel = None;
                            break;
                        }
                        Ok(AiStreamEvent::Error(err)) => {
                            modal.streaming = false;
                            modal.add_plan_apply_requested = false;
                            append_project_chat_log(
                                &modal.project_path,
                                "LLM_ERROR",
                                &err,
                            );
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
                            modal.add_plan_apply_requested = false;
                            append_project_chat_log(
                                &modal.project_path,
                                "LLM_CANCELLED",
                                "cancelled by user",
                            );
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
        if let Some(project_index) = auto_next_bootstrap {
            close_ai_chat_modal_and_open_bootstrap(&mut app, projects, project_index);
            continue;
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
                                    && normalize_feature_item(&item).is_err()
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
                            save_project_md_list(
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
                            app.pending_action = Some(PendingUiAction::ApplyBootstrap { confirm });
                            app.busy_message = Some("bootstrap preset + spec 기준 LLM 준비/초기화 실행 중".to_string());
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
                            if modal.mode == AiChatMode::DetailProject {
                                let idx = modal.project_index;
                                close_ai_chat_modal_and_open_bootstrap(&mut app, projects, idx);
                            } else {
                                cancel_ai_stream(&mut app);
                                app.ai_chat_modal = None;
                                app.status_line = "ai add_code_plan closed".to_string();
                            }
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
                            modal.allow_full_md_response = is_full_project_md_request(&msg);
                            if modal.mode == AiChatMode::AddPlan {
                                modal.add_plan_apply_requested = is_add_plan_apply_request(&msg);
                            }
                            let user_line = format!("You:\n{}", msg);
                            modal.history.push(user_line.clone());
                            let prompt = build_ai_chat_prompt(modal, &msg);
                            append_project_chat_log(&modal.project_path, "USER_MESSAGE", &msg);
                            append_project_chat_log(&modal.project_path, "LLM_PROMPT", &prompt);
                            modal.streaming = true;
                            modal.streaming_buffer.clear();
                            let (rx, cancel) = spawn_ai_stream(&modal.model_bin, prompt);
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
                            open_ai_chat_modal(&mut app, projects, confirm.project_index);
                        } else {
                            app.status_line = "skip detail fill".to_string();
                            open_bootstrap_confirm(&mut app, projects, confirm.project_index);
                        }
                    }
                    KeyCode::Char('y') | KeyCode::Char('Y') if confirm.confirm_selected => {
                        open_ai_chat_modal(&mut app, projects, confirm.project_index);
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        app.status_line = "skip detail fill".to_string();
                        open_bootstrap_confirm(&mut app, projects, confirm.project_index);
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
                            apply_delete_confirm(projects, &mut app, confirm, false)?;
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        apply_delete_confirm(projects, &mut app, confirm, false)?;
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
                            apply_path_change_confirm(projects, &mut app, confirm, false)?;
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        apply_path_change_confirm(projects, &mut app, confirm, false)?;
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
                let _ = handle_modal_input(projects, &mut app, key_event.code)?;
                continue;
            }
            if app.alarm_modal.is_some() {
                if matches!(key_event.code, KeyCode::Enter) {
                    app.alarm_modal = None;
                }
                continue;
            }
            match key_event.code {
                KeyCode::Char('q') => {
                    if app.menu_active {
                        app.menu_active = false;
                        app.status_line = "focus closed (inactive)".to_string();
                    } else {
                        cancel_ai_stream(&mut app);
                        break 'app_loop;
                    }
                }
                KeyCode::Enter => {
                    if !app.menu_active {
                        app.menu_active = true;
                        app.status_line = "focus active".to_string();
                        start_pane_activate_tween(&mut app);
                    } else if app.tab_index == 0 {
                        app.tab_index = 1;
                        if let Some(project) = projects.get(app.project_index) {
                            *recent_active_pane = Some(project.id.clone());
                            app.changed = true;
                        }
                        app.status_line = "moved to detail tab".to_string();
                        start_pane_activate_tween(&mut app);
                    } else if app.tab_index == 1 && app.pane_focus == 1 {
                        open_list_edit_modal(&mut app, projects, ListEditTarget::Rule);
                    } else if app.tab_index == 1 && app.pane_focus == 2 {
                        open_list_edit_modal(
                            &mut app,
                            projects,
                            ListEditTarget::Constraint,
                        );
                    } else if app.tab_index == 1 && app.pane_focus == 3 {
                        open_list_edit_modal(&mut app, projects, ListEditTarget::Feature);
                    } else if app.tab_index == 1 && (app.pane_focus == 4 || app.pane_focus == 5) {
                        if let Some(project) = projects.get(app.project_index) {
                            let planned = collect_planned_drafts_from_project(project);
                            if app.pane_focus == 4 {
                                if planned.is_empty() {
                                    let project_index = app.project_index;
                                    open_draft_bulk_add_modal(&mut app, project_index);
                                } else {
                                    let project_index = app.project_index;
                                    open_draft_create_confirm(&mut app, project_index);
                                }
                            } else {
                                let generated = collect_generated_draft_items_from_project(project);
                                if generated.is_empty() {
                                    app.pending_action = Some(PendingUiAction::ApplyCreateDraft {
                                        project_index: app.project_index,
                                    });
                                    app.busy_message =
                                        Some("enter_draft 실행: create_code_draft 요청 중".to_string());
                                } else if !planned.is_empty()
                                    && !all_planned_task_files_exist(project, &planned)
                                {
                                    app.pending_action = Some(PendingUiAction::ApplyCreateDraft {
                                        project_index: app.project_index,
                                    });
                                    app.busy_message = Some(
                                        "planned 항목 파일 누락 감지: create_code_draft 보정 실행 중".to_string(),
                                    );
                                } else {
                                    let project_index = app.project_index;
                                    if let Err(e) = start_build_parallel_via_cli_async(
                                        projects,
                                        &mut app,
                                        project_index,
                                    ) {
                                        app.status_line = e;
                                    }
                                }
                            }
                        } else {
                            app.status_line = "no selected project".to_string();
                        }
                    } else {
                        app.status_line = "focus active".to_string();
                        start_pane_activate_tween(&mut app);
                    }
                }
                KeyCode::Char('a') if app.menu_active && app.tab_index == 0 => {
                    open_create_modal(&mut app)
                }
                KeyCode::Char('l') if app.menu_active && app.tab_index == 0 => {
                    if app.create_modal.is_none() {
                        open_create_modal(&mut app);
                    }
                    apply_first_project_preset_to_create_modal(&mut app);
                }
                KeyCode::Char('a')
                    if app.menu_active && app.tab_index == 1 && app.pane_focus == 5 =>
                {
                    if let Some(project) = projects.get(app.project_index) {
                        let generated = collect_generated_draft_items_from_project(project);
                        if generated.is_empty() {
                            app.status_line = "add_draft requires active draft items".to_string();
                        } else {
                            let project_index = app.project_index;
                            open_draft_bulk_add_modal(&mut app, project_index);
                        }
                    } else {
                        app.status_line = "no selected project".to_string();
                    }
                }
                KeyCode::Char('m') if app.menu_active && app.tab_index == 0 => {
                    open_edit_modal(&mut app, projects);
                }
                KeyCode::Char('d') if app.menu_active && app.tab_index == 0 => {
                    open_delete_confirm(&mut app, projects);
                }
                KeyCode::Char('b')
                    if app.menu_active
                        && app.tab_index == 1
                        && (app.pane_focus == 4 || app.pane_focus == 5) =>
                {
                    if let Some(project) = projects.get(app.project_index) {
                        let planned = collect_planned_drafts_from_project(project);
                        if app.pane_focus == 4 {
                            if planned.is_empty() {
                                let project_index = app.project_index;
                                open_draft_bulk_add_modal(&mut app, project_index);
                            } else {
                                let project_index = app.project_index;
                                open_draft_create_confirm(&mut app, project_index);
                            }
                        } else {
                            let generated = collect_generated_draft_items_from_project(project);
                            if generated.is_empty() {
                                app.pending_action = Some(PendingUiAction::ApplyCreateDraft {
                                    project_index: app.project_index,
                                });
                                app.busy_message =
                                    Some("enter_draft 실행: create_code_draft 요청 중".to_string());
                            } else if !planned.is_empty()
                                && !all_planned_task_files_exist(project, &planned)
                            {
                                app.pending_action = Some(PendingUiAction::ApplyCreateDraft {
                                    project_index: app.project_index,
                                });
                                app.busy_message = Some(
                                    "planned 항목 파일 누락 감지: create_code_draft 보정 실행 중".to_string(),
                                );
                            } else {
                                let project_index = app.project_index;
                                if let Err(e) = start_build_parallel_via_cli_async(
                                    projects,
                                    &mut app,
                                    project_index,
                                ) {
                                    app.status_line = e;
                                }
                            }
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
                    if let Err(e) = save_projects_to_registry(projects, recent_active_pane) {
                        app.status_line = e;
                        continue;
                    }
                    if let Err(e) = reload_projects_from_registry(
                        projects,
                        recent_active_pane,
                        &mut app,
                    ) {
                        app.status_line = e;
                        continue;
                    }
                    app.status_line = format!("tab changed to {}", app.tab_index + 1);
                }
                KeyCode::Char('1') if app.menu_active => {
                    app.tab_index = 0;
                    if let Err(e) = save_projects_to_registry(projects, recent_active_pane) {
                        app.status_line = e;
                        continue;
                    }
                    if let Err(e) = reload_projects_from_registry(
                        projects,
                        recent_active_pane,
                        &mut app,
                    ) {
                        app.status_line = e;
                    }
                }
                KeyCode::Char('2') if app.menu_active => {
                    app.tab_index = 1;
                    if let Err(e) = save_projects_to_registry(projects, recent_active_pane) {
                        app.status_line = e;
                        continue;
                    }
                    if let Err(e) = reload_projects_from_registry(
                        projects,
                        recent_active_pane,
                        &mut app,
                    ) {
                        app.status_line = e;
                    }
                }
                KeyCode::Char('k') => move_project_grid_selection(projects, &mut app, -3),
                KeyCode::Char('j') => move_project_grid_selection(projects, &mut app, 3),
                KeyCode::Up if app.tab_index == 0 => {
                    move_project_grid_selection(projects, &mut app, -3);
                }
                KeyCode::Down if app.tab_index == 0 => {
                    move_project_grid_selection(projects, &mut app, 3);
                }
                KeyCode::Left if app.tab_index == 0 => {
                    move_project_grid_selection(projects, &mut app, -1);
                }
                KeyCode::Right if app.tab_index == 0 => {
                    move_project_grid_selection(projects, &mut app, 1);
                }
                KeyCode::Left if app.tab_index == 1 => move_detail_pane_focus(&mut app, KeyCode::Left),
                KeyCode::Right if app.tab_index == 1 => move_detail_pane_focus(&mut app, KeyCode::Right),
                KeyCode::Up if app.tab_index == 1 => move_detail_pane_focus(&mut app, KeyCode::Up),
                KeyCode::Down if app.tab_index == 1 => move_detail_pane_focus(&mut app, KeyCode::Down),
                _ => {}
            }
        }
    }

    cancel_ai_stream(&mut app);

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
        });
    }
    run_result
}
