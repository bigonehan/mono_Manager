use std::time::Duration;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use anyhow::Result;
use crossterm::event::DisableMouseCapture;
use crossterm::event::EnableMouseCapture;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedReceiver;

#[derive(Debug, Clone)]
pub enum WorkingPaneEvent {
    SetRunning { worker_id: usize },
    SetDone { worker_id: usize, result: String },
    Finish,
}

#[derive(Debug, Clone, Copy)]
enum WorkingStatus {
    Ready,
    Running,
    Done,
}

#[derive(Debug, Clone)]
struct WorkingRow {
    request: String,
    result: String,
    status: WorkingStatus,
}

#[derive(Debug, Clone, Deserialize)]
struct StyleConfig {
    basic: BasicStyle,
    layout: LayoutStyle,
    symbol: SymbolStyle,
}

#[derive(Debug, Clone, Deserialize)]
struct BasicStyle {
    primary: String,
    secondary: String,
<<<<<<< HEAD
=======
    #[serde(default)]
    active: String,
    #[serde(default)]
    inactive: String,
    #[serde(default)]
    active: String,
    #[serde(default)]
    inactive: String,
    #[serde(default)]
    focus: String,
>>>>>>> 5b2a204 (fix: seperate process)
    background: String,
}

#[derive(Debug, Clone, Deserialize)]
struct LayoutStyle {
    margin: u16,
    padding: u16,
}

#[derive(Debug, Clone, Deserialize)]
struct SymbolStyle {
    state: StateSymbolStyle,
}

#[derive(Debug, Clone, Deserialize)]
struct StateSymbolStyle {
    ready: String,
    running: String,
    done: String,
}

#[derive(Debug, Clone)]
struct WorkingTheme {
    primary: Color,
    secondary: Color,
<<<<<<< HEAD
=======
    active: Color,
    inactive: Color,
    focus: Color,
>>>>>>> 5b2a204 (fix: seperate process)
    background: Color,
    margin: u16,
    padding: u16,
    state_ready: String,
    state_running: String,
    state_done: String,
}

#[derive(Debug, Clone, Copy)]
enum PaneFocus {
    Project,
    TaskSpec,
    Todos,
    Working,
}

#[derive(Debug, Clone, Copy)]
enum RequestButton {
    Cancel,
    Confirm,
}

#[derive(Debug, Clone, Copy)]
enum RequestPaneFocus {
    Input,
    Buttons,
}

#[derive(Debug, Clone)]
struct RequestInputPane {
    open: bool,
    text: String,
    input_scroll: u16,
    focus: RequestPaneFocus,
    selected_button: RequestButton,
}

#[derive(Debug, Clone)]
struct MakeTodosProgressPane {
    open: bool,
    running: bool,
    kind: BackgroundJobKind,
    lines: Vec<String>,
}

#[derive(Debug, Clone)]
struct PlanChatPane {
    open: bool,
    running: bool,
    input_text: String,
    output_scroll: u16,
    lines: Vec<String>,
    history: Vec<PlanChatTurn>,
    project_path: std::path::PathBuf,
    tasks_path: std::path::PathBuf,
    plan_path: std::path::PathBuf,
}

struct MakeTodosJob {
    rx: Receiver<MakeTodosEvent>,
}

struct PlanChatJob {
    rx: Receiver<PlanChatEvent>,
}

struct PlanWatchJob {
    rx: Receiver<PlanWatchEvent>,
}

struct PlanChatJob {
    rx: Receiver<PlanChatEvent>,
}

struct PlanWatchJob {
    rx: Receiver<PlanWatchEvent>,
}

enum MakeTodosEvent {
    Progress(String),
    Finished(Result<MakeTodosOutput, String>),
}

<<<<<<< HEAD
=======
enum PlanChatEvent {
    Finished(Result<PlanChatTurnOutput, String>),
}

enum PlanWatchEvent {
    Ready(Result<usize, String>),
}

#[derive(Debug, Clone)]
struct PlanChatTurn {
    role: String,
    content: String,
}

#[derive(Debug, Clone)]
struct PlanChatTurnOutput {
    reply: String,
    plan_md: String,
}

#[derive(Debug, Clone, Copy)]
enum BackgroundJobKind {
    MakeTodos,
    FillProjectTasks,
}

>>>>>>> 5b2a204 (fix: seperate process)
static CODEX_OUTPUT_SEQ: AtomicU64 = AtomicU64::new(0);

struct MakeTodosOutput {
    updated_spec: TaskSpecYaml,
    generated_todos: Vec<TaskSpecItem>,
}

#[derive(Debug, Clone, Copy)]
enum TaskSpecMode {
    List,
    Form,
}

#[derive(Debug, Clone, Copy)]
enum TaskListFocus {
    Pane,
    Item,
}

#[derive(Debug, Clone)]
struct PaneTaskSpec {
    path: std::path::PathBuf,
    todos_path: std::path::PathBuf,
    spec: TaskSpecYaml,
    todos: Vec<TaskSpecItem>,
    selected_task: usize,
    list_focus: TaskListFocus,
    mode: TaskSpecMode,
    selected_field: usize,
    input_mode: bool,
    input_buffer: String,
    status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TaskSpecYaml {
    #[serde(default)]
    name: String,
    #[serde(default)]
    framework: String,
    #[serde(default)]
    rule: Vec<String>,
    #[serde(default)]
    features: ProjectFeatures,
    #[serde(default)]
    tasks: Vec<TaskSpecItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ProjectFeatures {
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    domain: Vec<String>,
    #[serde(default)]
    feature: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TaskSpecItem {
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

pub async fn stage_run_working_pane(
    worker_requests: Vec<String>,
    task_spec_path: std::path::PathBuf,
    mut rx: UnboundedReceiver<WorkingPaneEvent>,
    run_start_tx: tokio::sync::mpsc::UnboundedSender<()>,
) -> Result<()> {
    let theme = load_working_theme();
    let mut focus = PaneFocus::Project;
    let mut rows = worker_requests
        .into_iter()
        .map(|request| WorkingRow {
            request,
            result: String::new(),
            status: WorkingStatus::Ready,
        })
        .collect::<Vec<_>>();
    let mut pane_task_spec = load_pane_task_spec(task_spec_path);

    let mut stdout = std::io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut should_finish = false;
    let mut quit_requested = false;
    let mut run_requested = false;
    let mut request_input_pane = RequestInputPane {
        open: false,
        text: String::new(),
        input_scroll: 0,
        focus: RequestPaneFocus::Input,
        selected_button: RequestButton::Confirm,
    };
    let mut make_todos_progress_pane = MakeTodosProgressPane {
        open: false,
        running: false,
        kind: BackgroundJobKind::MakeTodos,
        lines: Vec::new(),
    };
<<<<<<< HEAD
=======
    let mut plan_chat_pane = PlanChatPane {
        open: false,
        running: false,
        input_text: String::new(),
        output_scroll: 0,
        lines: Vec::new(),
        history: Vec::new(),
        project_path: pane_task_spec.project_path.clone(),
        tasks_path: pane_task_spec.tasks_path.clone(),
        plan_path: pane_task_spec
            .project_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("plan.md"),
    };
    let mut pane_detached = false;
>>>>>>> 5b2a204 (fix: seperate process)
    let mut make_todos_job: Option<MakeTodosJob> = None;
    let mut plan_chat_job: Option<PlanChatJob> = None;
    let mut plan_watch_job: Option<PlanWatchJob> = None;
    let mut auto_run_after_todos = false;
    let mut tick = tokio::time::interval(Duration::from_millis(80));
    loop {
        stage_handle_pane_key_events(
            &mut focus,
            &mut pane_task_spec,
            &run_start_tx,
            &mut run_requested,
            &mut request_input_pane,
            &mut rows,
            &mut quit_requested,
            &mut make_todos_progress_pane,
            &mut make_todos_job,
            &mut plan_chat_pane,
            &mut plan_chat_job,
            &mut plan_watch_job,
            &mut auto_run_after_todos,
        )?;
        poll_make_todos_job(
            &mut make_todos_job,
            &mut make_todos_progress_pane,
            &mut pane_task_spec,
            &run_start_tx,
            &mut run_requested,
            &mut auto_run_after_todos,
        );
        poll_plan_chat_job(&mut plan_chat_job, &mut plan_chat_pane, &mut pane_task_spec);
        poll_plan_watch_job(
            &mut plan_watch_job,
            &mut plan_chat_pane,
            &mut pane_task_spec,
            &mut rows,
        );
        if quit_requested {
            break;
        }
        terminal.draw(|frame| {
            let area = working_area(frame.area(), &theme);
            let outer_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(8), Constraint::Length(1)])
                .split(area);
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(outer_chunks[0]);
            let left_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(9), Constraint::Min(8)])
                .split(chunks[0]);
            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(if matches!(focus, PaneFocus::Working) {
                    [Constraint::Length(6), Constraint::Min(8)]
                } else {
                    [Constraint::Min(8), Constraint::Length(5)]
                })
                .split(chunks[1]);

            let project_active = matches!(focus, PaneFocus::Project);
            let task_active = matches!(focus, PaneFocus::TaskSpec);
            let todos_active = matches!(focus, PaneFocus::Todos);
            let working_active = matches!(focus, PaneFocus::Working);
            let task_ready = pane_task_spec.tasks_path.exists() && !pane_task_spec.spec.tasks.is_empty();
            let project_border_style = if project_active {
<<<<<<< HEAD
                Style::default().fg(theme.secondary).bg(theme.background)
            } else {
                Style::default().fg(theme.primary)
            };
            let task_border_style = if task_active {
                Style::default().fg(theme.secondary).bg(theme.background)
=======
                Style::default().fg(theme.active)
            } else {
                Style::default().fg(theme.primary)
            };
            let task_border_style = if !task_ready {
                Style::default().fg(theme.inactive)
            } else if task_active {
                Style::default().fg(theme.active)
>>>>>>> 5b2a204 (fix: seperate process)
            } else {
                Style::default().fg(theme.primary)
            };
            let project_title_style = if project_active {
                Style::default().fg(theme.background).bg(theme.secondary)
            } else {
                Style::default().fg(theme.primary)
            };
<<<<<<< HEAD
            let task_title_style = if task_active {
                Style::default().fg(theme.background).bg(theme.secondary)
            } else {
                Style::default().fg(theme.primary)
            };
            let working_border_style = if working_active {
                Style::default().fg(theme.secondary).bg(theme.background)
            } else {
                Style::default().fg(theme.primary)
            };
            let todos_border_style = if todos_active {
                Style::default().fg(theme.secondary).bg(theme.background)
            } else {
                Style::default().fg(theme.primary)
            };
            let working_title_style = if working_active {
                Style::default().fg(theme.background).bg(theme.secondary)
            } else {
=======
            let task_title_style = if !task_ready {
                Style::default().fg(theme.inactive)
            } else if task_active {
                Style::default().fg(theme.secondary)
            } else if all_done {
                Style::default().fg(theme.primary)
            } else if working_active {
                Style::default().fg(theme.active)
            } else {
                inactive_style
            };
            let todos_ready = pane_task_spec.tasks_path.exists();
            let working_ready = pane_task_spec.tasks_path.exists()
                && pane_task_spec.todos_path.exists()
                && !pane_task_spec.todos.is_empty();
            let any_running = rows.iter().any(|r| matches!(r.status, WorkingStatus::Running));
            let all_done =
                !rows.is_empty() && rows.iter().all(|r| matches!(r.status, WorkingStatus::Done));
            let inactive_style = Style::default().fg(theme.inactive);
            let working_border_style = if !working_ready {
                inactive_style
            } else if any_running {
                Style::default().fg(theme.focus)
            } else if all_done {
                Style::default().fg(theme.primary)
            } else if working_active {
                Style::default().fg(theme.active)
            } else {
                inactive_style
            };
            let todos_border_style = if !todos_ready {
                inactive_style
            } else if todos_active {
                Style::default().fg(theme.active)
            } else {
                Style::default().fg(theme.primary)
            };
            let working_title_style = if !working_ready {
                inactive_style
            } else if any_running {
                Style::default().fg(theme.secondary)
            } else if all_done {
>>>>>>> 5b2a204 (fix: seperate process)
                Style::default().fg(theme.primary)
            } else if working_active {
                Style::default().fg(theme.active)
            } else {
                inactive_style
            };
<<<<<<< HEAD
            let todos_title_style = if todos_active {
                Style::default().fg(theme.background).bg(theme.secondary)
=======
            let todos_title_style = if !todos_ready {
                inactive_style
            } else if todos_active {
                Style::default().fg(theme.active)
>>>>>>> 5b2a204 (fix: seperate process)
            } else {
                Style::default().fg(theme.primary)
            };

            let mode_text = match pane_task_spec.mode {
                TaskSpecMode::List => match pane_task_spec.list_focus {
                    TaskListFocus::Pane => "card-list:pane",
                    TaskListFocus::Item => "card-list:item",
                },
                TaskSpecMode::Form => "form",
            };
            let project_block = Block::default()
                .title(Line::from("project_spec").style(project_title_style))
                .borders(Borders::ALL)
                .border_style(project_border_style)
                .padding(Padding::uniform(theme.padding))
                .style(Style::default().fg(theme.primary));
            let project_inner = project_block.inner(left_chunks[0]);
            frame.render_widget(project_block, left_chunks[0]);
            render_project_spec(frame, project_inner, &pane_task_spec);

            let task_block = Block::default()
                .title(
                    Line::from(format!("task ({mode_text}) | {}", pane_task_spec.status))
                        .style(task_title_style),
                )
                .borders(Borders::ALL)
                .border_style(task_border_style)
                .padding(Padding::uniform(theme.padding))
                .style(Style::default().fg(theme.primary));
            let task_inner = task_block.inner(left_chunks[1]);
            frame.render_widget(task_block, left_chunks[1]);
            match pane_task_spec.mode {
                TaskSpecMode::List => render_task_spec_cards(frame, task_inner, &pane_task_spec, &theme),
                TaskSpecMode::Form => render_task_spec_form(frame, task_inner, &pane_task_spec, &theme),
            }

            let todos_block = Block::default()
                .title(Line::from("todos").style(todos_title_style))
                .borders(Borders::ALL)
                .border_style(todos_border_style)
                .padding(Padding::uniform(theme.padding))
                .style(Style::default().fg(theme.primary));
            let todos_inner = todos_block.inner(right_chunks[0]);
            frame.render_widget(todos_block, right_chunks[0]);
            render_todos_pane(frame, todos_inner, &pane_task_spec, &theme);

            let working_block = Block::default()
                .title(Line::from("working").style(working_title_style))
                .borders(Borders::ALL)
                .border_style(working_border_style)
                .padding(Padding::uniform(theme.padding))
                .style(Style::default().fg(theme.primary));
            let working_inner = working_block.inner(right_chunks[1]);
            frame.render_widget(working_block, right_chunks[1]);
            render_working_compact(frame, working_inner, &pane_task_spec);

            if request_input_pane.open {
                render_request_input_pane(frame, area, &request_input_pane, &theme);
            }
            if make_todos_progress_pane.open {
                render_make_todos_progress_pane(frame, area, &make_todos_progress_pane, &theme);
            }
<<<<<<< HEAD
            render_shortcut_bar(frame, outer_chunks[1], &focus);
=======
            if plan_chat_pane.open {
                render_plan_chat_pane(frame, area, &plan_chat_pane, &theme);
            }
            render_shortcut_bar(
                frame,
                outer_chunks[1],
                &focus,
                pane_detached,
                &request_input_pane,
                &project_select_pane,
                &make_todos_progress_pane,
                &plan_chat_pane,
            );
>>>>>>> 5b2a204 (fix: seperate process)
        })?;

        if should_finish && rows.iter().all(|r| matches!(r.status, WorkingStatus::Done)) {
            tokio::time::sleep(Duration::from_millis(350)).await;
            break;
        }

        tokio::select! {
            _ = tick.tick() => {}
            maybe_event = rx.recv() => {
                match maybe_event {
                    Some(WorkingPaneEvent::SetRunning { worker_id }) => {
                        if let Some(row) = rows.get_mut(worker_id) {
                            row.status = WorkingStatus::Running;
                        }
                    }
                    Some(WorkingPaneEvent::SetDone { worker_id, result }) => {
                        if let Some(row) = rows.get_mut(worker_id) {
                            row.result = result;
                            row.status = WorkingStatus::Done;
                        }
                    }
                    Some(WorkingPaneEvent::Finish) | None => {
                        should_finish = true;
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

fn load_pane_task_spec(path: std::path::PathBuf) -> PaneTaskSpec {
    let mut status =
        "Working + Enter: run | Task(pane) + Enter: add task | Down: select item | Item + Enter: edit | P: make todos"
            .to_string();
    let spec = match std::fs::read_to_string(&path) {
        Ok(raw) => match serde_yaml::from_str::<TaskSpecYaml>(&raw) {
            Ok(parsed) => parsed,
            Err(err) => {
                status = format!("yaml parse failed: {err}");
                TaskSpecYaml::default()
            }
        },
        Err(err) => {
            status = format!("yaml read failed: {err}");
            TaskSpecYaml::default()
        }
    };

    let todos_path = path
        .parent()
        .map(|v| v.join("todos.yaml"))
        .unwrap_or_else(|| std::path::PathBuf::from("todos.yaml"));
    let todos = load_todos_items(&todos_path);

    PaneTaskSpec {
        path,
        todos_path,
        spec,
        todos,
        selected_task: 0,
        list_focus: TaskListFocus::Pane,
        mode: TaskSpecMode::List,
        selected_field: 0,
        input_mode: false,
        input_buffer: String::new(),
        status,
    }
}

fn render_task_spec_cards(
    frame: &mut ratatui::Frame,
    area: Rect,
    pane_task_spec: &PaneTaskSpec,
    theme: &WorkingTheme,
) {
    if pane_task_spec.spec.tasks.is_empty() {
        frame.render_widget(
            Paragraph::new("spec.yaml에 task가 없습니다."),
            area,
        );
        return;
    }

    let card_height: u16 = 5;
    let max_cards = std::cmp::max(1, usize::from((area.height / card_height).max(1)));
    let total = pane_task_spec.spec.tasks.len();
    let end = std::cmp::min(total, pane_task_spec.selected_task + 1);
    let start = end.saturating_sub(max_cards);
    let visible_end = std::cmp::min(total, start + max_cards);

    let mut constraints = Vec::new();
    constraints.push(Constraint::Length(1));
    for _ in start..visible_end {
        constraints.push(Constraint::Length(card_height));
    }
    if constraints.len() == 1 {
        constraints.push(Constraint::Length(card_height));
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let up_hint = if start > 0 { "^ more" } else { "top" };
    let down_hint = if visible_end < total { "v more" } else { "bottom" };
    frame.render_widget(
        Paragraph::new(format!(
            "{up_hint} | selected: {}/{} | visible: {}-{} | {down_hint}",
            pane_task_spec.selected_task + 1,
            total,
            start + 1,
            visible_end
        ))
        .style(Style::default().fg(theme.secondary)),
        chunks[0],
    );

    for (card_slot, task_index) in (start..visible_end).enumerate() {
        let task = &pane_task_spec.spec.tasks[task_index];
        let selected = matches!(pane_task_spec.list_focus, TaskListFocus::Item)
            && task_index == pane_task_spec.selected_task;
        let border_style = if selected {
            Style::default().fg(theme.secondary).bg(theme.background)
        } else {
            Style::default().fg(theme.primary)
        };
        let title_style = if selected {
            Style::default().fg(theme.background).bg(theme.secondary)
        } else {
            Style::default().fg(theme.primary)
        };

        let first_step = task
            .step
            .first()
            .map(String::as_str)
            .unwrap_or("-");
        let first_rule = task
            .rule
            .first()
            .map(String::as_str)
            .unwrap_or("-");
        let content = format!(
            "type: {}\nstep: {}\nrule: {}",
            task.task_type,
            first_step,
            first_rule
        );
        let card = Paragraph::new(content).block(
            Block::default()
                .title(Line::from(format!("{}: {}", task_index + 1, task.name)).style(title_style))
                .borders(Borders::ALL)
                .border_style(border_style)
                .padding(Padding::new(1, 1, 0, 0)),
        );
        frame.render_widget(card, chunks[card_slot + 1]);
    }
}

fn render_task_spec_form(
    frame: &mut ratatui::Frame,
    area: Rect,
    pane_task_spec: &PaneTaskSpec,
    theme: &WorkingTheme,
) {
    let Some(task) = pane_task_spec.spec.tasks.get(pane_task_spec.selected_task) else {
        frame.render_widget(Paragraph::new("선택된 task가 없습니다."), area);
        return;
    };

    let fields = [
        format!("name: {}", task.name),
        format!("type: {}", task.task_type),
        format!("scope: {}", join_items_with_bar(&task.scope)),
        format!("rule: {}", join_items_with_bar(&task.rule)),
        format!("step: {}", join_items_with_bar(&task.step)),
    ];

    let mut lines = Vec::new();
    lines.push(Line::from("Form mode: Up/Down field | Enter edit/commit | Esc back"));
    lines.push(Line::from("scope/rule/step 입력은 ';' 구분"));
    lines.push(Line::from(""));

    for (idx, field) in fields.iter().enumerate() {
        let selected = idx == pane_task_spec.selected_field;
        let style = if selected {
            Style::default().fg(theme.background).bg(theme.secondary)
        } else {
            Style::default().fg(theme.primary)
        };
        let marker = if selected { ">" } else { " " };
        lines.push(Line::from(format!("{marker} {field}")).style(style));
    }

    if pane_task_spec.input_mode {
        lines.push(Line::from(""));
        lines.push(Line::from("editing:"));
        lines.push(Line::from(pane_task_spec.input_buffer.clone()));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_request_input_pane(
    frame: &mut ratatui::Frame,
    area: Rect,
    request_input_pane: &RequestInputPane,
    theme: &WorkingTheme,
) {
    let width = area.width.saturating_mul(70) / 100;
    let height = area.height.saturating_mul(60) / 100;
    let x = area
        .x
        .saturating_add((area.width.saturating_sub(width)) / 2);
    let y = area
        .y
        .saturating_add((area.height.saturating_sub(height)) / 2);
    let popup = Rect::new(x, y, width.max(20), height.max(10));

    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title("set_requset_function")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.secondary).bg(theme.background))
        .padding(Padding::new(1, 1, 1, 1));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(inner);

    let input_title = if matches!(request_input_pane.focus, RequestPaneFocus::Input) {
<<<<<<< HEAD
        "input (multiline, PgUp/PgDn scroll, Tab to buttons)"
=======
        match request_input_pane.kind {
            RequestInputKind::TaskAppend => "input",
            RequestInputKind::ProjectValueEdit => {
                if request_input_pane.project_field == 1 {
                    "value (one rule per line)"
                } else {
                    "value"
                }
            }
        }
>>>>>>> 5b2a204 (fix: seperate process)
    } else {
        "input"
    };
    let viewport_height = chunks[0].height.saturating_sub(2);
    let input_width = chunks[0].width.saturating_sub(2).max(1);
    let total_lines = count_request_input_lines(&request_input_pane.text, input_width);
    let max_scroll = total_lines.saturating_sub(usize::from(viewport_height)) as u16;
    let scroll = request_input_pane.input_scroll.min(max_scroll);
    frame.render_widget(
        Paragraph::new(request_input_pane.text.clone())
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0))
            .block(Block::default().title(input_title).borders(Borders::ALL)),
        chunks[0],
    );

    let cancel_selected = matches!(request_input_pane.focus, RequestPaneFocus::Buttons)
        && matches!(request_input_pane.selected_button, RequestButton::Cancel);
    let confirm_selected = matches!(request_input_pane.focus, RequestPaneFocus::Buttons)
        && matches!(request_input_pane.selected_button, RequestButton::Confirm);
    let cancel_style = if cancel_selected {
        Style::default().bg(theme.secondary).fg(Color::White)
    } else {
        Style::default().fg(theme.primary)
    };
    let confirm_style = if confirm_selected {
        Style::default().bg(theme.secondary).fg(Color::White)
    } else {
        Style::default().fg(theme.primary)
    };
    let button_line = Line::from(vec![
        Span::raw("  "),
        Span::styled(" 취소 ", cancel_style),
        Span::raw("   "),
        Span::styled(" 확인 ", confirm_style),
    ]);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from("Tab: focus change, Left/Right: button, Enter: action"),
            button_line,
        ]),
        chunks[1],
    );
}

fn render_make_todos_progress_pane(
    frame: &mut ratatui::Frame,
    area: Rect,
    pane: &MakeTodosProgressPane,
    theme: &WorkingTheme,
) {
    let width = area.width.saturating_mul(64) / 100;
    let height = area.height.saturating_mul(45) / 100;
    let x = area
        .x
        .saturating_add((area.width.saturating_sub(width)) / 2);
    let y = area
        .y
        .saturating_add((area.height.saturating_sub(height)) / 2);
    let popup = Rect::new(x, y, width.max(26), height.max(8));

    frame.render_widget(Clear, popup);
    let title = match (pane.kind, pane.running) {
        (BackgroundJobKind::MakeTodos, true) => "make_todos (running)",
        (BackgroundJobKind::MakeTodos, false) => "make_todos (done)",
        (BackgroundJobKind::FillProjectTasks, true) => "fill_tasks (running)",
        (BackgroundJobKind::FillProjectTasks, false) => "fill_tasks (done)",
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.secondary).bg(theme.background))
        .padding(Padding::new(1, 1, 1, 1));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let mut lines = pane
        .lines
        .iter()
        .rev()
        .take(12)
        .cloned()
        .collect::<Vec<_>>();
    lines.reverse();
    frame.render_widget(Paragraph::new(lines.join("\n")), inner);
}

fn render_plan_chat_pane(
    frame: &mut ratatui::Frame,
    area: Rect,
    pane: &PlanChatPane,
    theme: &WorkingTheme,
) {
    let width = area.width.saturating_mul(80) / 100;
    let height = area.height.saturating_mul(72) / 100;
    let x = area
        .x
        .saturating_add((area.width.saturating_sub(width)) / 2);
    let y = area
        .y
        .saturating_add((area.height.saturating_sub(height)) / 2);
    let popup = Rect::new(x, y, width.max(40), height.max(12));

    frame.render_widget(Clear, popup);
    let title = if pane.running {
        format!("plan-chat (running) | {}", pane.plan_path.display())
    } else {
        format!("plan-chat | {}", pane.plan_path.display())
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.focus))
        .padding(Padding::new(1, 1, 1, 1));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(3), Constraint::Length(1)])
        .split(inner);

    let viewport_height = chunks[0].height.saturating_sub(2);
    let max_scroll = pane
        .lines
        .len()
        .saturating_sub(usize::from(viewport_height)) as u16;
    let scroll = pane.output_scroll.min(max_scroll);
    frame.render_widget(
        Paragraph::new(pane.lines.join("\n"))
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0))
            .block(Block::default().title("chat log").borders(Borders::ALL)),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(pane.input_text.clone())
            .block(Block::default().title("input").borders(Borders::ALL)),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new("Enter: send | PgUp/PgDn: scroll | Esc: close"),
        chunks[2],
    );
}

fn set_requset_function(
    request_input_pane: &mut RequestInputPane,
    key: KeyCode,
    pane_task_spec: &mut PaneTaskSpec,
    rows: &mut Vec<WorkingRow>,
) -> bool {
    if !request_input_pane.open {
        return false;
    }

    match key {
        KeyCode::Tab => {
            request_input_pane.focus = match request_input_pane.focus {
                RequestPaneFocus::Input => RequestPaneFocus::Buttons,
                RequestPaneFocus::Buttons => RequestPaneFocus::Input,
            };
            return true;
        }
        KeyCode::Esc => {
            request_input_pane.open = false;
            pane_task_spec.status = "request input canceled".to_string();
            return true;
        }
        _ => {}
    }

    match request_input_pane.focus {
        RequestPaneFocus::Input => {
            match key {
                KeyCode::Enter => {
                    request_input_pane.text.push('\n');
                    request_input_pane.input_scroll = u16::MAX;
                }
                KeyCode::Backspace => {
                    let _ = request_input_pane.text.pop();
                    request_input_pane.input_scroll = u16::MAX;
                }
                KeyCode::Char(c) => {
                    request_input_pane.text.push(c);
                    request_input_pane.input_scroll = u16::MAX;
                }
                KeyCode::PageUp => {
                    request_input_pane.input_scroll =
                        request_input_pane.input_scroll.saturating_sub(3);
                }
                KeyCode::PageDown => {
                    request_input_pane.input_scroll =
                        request_input_pane.input_scroll.saturating_add(3);
                }
                KeyCode::Down => request_input_pane.focus = RequestPaneFocus::Buttons,
                _ => {}
            }
            true
        }
        RequestPaneFocus::Buttons => {
            match key {
                KeyCode::Left => request_input_pane.selected_button = RequestButton::Cancel,
                KeyCode::Right => request_input_pane.selected_button = RequestButton::Confirm,
                KeyCode::Up => request_input_pane.focus = RequestPaneFocus::Input,
                KeyCode::Enter => match request_input_pane.selected_button {
                    RequestButton::Cancel => {
                        request_input_pane.open = false;
                        pane_task_spec.status = "request input canceled".to_string();
                    }
                    RequestButton::Confirm => {
                        let parsed_tasks = parsing_request_function(&request_input_pane.text);
                        if !parsed_tasks.is_empty() {
                            let added_count = parsed_tasks.len();
                            pane_task_spec.spec.tasks.extend(parsed_tasks);
                            pane_task_spec.selected_task = pane_task_spec.spec.tasks.len().saturating_sub(1);
                            pane_task_spec.list_focus = TaskListFocus::Pane;
                            pane_task_spec.mode = TaskSpecMode::List;
                            pane_task_spec.selected_field = 0;
                            pane_task_spec.input_mode = false;
                            rows.clear();
                            rows.extend(build_working_rows_from_tasks(&pane_task_spec.spec.tasks));
                            stage_save_task_spec(pane_task_spec);
                            pane_task_spec.status = format!("request parsed and appended: {added_count}");
                        } else {
                            pane_task_spec.status = "parse failed: '# name' is required".to_string();
                            return true;
                        }
                        request_input_pane.open = false;
                    }
                },
                _ => {}
            }
            true
        }
    }
}

fn open_set_request_function(request_input_pane: &mut RequestInputPane, pane_task_spec: &mut PaneTaskSpec) {
    request_input_pane.open = true;
    request_input_pane.text.clear();
    request_input_pane.input_scroll = 0;
    request_input_pane.focus = RequestPaneFocus::Input;
    request_input_pane.selected_button = RequestButton::Confirm;
    pane_task_spec.status = "set_request_function opened".to_string();
}

<<<<<<< HEAD
fn count_request_input_lines(text: &str) -> usize {
    let count = text.lines().count();
    if count == 0 { 1 } else { count }
}

=======
fn get_project_field_name(field: usize) -> &'static str {
    match field {
        0 => "name",
        1 => "rule",
        2 => "framework",
        3 => "description",
        _ => "name",
    }
}

fn get_project_field_value(spec: &TaskSpecYaml, field: usize) -> String {
    match field {
        0 => spec.name.clone(),
        1 => spec.rule.join("\n"),
        2 => spec.framework.clone(),
        3 => spec.description.clone(),
        _ => String::new(),
    }
}

fn apply_project_field_value(spec: &mut TaskSpecYaml, field: usize, value: &str) {
    match field {
        0 => spec.name = value.trim().to_string(),
        1 => spec.rule = split_line_items(value),
        2 => spec.framework = value.trim().to_string(),
        3 => spec.description = value.trim().to_string(),
        _ => {}
    }
}

fn open_project_value_edit_modal(
    request_input_pane: &mut RequestInputPane,
    pane_task_spec: &mut PaneTaskSpec,
) {
    let field = pane_task_spec.project_selected_field;
    request_input_pane.open = true;
    request_input_pane.kind = RequestInputKind::ProjectValueEdit;
    request_input_pane.project_field = field;
    request_input_pane.text = get_project_field_value(&pane_task_spec.spec, field);
    request_input_pane.input_scroll = 0;
    request_input_pane.focus = RequestPaneFocus::Input;
    request_input_pane.selected_button = RequestButton::Confirm;
    pane_task_spec.status = format!("edit {} value", get_project_field_name(field));
}

fn count_request_input_lines(text: &str, content_width: u16) -> usize {
    let width = usize::from(content_width.max(1));
    let mut total = 0usize;
    for line in text.lines() {
        let len = line.chars().count();
        total += std::cmp::max(1, len.div_ceil(width));
    }
    if total == 0 { 1 } else { total }
}

fn open_project_select_pane(
    project_select_pane: &mut ProjectSelectPane,
    pane_task_spec: &mut PaneTaskSpec,
) {
    project_select_pane.open = true;
    project_select_pane.focus = ProjectSelectFocus::List;
    project_select_pane.input_text.clear();
    project_select_pane.selected_button = RequestButton::Confirm;
    project_select_pane.project_list = pane_task_spec.project_list.clone();
    if project_select_pane.project_list.is_empty() {
        project_select_pane.project_list.push("test".to_string());
    }
    project_select_pane.selected_project = project_select_pane
        .project_list
        .iter()
        .position(|v| v == &pane_task_spec.current_project)
        .unwrap_or(0);
    pane_task_spec.status = "project selector opened".to_string();
}

fn set_project_select_pane(
    project_select_pane: &mut ProjectSelectPane,
    key: KeyCode,
    pane_task_spec: &mut PaneTaskSpec,
    rows: &mut Vec<WorkingRow>,
) -> bool {
    if !project_select_pane.open {
        return false;
    }

    match key {
        KeyCode::Esc | KeyCode::F(1) => {
            project_select_pane.open = false;
            pane_task_spec.status = "project selector canceled".to_string();
            return true;
        }
        KeyCode::Tab => {
            project_select_pane.focus = match project_select_pane.focus {
                ProjectSelectFocus::List => ProjectSelectFocus::Input,
                ProjectSelectFocus::Input => ProjectSelectFocus::Buttons,
                ProjectSelectFocus::Buttons => ProjectSelectFocus::List,
            };
            return true;
        }
        _ => {}
    }

    match project_select_pane.focus {
        ProjectSelectFocus::List => {
            match key {
                KeyCode::Up => {
                    project_select_pane.selected_project =
                        project_select_pane.selected_project.saturating_sub(1);
                }
                KeyCode::Down => {
                    if project_select_pane.selected_project + 1
                        < project_select_pane.project_list.len()
                    {
                        project_select_pane.selected_project += 1;
                    }
                }
                KeyCode::Enter => {
                    if let Some(project) = project_select_pane
                        .project_list
                        .get(project_select_pane.selected_project)
                    {
                        pane_task_spec.status = format!("selected project: {project}");
                    }
                }
                _ => {}
            }
            true
        }
        ProjectSelectFocus::Input => {
            match key {
                KeyCode::Backspace => {
                    let _ = project_select_pane.input_text.pop();
                }
                KeyCode::Char(c) => {
                    project_select_pane.input_text.push(c);
                }
                KeyCode::Enter => {
                    let candidate = project_select_pane.input_text.trim();
                    if !candidate.is_empty()
                        && !project_select_pane
                            .project_list
                            .iter()
                            .any(|v| v == candidate)
                    {
                        project_select_pane.project_list.push(candidate.to_string());
                        project_select_pane.project_list.sort();
                    }
                    if let Some(index) = project_select_pane
                        .project_list
                        .iter()
                        .position(|v| v == candidate)
                    {
                        project_select_pane.selected_project = index;
                        pane_task_spec.status = format!("project added/selected: {candidate}");
                    }
                    project_select_pane.input_text.clear();
                }
                _ => {}
            }
            true
        }
        ProjectSelectFocus::Buttons => {
            match key {
                KeyCode::Left => project_select_pane.selected_button = RequestButton::Cancel,
                KeyCode::Right => project_select_pane.selected_button = RequestButton::Confirm,
                KeyCode::Up => project_select_pane.focus = ProjectSelectFocus::List,
                KeyCode::Enter => match project_select_pane.selected_button {
                    RequestButton::Cancel => {
                        project_select_pane.open = false;
                        pane_task_spec.status = "project selector canceled".to_string();
                    }
                    RequestButton::Confirm => {
                        pane_task_spec.project_list = project_select_pane.project_list.clone();
                        if let Some(project_name) = project_select_pane
                            .project_list
                            .get(project_select_pane.selected_project)
                            .cloned()
                        {
                            apply_selected_project(pane_task_spec, rows, &project_name);
                        }
                        project_select_pane.open = false;
                    }
                },
                _ => {}
            }
            true
        }
    }
}

fn open_plan_chat_pane(plan_chat_pane: &mut PlanChatPane, pane_task_spec: &PaneTaskSpec) {
    plan_chat_pane.open = true;
    plan_chat_pane.running = false;
    plan_chat_pane.project_path = pane_task_spec.project_path.clone();
    plan_chat_pane.tasks_path = pane_task_spec.tasks_path.clone();
    plan_chat_pane.plan_path = pane_task_spec
        .project_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("plan.md");
    if plan_chat_pane.lines.is_empty() {
        plan_chat_pane.lines.push(
            "plan-chat opened: Enter로 codex에게 설계 질문을 보내고 plan.md를 갱신합니다."
                .to_string(),
        );
    }
}

fn set_plan_chat_pane(
    plan_chat_pane: &mut PlanChatPane,
    plan_chat_job: &mut Option<PlanChatJob>,
    key: KeyCode,
    pane_task_spec: &mut PaneTaskSpec,
) -> Result<bool> {
    if !plan_chat_pane.open {
        return Ok(false);
    }

    match key {
        KeyCode::Esc => {
            plan_chat_pane.open = false;
            return Ok(true);
        }
        KeyCode::PageUp => {
            plan_chat_pane.output_scroll = plan_chat_pane.output_scroll.saturating_sub(3);
            return Ok(true);
        }
        KeyCode::PageDown => {
            plan_chat_pane.output_scroll = plan_chat_pane.output_scroll.saturating_add(3);
            return Ok(true);
        }
        _ => {}
    }

    if plan_chat_pane.running {
        return Ok(true);
    }

    match key {
        KeyCode::Backspace => {
            let _ = plan_chat_pane.input_text.pop();
            Ok(true)
        }
        KeyCode::Char(c) => {
            plan_chat_pane.input_text.push(c);
            Ok(true)
        }
        KeyCode::Enter => {
            let message = plan_chat_pane.input_text.trim().to_string();
            if message.is_empty() {
                return Ok(true);
            }
            start_plan_chat_turn(plan_chat_pane, plan_chat_job, &message)?;
            pane_task_spec.status = "plan-chat request sent".to_string();
            plan_chat_pane.input_text.clear();
            Ok(true)
        }
        _ => Ok(true),
    }
}

fn start_plan_chat_turn(
    plan_chat_pane: &mut PlanChatPane,
    plan_chat_job: &mut Option<PlanChatJob>,
    user_message: &str,
) -> Result<()> {
    if plan_chat_job.is_some() {
        return Ok(());
    }
    let user_message = user_message.trim();
    if user_message.is_empty() {
        return Ok(());
    }
    plan_chat_pane
        .lines
        .push(format!("user> {user_message}"));
    plan_chat_pane.history.push(PlanChatTurn {
        role: "user".to_string(),
        content: user_message.to_string(),
    });
    plan_chat_pane.output_scroll = u16::MAX;
    plan_chat_pane.running = true;

    let project_path = plan_chat_pane.project_path.clone();
    let plan_path = plan_chat_pane.plan_path.clone();
    let history = plan_chat_pane.history.clone();
    let message = user_message.to_string();
    let (tx, rx) = mpsc::channel::<PlanChatEvent>();
    thread::spawn(move || {
        let result = run_plan_chat_turn(&project_path, &plan_path, &history, &message)
            .map_err(|err| err.to_string());
        let _ = tx.send(PlanChatEvent::Finished(result));
    });
    *plan_chat_job = Some(PlanChatJob { rx });
    Ok(())
}

fn start_plan_file_watch(plan_chat_pane: &PlanChatPane, plan_watch_job: &mut Option<PlanWatchJob>) {
    if plan_watch_job.is_some() {
        return;
    }
    let project_path = plan_chat_pane.project_path.clone();
    let tasks_path = plan_chat_pane.tasks_path.clone();
    let plan_path = plan_chat_pane.plan_path.clone();
    let baseline_modified = std::fs::metadata(&plan_path).and_then(|m| m.modified()).ok();
    let (tx, rx) = mpsc::channel::<PlanWatchEvent>();
    thread::spawn(move || {
        let started = std::time::Instant::now();
        loop {
            if started.elapsed() > Duration::from_secs(1800) {
                let _ = tx.send(PlanWatchEvent::Ready(Err(
                    "plan watcher timeout (30m)".to_string(),
                )));
                break;
            }
            let generated = match std::fs::metadata(&plan_path) {
                Ok(meta) => {
                    if meta.len() == 0 {
                        false
                    } else if let Ok(modified) = meta.modified() {
                        baseline_modified.map(|v| modified > v).unwrap_or(true)
                    } else {
                        baseline_modified.is_none()
                    }
                }
                Err(_) => false,
            };
            if generated {
                let plan_md = std::fs::read_to_string(&plan_path).unwrap_or_default();
                let result = enforce_tasks_from_plan_with_codex(&project_path, &tasks_path, &plan_md)
                    .map_err(|err| err.to_string());
                let _ = tx.send(PlanWatchEvent::Ready(result));
                break;
            }
            thread::sleep(Duration::from_millis(700));
        }
    });
    *plan_watch_job = Some(PlanWatchJob { rx });
}

fn poll_plan_chat_job(
    plan_chat_job: &mut Option<PlanChatJob>,
    plan_chat_pane: &mut PlanChatPane,
    pane_task_spec: &mut PaneTaskSpec,
) {
    let Some(job) = plan_chat_job.as_mut() else {
        return;
    };
    match job.rx.try_recv() {
        Ok(PlanChatEvent::Finished(result)) => {
            plan_chat_pane.running = false;
            match result {
                Ok(output) => {
                    plan_chat_pane.lines.push(format!("assistant> {}", output.reply));
                    plan_chat_pane.history.push(PlanChatTurn {
                        role: "assistant".to_string(),
                        content: output.reply,
                    });
                    pane_task_spec.status = format!(
                        "plan.md updated: {}",
                        plan_chat_pane.plan_path.display()
                    );
                    if output.plan_md.trim().is_empty() {
                        plan_chat_pane
                            .lines
                            .push("warning: plan.md 내용이 비어 있습니다.".to_string());
                    }
                }
                Err(err) => {
                    plan_chat_pane.lines.push(format!("error> {err}"));
                    pane_task_spec.status = format!("plan-chat failed: {err}");
                }
            }
            plan_chat_pane.output_scroll = u16::MAX;
            *plan_chat_job = None;
        }
        Err(TryRecvError::Disconnected) => {
            plan_chat_pane.running = false;
            plan_chat_pane
                .lines
                .push("error> plan-chat channel disconnected".to_string());
            *plan_chat_job = None;
        }
        Err(TryRecvError::Empty) => {}
    }
}

fn poll_plan_watch_job(
    plan_watch_job: &mut Option<PlanWatchJob>,
    plan_chat_pane: &mut PlanChatPane,
    pane_task_spec: &mut PaneTaskSpec,
    rows: &mut Vec<WorkingRow>,
) {
    let Some(job) = plan_watch_job.as_mut() else {
        return;
    };
    match job.rx.try_recv() {
        Ok(PlanWatchEvent::Ready(result)) => {
            match result {
                Ok(updated) => {
                    let tasks = load_tasks_items(&plan_chat_pane.tasks_path);
                    pane_task_spec.spec.tasks = tasks;
                    rows.clear();
                    rows.extend(build_working_rows_from_tasks(&pane_task_spec.spec.tasks));
                    pane_task_spec.status = format!("plan ready -> tasks updated: {updated}");
                    plan_chat_pane.lines.push(format!(
                        "system> plan.md detected. tasks.yaml updated: {updated}"
                    ));
                    plan_chat_pane.open = false;
                }
                Err(err) => {
                    pane_task_spec.status = format!("plan watcher failed: {err}");
                    plan_chat_pane.lines.push(format!("error> {err}"));
                }
            }
            *plan_watch_job = None;
        }
        Err(TryRecvError::Disconnected) => {
            pane_task_spec.status = "plan watcher disconnected".to_string();
            *plan_watch_job = None;
        }
        Err(TryRecvError::Empty) => {}
    }
}

fn run_plan_chat_turn(
    project_path: &std::path::Path,
    plan_path: &std::path::Path,
    history: &[PlanChatTurn],
    user_message: &str,
) -> Result<PlanChatTurnOutput> {
    let project_yaml = std::fs::read_to_string(project_path).unwrap_or_default();
    let prompt = build_plan_chat_prompt(&project_yaml, history, user_message);
    let raw = execute_codex_prompt(&prompt)?;
    let (reply, plan_md) = parse_plan_chat_output(&raw);
    let content = if plan_md.trim().is_empty() {
        "# Plan\n\n- (empty)\n".to_string()
    } else {
        plan_md
    };
    std::fs::write(plan_path, &content).map_err(|e| {
        anyhow::anyhow!("failed to write plan.md ({}): {e}", plan_path.display())
    })?;
    Ok(PlanChatTurnOutput {
        reply,
        plan_md: content,
    })
}

fn build_plan_chat_prompt(project_yaml: &str, history: &[PlanChatTurn], user_message: &str) -> String {
    let history_text = history
        .iter()
        .map(|turn| format!("{}: {}", turn.role, turn.content))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "스킬 사용:\n- /home/tree/ai/skills/plan-code/SKILL.md\n\n\
목표:\n- project.yaml을 바탕으로 plan.md를 작성/갱신한다.\n\
- 사용자 질문에 짧게 답하고, 항상 plan.md 전체 최신본을 제공한다.\n\n\
출력 형식(반드시 준수):\n\
[CHAT]\n\
사용자에게 보여줄 답변\n\
[/CHAT]\n\
[PLAN_MD]\n\
plan.md 전체 markdown\n\
[/PLAN_MD]\n\n\
project.yaml:\n{project_yaml}\n\n\
history:\n{history_text}\n\n\
latest_user_message:\n{user_message}\n"
    )
}

fn parse_plan_chat_output(raw: &str) -> (String, String) {
    let chat = extract_tag_block(raw, "[CHAT]", "[/CHAT]");
    let plan = extract_tag_block(raw, "[PLAN_MD]", "[/PLAN_MD]");
    let reply = chat.unwrap_or_else(|| raw.trim().to_string());
    let plan_md = plan.unwrap_or_else(|| raw.trim().to_string());
    (reply, plan_md)
}

fn enforce_tasks_from_plan_with_codex(
    project_path: &std::path::Path,
    tasks_path: &std::path::Path,
    plan_md: &str,
) -> Result<usize> {
    let project_yaml = std::fs::read_to_string(project_path).unwrap_or_default();
    let current_tasks = std::fs::read_to_string(tasks_path).unwrap_or_default();
    let prompt = format!(
        "스킬 사용:\n- /home/tree/ai/skills/plan-code/SKILL.md\n\n\
아래 plan.md를 기준으로 tasks.yaml을 생성/갱신해라.\n\
규칙:\n\
- tasks.yaml 형식의 순수 YAML만 출력\n\
- 최상위 키는 tasks\n\
- 각 item 키는 name,type,domain,depends_on,scope,state,rule,step\n\
- type은 action|calc\n\
- 기존 tasks.yaml을 참고하되 plan.md를 우선 반영\n\n\
project.yaml:\n{project_yaml}\n\n\
current tasks.yaml:\n{current_tasks}\n\n\
plan.md:\n{plan_md}"
    );
    let raw = execute_codex_prompt(&prompt)?;
    let candidate = extract_yaml_candidate(&raw);
    let parsed = serde_yaml::from_str::<TasksYaml>(&candidate)
        .map_err(|e| anyhow::anyhow!("failed to parse tasks yaml from plan output: {e}"))?;
    if parsed.tasks.is_empty() {
        return Err(anyhow::anyhow!(
            "plan-based tasks generation returned empty tasks"
        ));
    }
    let tasks_yaml = serde_yaml::to_string(&parsed)
        .map_err(|e| anyhow::anyhow!("failed to serialize tasks yaml: {e}"))?;
    std::fs::write(tasks_path, tasks_yaml).map_err(|e| {
        anyhow::anyhow!(
            "failed to save tasks.yaml from generated plan ({}): {e}",
            tasks_path.display()
        )
    })?;
    Ok(parsed.tasks.len())
}

fn extract_tag_block(raw: &str, start_tag: &str, end_tag: &str) -> Option<String> {
    let start = raw.find(start_tag)?;
    let end = raw.find(end_tag)?;
    if end <= start {
        return None;
    }
    let from = start + start_tag.len();
    Some(raw[from..end].trim().to_string())
}

fn apply_selected_project(
    pane_task_spec: &mut PaneTaskSpec,
    rows: &mut Vec<WorkingRow>,
    project_name: &str,
) {
    if project_name.trim().is_empty() {
        return;
    }
    let mut registry = ProjectRegistryYaml {
        currentproject: project_name.to_string(),
        projectlist: pane_task_spec.project_list.clone(),
    };
    registry.projectlist.sort();
    registry.projectlist.dedup();
    let _ = save_project_registry_yaml(&pane_task_spec.project_registry_path, &registry);

    let project_dir = pane_task_spec.project_root_dir.join(project_name);
    let _ = std::fs::create_dir_all(&project_dir);
    let next_spec_path = project_dir.join("tasks.yaml");
    let mut next = load_pane_task_spec(next_spec_path);
    next.status = format!("project switched: {project_name} | {}", next.status);
    *pane_task_spec = next;
    rows.clear();
    rows.extend(build_working_rows_from_tasks(&pane_task_spec.spec.tasks));
}

fn load_project_list(path: &std::path::Path, fallback_current: &str) -> Vec<String> {
    let mut items = std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_yaml::from_str::<ProjectRegistryYaml>(&raw).ok())
        .map(|doc| doc.projectlist)
        .unwrap_or_default();
    if !fallback_current.trim().is_empty() && !items.iter().any(|v| v == fallback_current) {
        items.push(fallback_current.to_string());
    }
    if items.is_empty() {
        items.push("test".to_string());
    }
    items.sort();
    items.dedup();
    items
}

fn save_project_registry_yaml(
    path: &std::path::Path,
    registry: &ProjectRegistryYaml,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            anyhow::anyhow!(
                "failed to create project registry directory ({}): {e}",
                parent.display()
            )
        })?;
    }
    let yaml = serde_yaml::to_string(registry)
        .map_err(|e| anyhow::anyhow!("failed to serialize project registry yaml: {e}"))?;
    std::fs::write(path, yaml).map_err(|e| {
        anyhow::anyhow!(
            "failed to save project registry yaml ({}): {e}",
            path.display()
        )
    })?;
    Ok(())
}

>>>>>>> 5b2a204 (fix: seperate process)
fn stage_handle_pane_key_events(
    focus: &mut PaneFocus,
    pane_task_spec: &mut PaneTaskSpec,
    run_start_tx: &tokio::sync::mpsc::UnboundedSender<()>,
    run_requested: &mut bool,
    request_input_pane: &mut RequestInputPane,
    rows: &mut Vec<WorkingRow>,
    quit_requested: &mut bool,
    make_todos_progress_pane: &mut MakeTodosProgressPane,
    make_todos_job: &mut Option<MakeTodosJob>,
    plan_chat_pane: &mut PlanChatPane,
    plan_chat_job: &mut Option<PlanChatJob>,
    plan_watch_job: &mut Option<PlanWatchJob>,
    auto_run_after_todos: &mut bool,
) -> Result<()> {
    while crossterm::event::poll(Duration::from_millis(0))? {
        let event = crossterm::event::read()?;
        let Event::Key(key) = event else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        if make_todos_progress_pane.open {
            if !make_todos_progress_pane.running
                && matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('Q'))
            {
                make_todos_progress_pane.open = false;
                continue;
            }
            if make_todos_progress_pane.running {
                continue;
            }
        }
<<<<<<< HEAD
        if matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q')) {
=======
        if set_plan_chat_pane(plan_chat_pane, plan_chat_job, key.code, pane_task_spec)? {
            continue;
        }
        let typing_in_input = pane_task_spec.input_mode
            || (request_input_pane.open
                && matches!(request_input_pane.focus, RequestPaneFocus::Input))
            || (project_select_pane.open
                && matches!(project_select_pane.focus, ProjectSelectFocus::Input));
        if !typing_in_input
            && matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q'))
            && matches!(focus, PaneFocus::TaskSpec)
            && matches!(pane_task_spec.mode, TaskSpecMode::Form)
        {
            pane_task_spec.mode = TaskSpecMode::List;
            pane_task_spec.selected_field = 0;
            pane_task_spec.list_focus = TaskListFocus::Item;
            pane_task_spec.status = "card-list mode".to_string();
            continue;
        }
        if !typing_in_input && matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q')) {
>>>>>>> 5b2a204 (fix: seperate process)
            *quit_requested = true;
            continue;
        }

        if set_requset_function(request_input_pane, key.code, pane_task_spec, rows) {
            continue;
        }

        if pane_task_spec.input_mode {
            match key.code {
                KeyCode::Esc => {
                    pane_task_spec.input_mode = false;
                    pane_task_spec.status = "input canceled".to_string();
                }
                KeyCode::Enter => {
                    apply_form_buffer_to_task(pane_task_spec);
                    pane_task_spec.input_mode = false;
                    stage_save_task_spec(pane_task_spec);
                }
                KeyCode::Backspace => {
                    let _ = pane_task_spec.input_buffer.pop();
                }
                KeyCode::Char(c) => {
                    pane_task_spec.input_buffer.push(c);
                }
                _ => {}
            }
            continue;
        }

        if matches!(pane_task_spec.mode, TaskSpecMode::List) {
            match key.code {
                KeyCode::Left => {
                    *focus = PaneFocus::Project;
                    pane_task_spec.list_focus = TaskListFocus::Pane;
                }
                KeyCode::Right if matches!(focus, PaneFocus::Project) => *focus = PaneFocus::TaskSpec,
                KeyCode::Right if matches!(focus, PaneFocus::TaskSpec) => *focus = PaneFocus::Todos,
                KeyCode::Down if matches!(focus, PaneFocus::Todos) => *focus = PaneFocus::Working,
                KeyCode::Down if matches!(focus, PaneFocus::Project) => *focus = PaneFocus::TaskSpec,
                KeyCode::Up if matches!(focus, PaneFocus::Working) => *focus = PaneFocus::Todos,
                KeyCode::Up if matches!(focus, PaneFocus::TaskSpec) => *focus = PaneFocus::Project,
                KeyCode::Char('p') | KeyCode::Char('P') if matches!(focus, PaneFocus::Working) => {
                    if !pane_task_spec.todos_path.exists() || pane_task_spec.todos.is_empty() {
                        pane_task_spec.status =
                            "run blocked: generate todos first (todos pane: p)".to_string();
                    } else if !*run_requested {
                        let _ = run_start_tx.send(());
                        *run_requested = true;
                        pane_task_spec.status = "run started".to_string();
                    }
                }
<<<<<<< HEAD
=======
                KeyCode::Char('f') | KeyCode::Char('F') if matches!(focus, PaneFocus::Project) => {
                    if make_todos_job.is_none() {
                        let (pane, job) = start_fill_project_tasks_job(
                            &pane_task_spec.project_path,
                            &pane_task_spec.tasks_path,
                        );
                        *make_todos_progress_pane = pane;
                        *make_todos_job = Some(job);
                        pane_task_spec.status = "tasks.yaml fill started".to_string();
                    }
                }
                KeyCode::F(1) if matches!(focus, PaneFocus::Project) => {
                    open_project_select_pane(project_select_pane, pane_task_spec);
                }
                KeyCode::Char('P') if matches!(focus, PaneFocus::TaskSpec) => {
                    if pane_task_spec.spec.tasks.is_empty() {
                        open_plan_chat_pane(plan_chat_pane, pane_task_spec);
                        start_plan_file_watch(plan_chat_pane, plan_watch_job);
                        start_plan_chat_turn(
                            plan_chat_pane,
                            plan_chat_job,
                            "project.yaml을 기반으로 plan-code 방식의 plan.md를 작성해줘.",
                        )?;
                    }
                }
>>>>>>> 5b2a204 (fix: seperate process)
                KeyCode::Char('a') | KeyCode::Char('A') if matches!(focus, PaneFocus::Project) => {
                    let has_base = !pane_task_spec.spec.name.trim().is_empty()
                        && !pane_task_spec.spec.framework.trim().is_empty()
                        && !pane_task_spec.spec.rule.is_empty();
                    if !has_base {
                        pane_task_spec.status =
                            "auto blocked: project name/framework/rule required".to_string();
                    } else if make_todos_job.is_none() {
                        let (pane, job) = start_make_todos_job(&pane_task_spec.path);
                        *make_todos_progress_pane = pane;
                        *make_todos_job = Some(job);
                        *auto_run_after_todos = false;
                        pane_task_spec.status = "auto mode started (run is manual)".to_string();
                    }
                }
                _ => {}
            }
<<<<<<< HEAD
=======

            if matches!(focus, PaneFocus::Todos)
                && matches!(key.code, KeyCode::Char('p') | KeyCode::Char('P'))
                && make_todos_job.is_none()
            {
                if !pane_task_spec.tasks_path.exists() || pane_task_spec.spec.tasks.is_empty() {
                    pane_task_spec.status =
                        "make_todos blocked: fill tasks first (project/task pane: f)".to_string();
                } else {
                    let (pane, job) = start_make_todos_job(
                        &pane_task_spec.project_path,
                        &pane_task_spec.tasks_path,
                    );
                    *make_todos_progress_pane = pane;
                    *make_todos_job = Some(job);
                    pane_task_spec.status = "make_todos_spec started".to_string();
                }
            }
>>>>>>> 5b2a204 (fix: seperate process)
        }

        if !matches!(focus, PaneFocus::TaskSpec) {
            continue;
        }

        match pane_task_spec.mode {
            TaskSpecMode::List => match key.code {
                KeyCode::Up => {
                    match pane_task_spec.list_focus {
                        TaskListFocus::Pane => {}
                        TaskListFocus::Item => {
                            if pane_task_spec.selected_task == 0 {
                                pane_task_spec.list_focus = TaskListFocus::Pane;
                            } else {
                                pane_task_spec.selected_task = pane_task_spec.selected_task.saturating_sub(1);
                            }
                        }
                    }
                }
                KeyCode::Down => {
                    if pane_task_spec.spec.tasks.is_empty() {
                        pane_task_spec.status = "item이 없습니다. Enter로 task를 추가하세요.".to_string();
                    } else {
                        match pane_task_spec.list_focus {
                            TaskListFocus::Pane => {
                                pane_task_spec.list_focus = TaskListFocus::Item;
                            }
                            TaskListFocus::Item => {
                                if pane_task_spec.selected_task + 1 < pane_task_spec.spec.tasks.len() {
                                    pane_task_spec.selected_task += 1;
                                }
                            }
                        }
                    }
                }
<<<<<<< HEAD
                KeyCode::Char('p') | KeyCode::Char('P') => {
=======
                KeyCode::Char('f') | KeyCode::Char('F') => {
>>>>>>> 5b2a204 (fix: seperate process)
                    if make_todos_job.is_none() {
                        let (pane, job) = start_make_todos_job(&pane_task_spec.path);
                        *make_todos_progress_pane = pane;
                        *make_todos_job = Some(job);
                        pane_task_spec.status = "make_todos_spec started".to_string();
                    }
                }
                KeyCode::Enter => {
                    match pane_task_spec.list_focus {
                        TaskListFocus::Pane => {
                            open_set_request_function(request_input_pane, pane_task_spec);
                        }
                        TaskListFocus::Item => {
                            if pane_task_spec.spec.tasks.is_empty() {
                                pane_task_spec.status = "수정할 task가 없습니다.".to_string();
                            } else {
                                pane_task_spec.mode = TaskSpecMode::Form;
                                pane_task_spec.selected_field = 0;
                                pane_task_spec.status = "form mode".to_string();
                            }
                        }
                    }
                }
                _ => {}
            },
            TaskSpecMode::Form => match key.code {
                KeyCode::Esc => {
                    pane_task_spec.mode = TaskSpecMode::List;
                    pane_task_spec.selected_field = 0;
                    pane_task_spec.list_focus = TaskListFocus::Item;
                    pane_task_spec.status = "card-list mode".to_string();
                }
                KeyCode::Up => {
                    pane_task_spec.selected_field = pane_task_spec.selected_field.saturating_sub(1);
                }
                KeyCode::Down => {
                    if pane_task_spec.selected_field < 4 {
                        pane_task_spec.selected_field += 1;
                    }
                }
                KeyCode::Enter => {
                    pane_task_spec.input_mode = true;
                    pane_task_spec.input_buffer = get_selected_field_value(pane_task_spec);
                    pane_task_spec.status = "field editing".to_string();
                }
                _ => {}
            },
        }
    }
    Ok(())
}

fn parsing_request_function(raw: &str) -> Vec<TaskSpecItem> {
    let mut tasks = Vec::new();
    let mut current: Option<TaskSpecItem> = None;

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
            current = Some(TaskSpecItem {
                name: name.to_string(),
                task_type: "action".to_string(),
                domain: Vec::new(),
                depends_on: Vec::new(),
                scope: Vec::new(),
                state: Vec::new(),
                rule: Vec::new(),
                step: Vec::new(),
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

fn build_working_rows_from_tasks(tasks: &[TaskSpecItem]) -> Vec<WorkingRow> {
    tasks
        .iter()
        .map(|task| WorkingRow {
            request: format!(
                "# {}\n> {}\n- {}",
                task.name,
                if task.step.is_empty() {
                    "-".to_string()
                } else {
                    task.step.join(" | ")
                },
                if task.rule.is_empty() {
                    "-".to_string()
                } else {
                    task.rule.join(" | ")
                }
            ),
            result: String::new(),
            status: WorkingStatus::Ready,
        })
        .collect::<Vec<_>>()
}

fn render_project_spec(frame: &mut ratatui::Frame, area: Rect, pane_task_spec: &PaneTaskSpec) {
    let spec = &pane_task_spec.spec;
    let rule = if spec.rule.is_empty() {
        "-".to_string()
    } else {
        spec.rule.join(" | ")
    };
    let feature = if spec.features.feature.is_empty() {
        "-".to_string()
    } else {
        spec.features.feature.join(" | ")
    };
    let domain = if spec.features.domain.is_empty() {
        "-".to_string()
    } else {
        spec.features.domain.join(" | ")
    };

    let lines = vec![
        Line::from(format!("name: {}", show_or_dash(&spec.name))),
        Line::from(format!("framework: {}", show_or_dash(&spec.framework))),
        Line::from(format!("rule: {rule}")),
        Line::from(format!("domain: {domain}")),
        Line::from(format!("feature: {feature}")),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}

fn show_or_dash(value: &str) -> &str {
    if value.trim().is_empty() { "-" } else { value }
}

fn render_todos_pane(
    frame: &mut ratatui::Frame,
    area: Rect,
    pane_task_spec: &PaneTaskSpec,
    theme: &WorkingTheme,
) {
    if pane_task_spec.todos.is_empty() {
        if !pane_task_spec.tasks_path.exists() || pane_task_spec.spec.tasks.is_empty() {
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from("todo item이 없습니다."),
                    Line::from("선행조건 미완료: tasks.yaml 생성/채우기 (project/task pane: f)"),
                ]),
                area,
            );
        } else {
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from("todo item이 없습니다."),
                    Line::from("작업리스트 만들기: todos pane에서 p"),
                ]),
                area,
            );
        }
        return;
    }
    let task_opt = pane_task_spec
        .spec
        .tasks
        .get(pane_task_spec.selected_task)
        .and_then(|selected| {
            pane_task_spec
                .todos
                .iter()
                .enumerate()
                .find(|(_, item)| item.name == selected.name)
        })
        .or_else(|| pane_task_spec.todos.get(pane_task_spec.selected_task).map(|v| (pane_task_spec.selected_task, v)))
        .or_else(|| pane_task_spec.todos.first().map(|v| (0, v)));
    let Some((matched_index, task)) = task_opt else {
        frame.render_widget(Paragraph::new("todo item을 찾을 수 없습니다."), area);
        return;
    };

    let mut lines = vec![
        Line::from(format!(
            "item: {}/{} (match: {})",
            pane_task_spec.selected_task + 1,
            pane_task_spec.todos.len(),
            matched_index + 1
        ))
        .style(Style::default().fg(theme.secondary)),
        Line::from(format!("name: {}", task.name)),
        Line::from("todos:"),
    ];

    if task.step.is_empty() {
        lines.push(Line::from("  - (none)"));
    } else {
        for todo in &task.step {
            lines.push(Line::from(format!("  - {todo}")));
        }
    }
    if !task.rule.is_empty() {
        lines.push(Line::from("rules:"));
        for rule in &task.rule {
            lines.push(Line::from(format!("  - {rule}")));
        }
    }
    frame.render_widget(Paragraph::new(lines), area);
}

fn render_working_compact(frame: &mut ratatui::Frame, area: Rect, pane_task_spec: &PaneTaskSpec) {
    let mut lines = vec![Line::from("todo names:")];
    if !pane_task_spec.todos_path.exists() || pane_task_spec.todos.is_empty() {
        lines.push(Line::from("  - (none)"));
        lines.push(Line::from("선행조건 미완료: todos.yaml 생성 (todos pane: p)"));
    } else {
        for (idx, item) in pane_task_spec.todos.iter().enumerate() {
            lines.push(Line::from(format!("  {}. {}", idx + 1, item.name)));
        }
    }
    frame.render_widget(Paragraph::new(lines), area);
}

fn get_selected_field_value(pane_task_spec: &PaneTaskSpec) -> String {
    let Some(task) = pane_task_spec.spec.tasks.get(pane_task_spec.selected_task) else {
        return String::new();
    };
    match pane_task_spec.selected_field {
        0 => task.name.clone(),
        1 => task.task_type.clone(),
        2 => task.scope.join("; "),
        3 => task.rule.join("; "),
        4 => task.step.join("; "),
        _ => String::new(),
    }
}

fn apply_form_buffer_to_task(pane_task_spec: &mut PaneTaskSpec) {
    let Some(task) = pane_task_spec.spec.tasks.get_mut(pane_task_spec.selected_task) else {
        return;
    };
    match pane_task_spec.selected_field {
        0 => task.name = pane_task_spec.input_buffer.trim().to_string(),
        1 => task.task_type = pane_task_spec.input_buffer.trim().to_string(),
        2 => task.scope = split_semicolon_items(&pane_task_spec.input_buffer),
        3 => task.rule = split_semicolon_items(&pane_task_spec.input_buffer),
        4 => task.step = split_semicolon_items(&pane_task_spec.input_buffer),
        _ => {}
    }
}

fn split_semicolon_items(input: &str) -> Vec<String> {
    input
        .split(';')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>()
}

fn split_line_items(input: &str) -> Vec<String> {
    input
        .lines()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>()
}

fn stage_save_task_spec(pane_task_spec: &mut PaneTaskSpec) {
    let yaml = match serde_yaml::to_string(&pane_task_spec.spec) {
        Ok(v) => v,
        Err(err) => {
            pane_task_spec.status = format!("serialize failed: {err}");
            return;
        }
    };

    if let Err(err) = std::fs::write(&pane_task_spec.path, yaml) {
        pane_task_spec.status = format!("save failed: {err}");
    } else {
        pane_task_spec.status = "saved".to_string();
    }
}

fn start_make_todos_job(spec_path: &std::path::Path) -> (MakeTodosProgressPane, MakeTodosJob) {
    let (tx, rx) = mpsc::channel::<MakeTodosEvent>();
    let spec_path = spec_path.to_path_buf();
    thread::spawn(move || {
        let send_progress = |message: &str| {
            let _ = tx.send(MakeTodosEvent::Progress(message.to_string()));
        };
        let result = run_make_todos_pipeline_with_progress(&spec_path, send_progress)
            .map_err(|err| err.to_string());
        let _ = tx.send(MakeTodosEvent::Finished(result));
    });

    (
        MakeTodosProgressPane {
            open: true,
            running: true,
            kind: BackgroundJobKind::MakeTodos,
            lines: vec!["make_todos_spec started".to_string()],
        },
<<<<<<< HEAD
        MakeTodosJob { rx },
=======
        MakeTodosJob {
            rx,
            kind: BackgroundJobKind::MakeTodos,
        },
    )
}

fn start_fill_project_tasks_job(
    project_path: &std::path::Path,
    tasks_path: &std::path::Path,
) -> (MakeTodosProgressPane, MakeTodosJob) {
    let (tx, rx) = mpsc::channel::<MakeTodosEvent>();
    let project_path = project_path.to_path_buf();
    let tasks_path = tasks_path.to_path_buf();
    thread::spawn(move || {
        let send_progress = |message: &str| {
            let _ = tx.send(MakeTodosEvent::Progress(message.to_string()));
        };
        let result =
            run_project_task_fill_pipeline_with_progress(&project_path, &tasks_path, send_progress)
                .map_err(|err| err.to_string());
        let _ = tx.send(MakeTodosEvent::Finished(result));
    });

    (
        MakeTodosProgressPane {
            open: true,
            running: true,
            kind: BackgroundJobKind::FillProjectTasks,
            lines: vec!["project_spec tasks fill started".to_string()],
        },
        MakeTodosJob {
            rx,
            kind: BackgroundJobKind::FillProjectTasks,
        },
>>>>>>> 5b2a204 (fix: seperate process)
    )
}

fn poll_make_todos_job(
    make_todos_job: &mut Option<MakeTodosJob>,
    make_todos_progress_pane: &mut MakeTodosProgressPane,
    pane_task_spec: &mut PaneTaskSpec,
<<<<<<< HEAD
    run_start_tx: &tokio::sync::mpsc::UnboundedSender<()>,
    run_requested: &mut bool,
=======
    rows: &mut Vec<WorkingRow>,
    _run_start_tx: &tokio::sync::mpsc::UnboundedSender<()>,
    _run_requested: &mut bool,
>>>>>>> 5b2a204 (fix: seperate process)
    auto_run_after_todos: &mut bool,
) {
    let Some(job) = make_todos_job.as_mut() else {
        return;
    };

    loop {
        match job.rx.try_recv() {
            Ok(MakeTodosEvent::Progress(message)) => {
                make_todos_progress_pane.lines.push(message);
            }
            Ok(MakeTodosEvent::Finished(result)) => {
                make_todos_progress_pane.running = false;
                match result {
                    Ok(output) => {
                        pane_task_spec.spec = output.updated_spec;
<<<<<<< HEAD
                        let appended = output.generated_todos.len();
                        if appended == 0 {
                            pane_task_spec.status = "make_todos_spec failed: generated todos is empty".to_string();
                            make_todos_progress_pane
                                .lines
                                .push("failed: generated todos is empty".to_string());
                        } else {
                            pane_task_spec.todos.extend(output.generated_todos);
                            stage_save_todos_spec(pane_task_spec);
                            pane_task_spec.status =
                                format!("make_todos_spec appended: {appended}");
                            make_todos_progress_pane.lines.push(format!("done: appended {appended} items"));
                            make_todos_progress_pane.lines.push("auto-close: success".to_string());
                            make_todos_progress_pane.open = false;
                            if *auto_run_after_todos && !*run_requested {
                                let _ = run_start_tx.send(());
                                *run_requested = true;
                                pane_task_spec.status = "auto mode: run started".to_string();
=======
                        match job_kind {
                            BackgroundJobKind::MakeTodos => {
                                let appended = output.generated_todos.len();
                                if appended == 0 {
                                    pane_task_spec.status =
                                        "make_todos_spec failed: generated todos is empty"
                                            .to_string();
                                    make_todos_progress_pane
                                        .lines
                                        .push("failed: generated todos is empty".to_string());
                                    if let Some(path) = write_make_todos_failure_file(
                                        &pane_task_spec.status,
                                        &make_todos_progress_pane.lines,
                                    ) {
                                        make_todos_progress_pane
                                            .lines
                                            .push(format!("saved failure: {}", path.display()));
                                    }
                                } else {
                                    pane_task_spec.todos.extend(output.generated_todos);
                                    stage_save_todos_spec(pane_task_spec);
                                    pane_task_spec.status =
                                        format!("make_todos_spec appended: {appended}");
                                    make_todos_progress_pane
                                        .lines
                                        .push(format!("done: appended {appended} items"));
                                    make_todos_progress_pane
                                        .lines
                                        .push("auto-close: success".to_string());
                                    make_todos_progress_pane.open = false;
                                }
                            }
                            BackgroundJobKind::FillProjectTasks => {
                                rows.clear();
                                rows.extend(build_working_rows_from_tasks(
                                    &pane_task_spec.spec.tasks,
                                ));
                                pane_task_spec.status = format!(
                                    "project_spec tasks updated: {}",
                                    pane_task_spec.spec.tasks.len()
                                );
                                make_todos_progress_pane.lines.push(format!(
                                    "done: tasks {}",
                                    pane_task_spec.spec.tasks.len()
                                ));
                                make_todos_progress_pane
                                    .lines
                                    .push("auto-close: success".to_string());
                                make_todos_progress_pane.open = false;
>>>>>>> 5b2a204 (fix: seperate process)
                            }
                        }
                    }
                    Err(err) => {
                        pane_task_spec.status = format!("make_todos_spec failed: {err}");
                        make_todos_progress_pane.lines.push(format!("failed: {err}"));
                    }
                }
                make_todos_progress_pane
                    .lines
                    .push("Esc/Enter: close".to_string());
                *make_todos_job = None;
                *auto_run_after_todos = false;
                break;
            }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                make_todos_progress_pane.running = false;
                make_todos_progress_pane
                    .lines
                    .push("failed: progress channel disconnected".to_string());
                make_todos_progress_pane
                    .lines
                    .push("Esc/Enter: close".to_string());
                pane_task_spec.status = "make_todos_spec failed: channel disconnected".to_string();
                *make_todos_job = None;
                *auto_run_after_todos = false;
                break;
            }
        }
    }
}

<<<<<<< HEAD
fn render_shortcut_bar(frame: &mut ratatui::Frame, area: Rect, focus: &PaneFocus) {
    let base = "q: quit | ←/→/↑/↓: focus 이동 | Enter: 선택";
    let extra = match focus {
        PaneFocus::Project => " | a: auto",
        PaneFocus::TaskSpec => " | p: make_todos",
        PaneFocus::Todos => " | ↓: working",
        PaneFocus::Working => " | p: run",
=======
fn append_error_log_in_cwd(message: &str) {
    let path = std::path::Path::new("log.md");
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let line = format!("- [{ts}] {message}\n");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = std::io::Write::write_all(&mut file, line.as_bytes());
    }
}

fn write_make_todos_failure_file(
    status: &str,
    progress_lines: &[String],
) -> Option<std::path::PathBuf> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = std::path::PathBuf::from(format!("make_todos_spec_failed_{ts}.md"));
    let mut body = String::new();
    body.push_str("# make_todos_spec failure\n\n");
    body.push_str(&format!("- time: {ts}\n"));
    body.push_str(&format!("- status: {status}\n\n"));
    body.push_str("## Progress\n");
    for line in progress_lines {
        body.push_str("- ");
        body.push_str(line);
        body.push('\n');
    }
    if std::fs::write(&path, body).is_ok() {
        Some(path)
    } else {
        None
    }
}

fn render_shortcut_bar(
    frame: &mut ratatui::Frame,
    area: Rect,
    focus: &PaneFocus,
    pane_detached: bool,
    request_input_pane: &RequestInputPane,
    project_select_pane: &ProjectSelectPane,
    make_todos_progress_pane: &MakeTodosProgressPane,
    plan_chat_pane: &PlanChatPane,
) {
    let line = if request_input_pane.open {
        "modal(request): Tab focus | PgUp/PgDn scroll | Enter action | Esc close".to_string()
    } else if project_select_pane.open {
        "modal(project): Up/Down select | Tab focus | Enter apply | Esc/F1 close".to_string()
    } else if plan_chat_pane.open {
        "modal(plan-chat): Enter send | PgUp/PgDn scroll | Esc close".to_string()
    } else if make_todos_progress_pane.open {
        if make_todos_progress_pane.running {
            "background job running...".to_string()
        } else {
            "background job done: Esc/Enter close".to_string()
        }
    } else if pane_detached {
        "navigation mode: arrows move pane | Enter: focus pane | q: quit".to_string()
    } else {
        let base = "q: quit | arrows: focus | Enter: select";
        let extra = match focus {
            PaneFocus::Project => {
                " | F1: project | Up/Down: field | Enter: edit value | f: fill tasks"
            }
            PaneFocus::TaskSpec => " | Enter: add/edit | f: fill tasks | P: plan-chat",
            PaneFocus::Todos => " | Up/Down: focus | p/P: make_todos",
            PaneFocus::Working => " | p: run",
        };
        format!("{base}{extra}")
>>>>>>> 5b2a204 (fix: seperate process)
    };
    let line = format!("{base}{extra}");
    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(Color::Black).fg(Color::White)),
        area,
    );
}

fn run_make_todos_spec_with_progress<F>(
    parsed_spec: &TaskSpecYaml,
    mut send_progress: F,
) -> Result<Vec<TaskSpecItem>>
where
    F: FnMut(&str),
{
    if parsed_spec.tasks.is_empty() {
        return Ok(Vec::new());
    }

    let total = parsed_spec.tasks.len();
    send_progress(&format!("running codex in parallel for {total} tasks"));
    let run_token = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    );

    let mut handles = Vec::new();
    let mut temp_paths = Vec::new();
    for (idx, task) in parsed_spec.tasks.iter().enumerate() {
        let one_task_spec = TaskSpecYaml {
            name: parsed_spec.name.clone(),
            framework: parsed_spec.framework.clone(),
            rule: parsed_spec.rule.clone(),
            features: parsed_spec.features.clone(),
            tasks: vec![task.clone()],
        };
        let temp_path = std::env::temp_dir().join(format!(
            "orchestra_todos_item_{}_{}.yaml",
            run_token,
            idx
        ));
        temp_paths.push(temp_path.clone());
        let task_name = task.name.clone();
        handles.push(thread::spawn(move || -> Result<(usize, String, std::path::PathBuf)> {
            let one_task_spec_text = serde_yaml::to_string(&one_task_spec)
                .map_err(|e| anyhow::anyhow!("failed to serialize single-task spec: {e}"))?;
            let prompt = build_make_todos_prompt(&one_task_spec_text);
            let raw_output = execute_codex_prompt(&prompt)?;
            let generated = parse_todos_from_codex_output(&raw_output)?;
            let yaml = serde_yaml::to_string(&generated)
                .map_err(|e| anyhow::anyhow!("failed to serialize generated todos: {e}"))?;
            std::fs::write(&temp_path, yaml).map_err(|e| {
                anyhow::anyhow!("failed to write temp todos file ({}): {e}", temp_path.display())
            })?;
            Ok((idx, task_name, temp_path))
        }));
    }

    let mut finished = Vec::new();
    for handle in handles {
        let joined = handle
            .join()
            .map_err(|_| anyhow::anyhow!("task worker thread panicked"))?;
        match joined {
            Ok((idx, task_name, temp_path)) => {
                send_progress(&format!("worker done ({}/{}) {}", idx + 1, total, task_name));
                finished.push((idx, temp_path));
            }
            Err(err) => {
                cleanup_temp_files(&temp_paths);
                return Err(err);
            }
        }
    }
    if finished.len() != total {
        cleanup_temp_files(&temp_paths);
        return Err(anyhow::anyhow!(
            "parallel generation incomplete: expected {total}, got {}",
            finished.len()
        ));
    }
    if let Some((_, missing_path)) = finished
        .iter()
        .find(|(_, path)| !path.exists())
    {
        cleanup_temp_files(&temp_paths);
        return Err(anyhow::anyhow!(
            "parallel generation incomplete: missing temp file {}",
            missing_path.display()
        ));
    }

    send_progress("merging temporary todo files");
    finished.sort_by_key(|(idx, _)| *idx);
    let mut all_generated = Vec::new();
    for (_, temp_path) in &finished {
        let raw = match std::fs::read_to_string(temp_path) {
            Ok(v) => v,
            Err(e) => {
                cleanup_temp_files(&temp_paths);
                return Err(anyhow::anyhow!(
                    "failed to read temp todos file ({}): {e}",
                    temp_path.display()
                ));
            }
        };
        let generated = match serde_yaml::from_str::<TaskSpecYaml>(&raw) {
            Ok(v) => v,
            Err(e) => {
                cleanup_temp_files(&temp_paths);
                return Err(anyhow::anyhow!("failed to parse temp todos file: {e}"));
            }
        };
        all_generated.extend(generated.tasks);
    }
    cleanup_temp_files(&temp_paths);
    send_progress("temporary files cleaned");
    Ok(all_generated)
}

fn run_make_todos_pipeline_with_progress<F>(
    spec_path: &std::path::Path,
    mut send_progress: F,
) -> Result<MakeTodosOutput>
where
    F: FnMut(&str),
{
    send_progress("reading spec.yaml");
    let spec_text = std::fs::read_to_string(spec_path)
        .map_err(|e| anyhow::anyhow!("failed to read spec.yaml ({}): {e}", spec_path.display()))?;
    let parsed_spec = serde_yaml::from_str::<TaskSpecYaml>(&spec_text)
        .map_err(|e| anyhow::anyhow!("failed to parse spec.yaml: {e}"))?;
    send_progress("enriching spec tasks (type/scope/depends_on)");
    let enriched_spec = enrich_spec_tasks_with_codex(&parsed_spec)?;
    let enriched_yaml = serde_yaml::to_string(&enriched_spec)
        .map_err(|e| anyhow::anyhow!("failed to serialize enriched spec: {e}"))?;
    std::fs::write(spec_path, enriched_yaml)
        .map_err(|e| anyhow::anyhow!("failed to save enriched spec.yaml: {e}"))?;
    send_progress("enriched spec.yaml saved");

    let generated_todos = run_make_todos_spec_with_progress(&enriched_spec, &mut send_progress)?;
    Ok(MakeTodosOutput {
        updated_spec: enriched_spec,
        generated_todos,
    })
}

fn cleanup_temp_files(paths: &[std::path::PathBuf]) {
    for path in paths {
        let _ = std::fs::remove_file(path);
    }
}

fn enrich_spec_tasks_with_codex(spec: &TaskSpecYaml) -> Result<TaskSpecYaml> {
    let spec_text = serde_yaml::to_string(spec)
        .map_err(|e| anyhow::anyhow!("failed to serialize spec for enrich prompt: {e}"))?;
    let domain_candidates = extract_domain_candidates(spec);
<<<<<<< HEAD
    let prompt = build_enrich_spec_prompt(&spec_text, &domain_candidates);
=======
    let project_definition = build_project_definition_block(spec);
    let design_template = load_plan_code_design_template_for_prompt();
    let prompt = build_enrich_spec_prompt(
        &spec_text,
        &domain_candidates,
        &project_definition,
        &design_template,
    );
>>>>>>> 5b2a204 (fix: seperate process)
    let raw_output = execute_codex_prompt(&prompt)?;
    parse_spec_from_codex_output(&raw_output)
}

fn extract_domain_candidates(spec: &TaskSpecYaml) -> Vec<String> {
    let mut candidates = Vec::new();
    for part in &spec.features.domain {
        let trimmed = part.trim();
        if !trimmed.is_empty() {
            candidates.push(trimmed.to_string());
        }
    }
    if candidates.is_empty() {
        candidates.push("none".to_string());
    }
    candidates
}

<<<<<<< HEAD
fn build_enrich_spec_prompt(spec_text: &str, domain_candidates: &[String]) -> String {
    let fallback = "현재 spec.yaml의 tasks/feature를 전체적으로 검토하고 domain 구성을 보강해줘.\n\
\n\
스킬 사용:\n\
- /home/tree/ai/skills/domain_create/SKILL.md\n\
=======
fn build_project_definition_block(spec: &TaskSpecYaml) -> String {
    let project_rule = if spec.rule.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", spec.rule.join(", "))
    };
    format!(
        "name: {}\ndescription: {}\nframework: {}\nrule: {}",
        spec.name, spec.description, spec.framework, project_rule
    )
}

fn build_enrich_spec_prompt(
    spec_text: &str,
    domain_candidates: &[String],
    project_definition: &str,
    design_template: &str,
) -> String {
    let fallback = "현재 tasks.yaml의 tasks/feature를 전체적으로 검토하고 domain 구성을 보강해줘.\n\
>>>>>>> 5b2a204 (fix: seperate process)
\n\
목표:\n\
- 모든 기능 추가를 먼저 훑어본 뒤 spec.yaml의 features.domain을 보강\n\
- 각 task의 type/scope/depends_on을 보강\n\
\n\
규칙:\n\
- 단일 codex 호출로 전체를 처리(병렬 금지)\n\
<<<<<<< HEAD
=======
- 외부 Skill/plan 문서를 읽으려 하지 말고, 아래 제공된 design_template/project_definition/tasks 텍스트만 사용\n\
- design_template 형식으로 먼저 내부 설계를 고정한 뒤 결과를 tasks.yaml + features 순수 YAML로만 출력\n\
>>>>>>> 5b2a204 (fix: seperate process)
- features.domain은 문자열 배열로 유지\n\
- 도메인은 중복 없이 정규화\n\
- type은 action|calc만 허용\n\
- scope는 루트 상대 경로\n\
- depends_on은 선행 task의 name 배열\n\
- 기존 값은 가능한 유지하고 부족한 부분만 채움\n\
\n\
domain_candidates:\n\
{{domain_candidates}}\n\
\n\
<<<<<<< HEAD
spec.yaml:\n\
=======
project_definition(from project.yaml):\n\
{{project_definition}}\n\
\n\
design_template(from /home/tree/ai/skills/plan-code/references/plan.md):\n\
{{design_template}}\n\
\n\
tasks.yaml + features:\n\
>>>>>>> 5b2a204 (fix: seperate process)
{{spec_yaml}}\n";
    let template = std::fs::read_to_string("assets/prompts/Prompt_domain.txt")
        .unwrap_or_else(|_| fallback.to_string());
    template
        .replace("{{domain_candidates}}", &domain_candidates.join(", "))
<<<<<<< HEAD
=======
        .replace("{{project_definition}}", project_definition)
        .replace("{{design_template}}", design_template)
>>>>>>> 5b2a204 (fix: seperate process)
        .replace("{{spec_yaml}}", spec_text)
}

fn parse_spec_from_codex_output(raw: &str) -> Result<TaskSpecYaml> {
    let candidate = extract_yaml_candidate(raw);
    serde_yaml::from_str::<TaskSpecYaml>(&candidate)
        .map_err(|e| anyhow::anyhow!("failed to parse generated spec yaml: {e}"))
}

fn build_make_todos_prompt(spec_text: &str) -> String {
    let todos_template = load_todos_template_for_prompt();
    let allowed_domains = extract_allowed_domains_from_spec_text(spec_text);
    let allowed_domains_text = if allowed_domains.is_empty() {
        "(none)".to_string()
    } else {
        allowed_domains.join(", ")
    };
    format!(
        "현재 spec.yaml을 보고 각 tasks의 item을 바탕으로 todos.yaml 형식에 따른 todos.yaml 작성해줘.\n\
스킬 사용:\n\
- /home/tree/ai/skills/functional-code-structure/SKILL.md를 적용해 todo를 세부적으로 작성할것\n\
규칙:\n\
- todo는 \"대상 + 동작\" 형태의 작업 리스트로 작성\n\
- todo 항목 수량 제한 없음\n\
- todo step 설계 사고 절차:\n\
  1) 더이상 나눌 수 없는 작은 단위의 도메인을 떠올린다\n\
  2) 도메인이 하는 행동과 그에 따른 대상의 상태 변화를 고려한다\n\
  3) 조건이나 검증 규칙이 있을 경우 조건식을 건다\n\
  4) 도메인과 변화되는 변수들을 생각한다\n\
  5) 한번에 하나의 작업으로 사용되는 변수가 어떻게 변하는지 작성한다\n\
- 입력이 간단해도 결과 todo는 구체화할 것(검증/대기/직렬화/저장/전송/후처리 단계 포함)\n\
- rule은 단순 반복이 아니라 실행 가능한 제약으로 확장할 것(권한, 길이 제한, 상태 조건, 취소/수정 가능 여부, 완료 후 저장)\n\
- step은 UI/도메인 이벤트 순서를 따라 세분화할 것(선택 -> 검증 -> 입력대기 -> 확인/취소 분기 -> 저장/전송 -> 완료 반영)\n\
- 상태 전이 관점을 반영할 것(예: 선택됨, 입력중, 검증중, 전송대기, 완료)과 각 전이 조건을 step/rule에 드러낼 것\n\
- scope가 추상 키워드여도 실제 파일 후보로 구체화할 것(예: send message -> message.ts, friend.ts)\n\
- 각 task의 `rule`, `step`을 반드시 반영해 todo를 작성할 것\n\
- task의 `scope`가 비어 있으면 rule/step/name을 근거로 합리적인 파일 경로를 추론해 `scope`를 생성할 것\n\
- 추론한 scope 경로는 프로젝트 루트 기준 상대 경로로 작성할 것(예: src/task3/task3.rs)\n\
- 어떤 function이 다른 function의 결과/완료를 필요로 하면 해당 function의 `depends_on`에 의존 function 이름을 추가\n\
- domain은 spec.yaml의 features.domain 목록 중에서 선택해 `domain` 필드(string[])에 기록할 것\n\
- domain 후보가 여러 개면 현재 기능에 직접 영향 주는 도메인만 최소 집합으로 선택할 것\n\
- 완료 todos.yaml 파일에 덧붙것일것\n\
출력 형식:\n\
- 순수 YAML만 출력\n\
- 최상위 키는 tasks만 사용\n\
- 기존 파일 전체를 다시 쓰지 말고 append할 tasks 항목들만 작성\n\
- todos 항목의 키 이름은 `name,type,domain,depends_on,scope,state,rule,step`를 사용\n\
- 속성 역할 정의:\n\
  - name: 작업의 목적이 드러나는 기능명(사람이 읽고 이해 가능한 이름)\n\
  - type: 작업 성격(`action`=외부상태 변경/입출력, `calc`=순수 계산)\n\
  - domain: 기능이 영향을 받는 도메인 목록(spec.features.domain 후보 중 선택)\n\
  - depends_on: 선행 완료가 필요한 다른 task의 name 목록(없으면 빈 배열)\n\
  - scope: 수정/생성 대상 파일 경로 목록(프로젝트 루트 기준 상대 경로)\n\
  - state: 기능이 거치는 주요 상태 목록(예: 선택됨, 검증중, 전송대기, 완료)\n\
  - rule: 반드시 지켜야 하는 검증/제약/정책 목록\n\
  - step: 실제 수행 순서대로 쪼갠 작업 단계 목록\n\
- todos item 스키마:\n\
  - name: string\n\
  - type: action|calc\n\
  - domain: string[]\n\
  - depends_on: string[]\n\
  - scope: string[]\n\
  - state: string[]\n\
  - rule: string[]\n\
  - step: string[]\n\n\
allowed_domains(from spec.features.domain): [{allowed_domains_text}]\n\n\
todos.yaml template:\n{todos_template}\n\n\
spec.yaml:\n{spec_text}"
    )
}

fn execute_codex_prompt(prompt: &str) -> Result<String> {
    let seq = CODEX_OUTPUT_SEQ.fetch_add(1, Ordering::Relaxed);
    let output_path = std::env::temp_dir().join(format!(
        "orchestra_make_todos_spec_{}_{}_{}_{}.txt",
        std::process::id(),
        seq,
        format!("{:?}", thread::current().id()).replace(['(', ')', ' '], ""),
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
        .map_err(|e| anyhow::anyhow!("failed to execute codex: {e}"))?;

    let fallback_stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let text = std::fs::read_to_string(&output_path).unwrap_or(fallback_stdout);
    let _ = std::fs::remove_file(&output_path);

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "codex exited with code {}",
            output.status.code().unwrap_or(-1)
        ));
    }
    Ok(text)
}

fn parse_todos_from_codex_output(raw: &str) -> Result<TaskSpecYaml> {
    let candidate = extract_yaml_candidate(raw);
    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    struct TodosLikeYaml {
        #[serde(default)]
        tasks: Vec<TaskSpecItem>,
        #[serde(default)]
        todos: Vec<TaskSpecItem>,
    }
    let parsed = serde_yaml::from_str::<TodosLikeYaml>(&candidate)
        .map_err(|e| anyhow::anyhow!("failed to parse generated yaml: {e}"))?;
    let tasks = if parsed.tasks.is_empty() {
        parsed.todos
    } else {
        parsed.tasks
    };
    Ok(TaskSpecYaml {
        tasks,
        ..TaskSpecYaml::default()
    })
}

fn load_todos_items(path: &std::path::Path) -> Vec<TaskSpecItem> {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    struct TodosYaml {
        #[serde(default)]
        tasks: Vec<TaskSpecItem>,
        #[serde(default)]
        todos: Vec<TaskSpecItem>,
    }
    let Ok(parsed) = serde_yaml::from_str::<TodosYaml>(&raw) else {
        return Vec::new();
    };
    if parsed.tasks.is_empty() { parsed.todos } else { parsed.tasks }
}

fn load_tasks_items(path: &std::path::Path) -> Vec<TaskSpecItem> {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(parsed) = serde_yaml::from_str::<TasksYaml>(&raw) else {
        return Vec::new();
    };
    parsed.tasks
}

fn load_tasks_items(path: &std::path::Path) -> Vec<TaskSpecItem> {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(parsed) = serde_yaml::from_str::<TasksYaml>(&raw) else {
        return Vec::new();
    };
    parsed.tasks
}

fn stage_save_todos_spec(pane_task_spec: &mut PaneTaskSpec) {
    #[derive(Debug, Clone, Serialize)]
    struct TodosYaml<'a> {
        tasks: &'a [TaskSpecItem],
    }
    let yaml = match serde_yaml::to_string(&TodosYaml {
        tasks: &pane_task_spec.todos,
    }) {
        Ok(v) => v,
        Err(err) => {
            pane_task_spec.status = format!("todos serialize failed: {err}");
            return;
        }
    };
    if let Err(err) = serde_yaml::from_str::<serde_yaml::Value>(&yaml) {
        pane_task_spec.status = format!("todos yaml invalid: {err}");
        return;
    }
    if let Err(err) = std::fs::write(&pane_task_spec.todos_path, yaml) {
        pane_task_spec.status = format!("todos save failed: {err}");
    }
}

fn load_todos_template_for_prompt() -> String {
    let path = std::path::Path::new("assets/templates/todos.yaml");
    std::fs::read_to_string(path).unwrap_or_else(|_| {
        "tasks:\n  - name: \"\"\n    type: \"\"\n    domain: []\n    depends_on: []\n    scope: []\n    state: []\n    rule: []\n    step: []\n".to_string()
    })
}

fn load_plan_code_design_template_for_prompt() -> String {
    let path = std::path::Path::new("/home/tree/ai/skills/plan-code/references/plan.md");
    std::fs::read_to_string(path).unwrap_or_else(|_| {
        "# Design Document\n\
\n\
## Domain\n\
- **Subject**:\n\
- **Core Object**:\n\
- **States**:\n\
- **Actions**:\n\
\n\
## Flow (1st Iteration)\n\
- **State Transition**:\n\
- **Object Interaction**:\n\
- **Available Actions**:\n\
\n\
## Context\n\
- **Tech Stack**:\n\
- **Reusable Components**:\n\
- **Style Patterns**:\n\
\n\
## Constraints\n\
- **Business Rules**:\n\
\n\
## Verification\n\
- **Done Criteria**:\n\
\n\
## Next Iterations\n\
-\n"
            .to_string()
    })
}

fn extract_allowed_domains_from_spec_text(spec_text: &str) -> Vec<String> {
    let parsed = serde_yaml::from_str::<TaskSpecYaml>(spec_text).ok();
    parsed
        .map(|v| v.features.domain)
        .unwrap_or_default()
        .into_iter()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
}

fn deserialize_string_or_vec<'de, D>(deserializer: D) -> std::result::Result<Vec<String>, D::Error>
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
        StringOrVec::One(v) => Ok(if v.trim().is_empty() {
            Vec::new()
        } else {
            vec![v]
        }),
        StringOrVec::Many(vs) => Ok(vs),
    }
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

fn join_items_with_bar(values: &[String]) -> String {
    if values.is_empty() {
        "-".to_string()
    } else {
        values.join(" | ")
    }
}

fn load_working_theme() -> WorkingTheme {
    let path = std::path::Path::new("configs/style.yaml");
    let fallback = WorkingTheme {
        primary: Color::Rgb(18, 16, 16),
        secondary: Color::Rgb(105, 86, 86),
<<<<<<< HEAD
=======
        active: Color::Rgb(235, 160, 175),
        inactive: Color::DarkGray,
        focus: Color::Rgb(235, 160, 175),
>>>>>>> 5b2a204 (fix: seperate process)
        background: Color::Rgb(250, 227, 222),
        margin: 2,
        padding: 1,
        state_ready: "⯈".to_string(),
        state_running: "⯀".to_string(),
        state_done: "⬤".to_string(),
    };

    let content = match std::fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return fallback,
    };
    let parsed: StyleConfig = match serde_yaml::from_str(&content) {
        Ok(v) => v,
        Err(_) => return fallback,
    };

    WorkingTheme {
        primary: parse_hex_color(&parsed.basic.primary).unwrap_or(fallback.primary),
        secondary: parse_hex_color(&parsed.basic.secondary).unwrap_or(fallback.secondary),
<<<<<<< HEAD
=======
        active: parse_hex_color(&parsed.basic.active).unwrap_or(fallback.active),
        inactive: parse_hex_color(&parsed.basic.inactive).unwrap_or(fallback.inactive),
        focus: parse_hex_color(&parsed.basic.focus).unwrap_or(fallback.focus),
>>>>>>> 5b2a204 (fix: seperate process)
        background: parse_hex_color(&parsed.basic.background).unwrap_or(fallback.background),
        margin: parsed.layout.margin,
        padding: parsed.layout.padding,
        state_ready: if parsed.symbol.state.ready.trim().is_empty() {
            fallback.state_ready
        } else {
            parsed.symbol.state.ready
        },
        state_running: if parsed.symbol.state.running.trim().is_empty() {
            fallback.state_running
        } else {
            parsed.symbol.state.running
        },
        state_done: if parsed.symbol.state.done.trim().is_empty() {
            fallback.state_done
        } else {
            parsed.symbol.state.done
        },
    }
}

fn parse_hex_color(input: &str) -> Option<Color> {
    let hex = input.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

fn working_area(area: Rect, theme: &WorkingTheme) -> Rect {
    let m = theme.margin;
    Rect::new(
        area.x.saturating_add(m),
        area.y.saturating_add(m),
        area.width.saturating_sub(m.saturating_mul(2)),
        area.height.saturating_sub(m.saturating_mul(2)),
    )
}
<<<<<<< HEAD
=======

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_spec_from_codex_output_accepts_framework_sequence() {
        let raw = "요약 문장\n```yaml\nname: demo\ndescription: sample\nframework:\n  - rust\nrule: []\nfeatures:\n  domain: []\n  feature: []\ntasks: []\n```";
        let parsed = parse_spec_from_codex_output(raw).expect("spec parse should succeed");
        assert_eq!(parsed.framework, "rust");
    }

    #[test]
    fn parse_spec_from_codex_output_accepts_text_prefix_without_fence() {
        let raw = "설명 문장\nname: demo\ndescription: sample\nframework: rust\nrule: []\nfeatures:\n  domain: []\n  feature: []\ntasks: []\n";
        let parsed = parse_spec_from_codex_output(raw).expect("spec parse should succeed");
        assert_eq!(parsed.name, "demo");
        assert_eq!(parsed.framework, "rust");
    }

    #[test]
    fn parse_spec_from_codex_output_accepts_task_scalar_fields() {
        let raw = "name: demo\ndescription: sample\nframework: rust\nrule: []\nfeatures:\n  domain: []\n  feature: []\ntasks:\n  - name: t1\n    type: action\n    domain: catalog\n    depends_on: prev_task\n    scope: src/app\n    state: todo\n    rule: must-pass\n    step: do-work\n";
        let parsed = parse_spec_from_codex_output(raw).expect("spec parse should succeed");
        let task = parsed.tasks.first().expect("task should exist");
        assert_eq!(task.domain, vec!["catalog".to_string()]);
        assert_eq!(task.depends_on, vec!["prev_task".to_string()]);
        assert_eq!(task.scope, vec!["src/app".to_string()]);
        assert_eq!(task.state, vec!["todo".to_string()]);
        assert_eq!(task.rule, vec!["must-pass".to_string()]);
        assert_eq!(task.step, vec!["do-work".to_string()]);
    }

    #[test]
    fn parse_spec_from_codex_output_accepts_features_with_nested_mapping_values() {
        let raw = "name: demo\ndescription: sample\nframework: rust\nrule: []\nfeatures:\n  domain:\n    primary: catalog\n  feature:\n    - name: product.list\ntasks: []\n";
        let parsed = parse_spec_from_codex_output(raw).expect("spec parse should succeed");
        assert_eq!(parsed.features.domain, vec!["catalog".to_string()]);
        assert_eq!(parsed.features.feature, vec!["product.list".to_string()]);
    }

    #[test]
    fn split_line_items_parses_one_item_per_line() {
        let parsed = split_line_items(" keep auth \n\nlimit length\n");
        assert_eq!(
            parsed,
            vec!["keep auth".to_string(), "limit length".to_string()]
        );
    }

    #[test]
    fn count_request_input_lines_counts_wrapped_lines() {
        assert_eq!(count_request_input_lines("abcdef", 4), 2);
        assert_eq!(count_request_input_lines("ab\ncdef", 4), 2);
    }
}
>>>>>>> 5b2a204 (fix: seperate process)
