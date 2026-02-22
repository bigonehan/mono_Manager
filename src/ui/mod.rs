use crate::ProjectRecord;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap};
use ratatui::Terminal;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub fn render_project_list(projects: &[ProjectRecord]) -> String {
    let mut out = String::new();
    out.push_str("name\tcreated_at\tupdated_at\tdescription\tselected\n");
    for p in projects {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\n",
            p.name,
            p.created_at,
            p.updated_at,
            p.description,
            if p.selected { "yes" } else { "no" }
        ));
    }
    out
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
pub enum Pane {
    Project,
    Draft,
    Task,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneState {
    Inactive,
    Active,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskRuntimeState {
    Inactive,
    Active,
    Clear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiState {
    pub focused: Pane,
    pub pane_state: PaneState,
    pub list_layer_active: bool,
}

pub fn calc_next_focus(current: Pane, direction: &str) -> Pane {
    match (current, direction) {
        (Pane::Project, "right") => Pane::Draft,
        (Pane::Draft, "right") => Pane::Task,
        (Pane::Task, "right") => Pane::Project,
        (Pane::Project, "left") => Pane::Task,
        (Pane::Draft, "left") => Pane::Project,
        (Pane::Task, "left") => Pane::Draft,
        _ => current,
    }
}

pub fn flow_apply_key(state: UiState, key: &str) -> UiState {
    match key {
        "left" => UiState {
            focused: calc_next_focus(state.focused, "left"),
            ..state
        },
        "right" => UiState {
            focused: calc_next_focus(state.focused, "right"),
            ..state
        },
        "enter" => UiState {
            pane_state: PaneState::Active,
            ..state
        },
        "esc" if state.list_layer_active => UiState {
            list_layer_active: false,
            pane_state: PaneState::Active,
            ..state
        },
        "esc" if state.pane_state == PaneState::Active => UiState {
            pane_state: PaneState::Inactive,
            ..state
        },
        _ => state,
    }
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
            let template = fs::read_to_string("src/assets/templates/draft.yaml")
                .map_err(|e| format!("failed to read draft template: {}", e))?;
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
    inactive: Option<PaneStyleValue>,
}

#[derive(Debug, Clone, Copy)]
struct BorderPalette {
    active: Color,
    inactive: Color,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct DraftsListDoc {
    #[serde(default)]
    feature: Vec<String>,
    #[serde(default)]
    planned: Vec<String>,
}

#[derive(Debug, Clone)]
struct CreateProjectModal {
    name: String,
    description: String,
    path: String,
    field_index: usize,
    confirm_selected: bool,
}

#[derive(Debug, Clone)]
struct ConfirmPane {
    confirm_label: String,
    cancel_label: String,
    selected_confirm: bool,
}

#[derive(Debug, Clone)]
struct UiApp {
    tab_index: usize,
    project_index: usize,
    pane_focus: usize,
    parallel_statuses: Vec<(String, TaskRuntimeState)>,
    parallel_running: bool,
    last_tick: Instant,
    status_line: String,
    create_modal: Option<CreateProjectModal>,
    menu_active: bool,
    changed: bool,
    pane_activate_started_at: Option<Instant>,
    pane_activate_index: usize,
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
    let candidates = [
        PathBuf::from("configs").join("style.yaml"),
        PathBuf::from("src")
            .join("assets")
            .join("style")
            .join("pane_style.yaml"),
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
                let inactive = calc_parse_color(
                    doc.inactive.as_ref().and_then(|v| v.border.as_deref()),
                    Color::DarkGray,
                );
                return BorderPalette { active, inactive };
            }
        }
    }

    BorderPalette {
        active: Color::Green,
        inactive: Color::DarkGray,
    }
}

fn calc_now_unix() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_string()
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
        palette.inactive
    };
    Style::default().fg(color)
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
    let Some(progress) = calc_active_pane_tween_progress(app, pane_index) else {
        return Style::default().fg(palette.inactive);
    };
    Style::default().fg(calc_lerp_color(palette.inactive, palette.active, progress))
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

fn action_advance_parallel_runtime(app: &mut UiApp) {
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
    app.status_line = "parallel runtime finished".to_string();
}

fn calc_default_project_name_from_parent() -> String {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    cwd.parent()
        .and_then(|p| p.file_name())
        .and_then(|v| v.to_str())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "new-project".to_string())
}

fn action_open_create_modal(app: &mut UiApp) {
    let default_path = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| ".".to_string());
    app.create_modal = Some(CreateProjectModal {
        name: calc_default_project_name_from_parent(),
        description: String::new(),
        path: default_path,
        field_index: 0,
        confirm_selected: true,
    });
    app.status_line = "create project modal opened".to_string();
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
    let raw_path = modal.path.trim();
    let mut path = if raw_path.is_empty() {
        PathBuf::from(".")
    } else {
        PathBuf::from(raw_path)
    };
    if path.is_relative() {
        path = std::env::current_dir()
            .map_err(|e| format!("failed to read current dir: {}", e))?
            .join(path);
    }

    fs::create_dir_all(&path)
        .map_err(|e| format!("failed to create project dir {}: {}", path.display(), e))?;
    fs::create_dir_all(path.join(".project"))
        .map_err(|e| format!("failed to create project meta dir: {}", e))?;

    let now = calc_now_unix();
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
        projects.push(ProjectRecord {
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
    Ok(())
}

fn action_render_projects_tab(
    f: &mut ratatui::Frame,
    area: Rect,
    projects: &[ProjectRecord],
    selected_index: usize,
    active: bool,
    palette: BorderPalette,
) {
    let rows: Vec<Row> = projects
        .iter()
        .enumerate()
        .map(|(idx, p)| {
            let style = if idx == selected_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(p.name.clone()),
                Cell::from(p.path.clone()),
                Cell::from(p.description.clone()),
                Cell::from(if p.selected {
                    "yes".to_string()
                } else {
                    "no".to_string()
                }),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(18),
            Constraint::Percentage(45),
            Constraint::Percentage(30),
            Constraint::Length(10),
        ],
    )
    .header(
        Row::new(vec!["Name", "Path", "Description", "Selected"])
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .title("Project Select")
            .borders(Borders::ALL)
            .border_style(calc_pane_border_style(active, palette)),
    );
    f.render_widget(table, area);
}

fn action_render_details_tab(
    f: &mut ratatui::Frame,
    area: Rect,
    app: &UiApp,
    projects: &[ProjectRecord],
    features: &[String],
    _menu_active: bool,
    palette: BorderPalette,
) {
    let pane_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(area);

    let selected_project = projects.get(app.project_index);
    let project_lines: Vec<Line> = match selected_project {
        Some(project) => vec![
            Line::from(format!("name: {}", project.name)),
            Line::from(format!("path: {}", project.path)),
            Line::from(format!("description: {}", project.description)),
            Line::from(format!("created_at: {}", project.created_at)),
            Line::from(format!("updated_at: {}", project.updated_at)),
        ],
        None => vec![Line::from("no selected project")],
    };

    let project_area = calc_inset_rect(pane_layout[0], calc_active_pane_margin(app, 0));
    let project_block = Block::default()
        .title("Project Pane")
        .borders(Borders::ALL)
        .border_style(calc_tweened_pane_border_style(app, 0, palette));
    f.render_widget(
        Paragraph::new(project_lines)
            .block(project_block)
            .wrap(Wrap { trim: false }),
        project_area,
    );

    let draft_lines: Vec<Line> = if features.is_empty() {
        vec![Line::from("no feature in drafts_list.yaml")]
    } else {
        features
            .iter()
            .enumerate()
            .map(|(idx, name)| Line::from(format!("{}. {}", idx + 1, name)))
            .collect()
    };

    let draft_area = calc_inset_rect(pane_layout[1], calc_active_pane_margin(app, 1));
    let draft_block = Block::default()
        .title("Draft Features")
        .borders(Borders::ALL)
        .border_style(calc_tweened_pane_border_style(app, 1, palette));
    f.render_widget(
        Paragraph::new(draft_lines)
            .block(draft_block)
            .wrap(Wrap { trim: false }),
        draft_area,
    );

    let runtime_lines: Vec<Line> = if app.parallel_statuses.is_empty() {
        vec![Line::from("press p to start parallel runtime simulation")]
    } else {
        app.parallel_statuses
            .iter()
            .map(|(name, state)| Line::from(render_task_runtime_status(name, *state)))
            .collect()
    };
    let runtime_area = calc_inset_rect(pane_layout[2], calc_active_pane_margin(app, 2));
    let runtime_block = Block::default()
        .title("Parallel Runtime")
        .borders(Borders::ALL)
        .border_style(calc_tweened_pane_border_style(app, 2, palette));
    f.render_widget(
        Paragraph::new(runtime_lines)
            .block(runtime_block)
            .wrap(Wrap { trim: false }),
        runtime_area,
    );
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

fn action_render_confirm_pane(
    f: &mut ratatui::Frame,
    area: Rect,
    title: &str,
    body: &[Line],
    confirm: &ConfirmPane,
    palette: BorderPalette,
) {
    f.render_widget(Clear, area);
    let button_style = |active: bool| {
        if active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        }
    };

    let mut lines = body.to_vec();
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        " [".into(),
        confirm.confirm_label.clone().into(),
        "] ".into(),
        " [".into(),
        confirm.cancel_label.clone().into(),
        "]".into(),
    ]));

    let pane = Paragraph::new(lines)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(palette.active)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(pane, area);

    let button_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area)[1];
    let buttons = Line::from(vec![
        " ".into(),
        ratatui::text::Span::styled(
            format!("[{}]", confirm.confirm_label),
            button_style(confirm.selected_confirm),
        ),
        "  ".into(),
        ratatui::text::Span::styled(
            format!("[{}]", confirm.cancel_label),
            button_style(!confirm.selected_confirm),
        ),
    ]);
    f.render_widget(Paragraph::new(buttons), button_area);
}

fn action_render_create_modal(
    f: &mut ratatui::Frame,
    area: Rect,
    modal: &CreateProjectModal,
    palette: BorderPalette,
) {
    let field_style = |active: bool| {
        if active {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        }
    };
    let body = vec![
        Line::from(vec![
            ratatui::text::Span::styled("name: ", field_style(modal.field_index == 0)),
            modal.name.clone().into(),
            "  (default: parent folder)".into(),
        ]),
        Line::from(vec![
            ratatui::text::Span::styled("description: ", field_style(modal.field_index == 1)),
            modal.description.clone().into(),
        ]),
        Line::from(vec![
            ratatui::text::Span::styled("project path: ", field_style(modal.field_index == 2)),
            modal.path.clone().into(),
            "  (default: current dir)".into(),
        ]),
        Line::from("tab: next field | backspace: edit | enter: apply"),
    ];
    let confirm = ConfirmPane {
        confirm_label: "Confirm".to_string(),
        cancel_label: "Cancel".to_string(),
        selected_confirm: modal.confirm_selected,
    };
    action_render_confirm_pane(f, area, "Create Project", &body, &confirm, palette);
}

fn action_handle_modal_input(
    projects: &mut Vec<ProjectRecord>,
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
            modal.field_index = (modal.field_index + 1) % 4;
        }
        KeyCode::Up => {
            modal.field_index = if modal.field_index == 0 {
                3
            } else {
                modal.field_index - 1
            };
        }
        KeyCode::Left | KeyCode::Right if modal.field_index == 3 => {
            modal.confirm_selected = !modal.confirm_selected;
        }
        KeyCode::Backspace if modal.field_index == 0 => {
            modal.name.pop();
        }
        KeyCode::Backspace if modal.field_index == 1 => {
            modal.description.pop();
        }
        KeyCode::Backspace if modal.field_index == 2 => {
            modal.path.pop();
        }
        KeyCode::Char(c) if modal.field_index == 0 => modal.name.push(c),
        KeyCode::Char(c) if modal.field_index == 1 => modal.description.push(c),
        KeyCode::Char(c) if modal.field_index == 2 => modal.path.push(c),
        KeyCode::Enter if modal.field_index < 3 => {
            modal.field_index += 1;
        }
        KeyCode::Enter => {
            if modal.confirm_selected {
                action_apply_project_create(projects, app, &modal)?;
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

pub fn flow_run_ui(projects: &mut Vec<ProjectRecord>) -> Result<UiRunResult, String> {
    let palette = action_load_border_palette();
    let mut app = UiApp {
        tab_index: 0,
        project_index: action_pick_selected_project_index(projects),
        pane_focus: 0,
        parallel_statuses: Vec::new(),
        parallel_running: false,
        last_tick: Instant::now(),
        status_line: "ready".to_string(),
        create_modal: None,
        menu_active: true,
        changed: false,
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
        let features = action_collect_feature_names(projects.get(app.project_index));

        if let Err(e) = terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(1),
                    Constraint::Length(3),
                ])
                .split(f.area());

            let header = Line::from(vec![
                "Pane: ".into(),
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
            let header_widget = Paragraph::new(header).block(
                Block::default()
                    .title("Current Pane")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(palette.active)),
            );
            f.render_widget(header_widget, chunks[0]);

            if app.tab_index == 0 {
                action_render_projects_tab(
                    f,
                    chunks[1],
                    projects,
                    app.project_index,
                    app.menu_active,
                    palette,
                );
            } else {
                action_render_details_tab(
                    f,
                    chunks[1],
                    &app,
                    projects,
                    &features,
                    app.menu_active,
                    palette,
                );
            }

            let running = if app.parallel_running { "running" } else { "idle" };
            let footer = if app.create_modal.is_some() {
                "modal: tab/up/down move | type edit | left/right toggle confirm | enter submit | esc cancel".to_string()
            } else {
                format!(
                    "commands: q close-focus/exit | enter activate | tab/1/2 tabs | j/k project | left/right pane(detail) | p run | a add-project | m auto-mode(project) | status: {} ({})",
                    app.status_line, running
                )
            };
            let footer_widget = Paragraph::new(footer).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(palette.inactive)),
            );
            f.render_widget(footer_widget, chunks[2]);

            if let Some(modal) = &app.create_modal {
                let modal_rect = calc_centered_rect(70, 55, f.area());
                action_render_create_modal(f, modal_rect, modal, palette);
            }
        }) {
            run_result = Err(format!("ui draw failed: {}", e));
            break 'app_loop;
        }

        if app.parallel_running && app.last_tick.elapsed() >= Duration::from_millis(350) {
            action_advance_parallel_runtime(&mut app);
            app.last_tick = Instant::now();
        }

        let has_event =
            event::poll(Duration::from_millis(80)).map_err(|e| format!("ui event poll failed: {}", e))?;
        if !has_event {
            continue;
        }

        let ev = event::read().map_err(|e| format!("ui event read failed: {}", e))?;
        if let Event::Key(key_event) = ev {
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
                        break 'app_loop;
                    }
                }
                KeyCode::Enter => {
                    app.menu_active = true;
                    app.status_line = "focus active".to_string();
                    action_start_pane_activate_tween(&mut app);
                }
                KeyCode::Char('a') if app.menu_active => action_open_create_modal(&mut app),
                KeyCode::Char('m') if app.menu_active && app.tab_index == 0 => {
                    if let Some(project) = projects.get(app.project_index) {
                        run_result = Ok(UiRunResult {
                            changed: app.changed,
                            message: format!("auto mode requested for {}", project.name),
                            auto_mode_project: Some(project.name.clone()),
                        });
                        break 'app_loop;
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
                KeyCode::Char('k') => {
                    if !app.menu_active {
                        continue;
                    }
                    if app.project_index > 0 {
                        app.project_index -= 1;
                        action_set_selected(projects, app.project_index);
                        app.changed = true;
                        action_reset_parallel_runtime(&mut app);
                        app.status_line = "selected previous project".to_string();
                    }
                }
                KeyCode::Char('j') => {
                    if !app.menu_active {
                        continue;
                    }
                    if app.project_index + 1 < projects.len() {
                        app.project_index += 1;
                        action_set_selected(projects, app.project_index);
                        app.changed = true;
                        action_reset_parallel_runtime(&mut app);
                        app.status_line = "selected next project".to_string();
                    }
                }
                KeyCode::Left if app.tab_index == 1 => {
                    if !app.menu_active {
                        continue;
                    }
                    app.pane_focus = if app.pane_focus == 0 {
                        2
                    } else {
                        app.pane_focus - 1
                    };
                    action_start_pane_activate_tween(&mut app);
                }
                KeyCode::Right if app.tab_index == 1 => {
                    if !app.menu_active {
                        continue;
                    }
                    app.pane_focus = (app.pane_focus + 1) % 3;
                    action_start_pane_activate_tween(&mut app);
                }
                KeyCode::Char('p') if app.tab_index == 1 => {
                    if !app.menu_active {
                        continue;
                    }
                    action_start_parallel_runtime(&mut app, &features);
                }
                _ => {}
            }
        }
    }

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
