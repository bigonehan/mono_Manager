use std::time::Duration;

use anyhow::Result;
use crossterm::event::DisableMouseCapture;
use crossterm::event::EnableMouseCapture;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Cell, Padding, Paragraph, Row, Table};
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
    background: Color,
    margin: u16,
    padding: u16,
    state_ready: String,
    state_running: String,
    state_done: String,
}

#[derive(Debug, Clone, Copy)]
enum PaneFocus {
    TaskSpec,
    Working,
}

#[derive(Debug, Clone, Copy)]
enum TaskSpecMode {
    List,
    Form,
}

#[derive(Debug, Clone)]
struct PaneTaskSpec {
    path: std::path::PathBuf,
    spec: TaskSpecYaml,
    selected_task: usize,
    mode: TaskSpecMode,
    selected_field: usize,
    input_mode: bool,
    input_buffer: String,
    status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TaskSpecYaml {
    #[serde(default)]
    tasks: Vec<TaskSpecItem>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct TaskSpecYamlCompat {
    tasks: Option<Vec<TaskSpecItem>>,
    todos: Option<Vec<TaskSpecItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TaskSpecItem {
    #[serde(default)]
    name: String,
    #[serde(default, rename = "type")]
    task_type: String,
    #[serde(default)]
    scope: Vec<String>,
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
    let mut focus = PaneFocus::Working;
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
    let mut run_requested = false;
    let mut tick = tokio::time::interval(Duration::from_millis(80));
    loop {
        stage_handle_pane_key_events(
            &mut focus,
            &mut pane_task_spec,
            &run_start_tx,
            &mut run_requested,
        )?;
        terminal.draw(|frame| {
            let area = working_area(frame.area(), &theme);
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);

            let task_active = matches!(focus, PaneFocus::TaskSpec);
            let working_active = matches!(focus, PaneFocus::Working);
            let task_border_style = if task_active {
                Style::default().fg(theme.secondary).bg(theme.background)
            } else {
                Style::default().fg(theme.primary)
            };
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
            let working_title_style = if working_active {
                Style::default().fg(theme.background).bg(theme.secondary)
            } else {
                Style::default().fg(theme.primary)
            };

            let mode_text = match pane_task_spec.mode {
                TaskSpecMode::List => "card-list",
                TaskSpecMode::Form => "form",
            };
            let task_block = Block::default()
                .title(
                    Line::from(format!("pane_task_spec ({mode_text}) | {}", pane_task_spec.status))
                        .style(task_title_style),
                )
                .borders(Borders::ALL)
                .border_style(task_border_style)
                .padding(Padding::uniform(theme.padding))
                .style(Style::default().fg(theme.primary));
            let task_inner = task_block.inner(chunks[0]);
            frame.render_widget(task_block, chunks[0]);
            match pane_task_spec.mode {
                TaskSpecMode::List => render_task_spec_cards(frame, task_inner, &pane_task_spec, &theme),
                TaskSpecMode::Form => render_task_spec_form(frame, task_inner, &pane_task_spec, &theme),
            }

            let header = Row::new(vec!["요청 기능", "결과값", "상태"])
                .style(Style::default().fg(theme.primary))
                .height(1);

            let ui_rows = rows
                .iter()
                .enumerate()
                .map(|(index, row)| {
                    let row_style = if index % 2 == 0 {
                        Style::default().fg(theme.primary)
                    } else {
                        Style::default().bg(theme.background).fg(theme.primary)
                    };
                    Row::new(vec![
                        Cell::from(row.request.clone()),
                        Cell::from(row.result.clone()),
                        Cell::from(Line::from(status_text(row.status, &theme)).alignment(Alignment::Right)),
                    ])
                    .style(row_style)
                    .height(1)
                })
                .collect::<Vec<_>>();

            let table = Table::new(
                ui_rows,
                [
                    Constraint::Percentage(40),
                    Constraint::Percentage(48),
                    Constraint::Percentage(12),
                ],
            )
            .header(header)
            .block(
                Block::default()
                    .title(Line::from("working").style(working_title_style))
                    .borders(Borders::ALL)
                    .border_style(working_border_style)
                    .padding(Padding::uniform(theme.padding))
                    .style(Style::default().fg(theme.primary)),
            );
            frame.render_widget(table, chunks[1]);
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
    let mut status = "Working pane focus + Enter: run | Up/Down: select task | Enter(on task): open form".to_string();
    let spec = match std::fs::read_to_string(&path) {
        Ok(raw) => match serde_yaml::from_str::<TaskSpecYamlCompat>(&raw) {
            Ok(parsed) => TaskSpecYaml {
                tasks: parsed.tasks.or(parsed.todos).unwrap_or_default(),
            },
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

    PaneTaskSpec {
        path,
        spec,
        selected_task: 0,
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
            Paragraph::new("tasks.yaml에 task가 없습니다."),
            area,
        );
        return;
    }

    let card_height: u16 = 6;
    let max_cards = std::cmp::max(1, usize::from((area.height / card_height).max(1)));
    let end = std::cmp::min(pane_task_spec.spec.tasks.len(), pane_task_spec.selected_task + 1);
    let start = end.saturating_sub(max_cards);
    let visible_end = std::cmp::min(pane_task_spec.spec.tasks.len(), start + max_cards);

    let mut constraints = Vec::new();
    for _ in start..visible_end {
        constraints.push(Constraint::Length(card_height));
    }
    if constraints.is_empty() {
        constraints.push(Constraint::Length(card_height));
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for (card_slot, task_index) in (start..visible_end).enumerate() {
        let task = &pane_task_spec.spec.tasks[task_index];
        let selected = task_index == pane_task_spec.selected_task;
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

        let content = format!(
            "name: {}\ntype: {}\nscope: {}\nrule: {}",
            task.name,
            task.task_type,
            join_items(&task.scope),
            join_items(&task.rule),
        );
        let card = Paragraph::new(content).block(
            Block::default()
                .title(Line::from(format!("task {}", task_index + 1)).style(title_style))
                .borders(Borders::ALL)
                .border_style(border_style)
                .padding(Padding::new(1, 1, 0, 0)),
        );
        frame.render_widget(card, chunks[card_slot]);
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

fn stage_handle_pane_key_events(
    focus: &mut PaneFocus,
    pane_task_spec: &mut PaneTaskSpec,
    run_start_tx: &tokio::sync::mpsc::UnboundedSender<()>,
    run_requested: &mut bool,
) -> Result<()> {
    while crossterm::event::poll(Duration::from_millis(0))? {
        let event = crossterm::event::read()?;
        let Event::Key(key) = event else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
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
                KeyCode::Left => *focus = PaneFocus::TaskSpec,
                KeyCode::Right => *focus = PaneFocus::Working,
                KeyCode::Enter if matches!(focus, PaneFocus::Working) => {
                    if !*run_requested {
                        let _ = run_start_tx.send(());
                        *run_requested = true;
                        pane_task_spec.status = "run started".to_string();
                    }
                }
                _ => {}
            }
        }

        if !matches!(focus, PaneFocus::TaskSpec) {
            continue;
        }

        match pane_task_spec.mode {
            TaskSpecMode::List => match key.code {
                KeyCode::Up => {
                    pane_task_spec.selected_task = pane_task_spec.selected_task.saturating_sub(1);
                }
                KeyCode::Down => {
                    if pane_task_spec.selected_task + 1 < pane_task_spec.spec.tasks.len() {
                        pane_task_spec.selected_task += 1;
                    }
                }
                KeyCode::Enter => {
                    if pane_task_spec.spec.tasks.is_empty() {
                        pane_task_spec.status = "수정할 task가 없습니다.".to_string();
                    } else {
                        pane_task_spec.mode = TaskSpecMode::Form;
                        pane_task_spec.selected_field = 0;
                        pane_task_spec.status = "form mode".to_string();
                    }
                }
                _ => {}
            },
            TaskSpecMode::Form => match key.code {
                KeyCode::Esc => {
                    pane_task_spec.mode = TaskSpecMode::List;
                    pane_task_spec.selected_field = 0;
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

fn join_items(values: &[String]) -> String {
    if values.is_empty() {
        "-".to_string()
    } else {
        values.join(", ")
    }
}

fn join_items_with_bar(values: &[String]) -> String {
    if values.is_empty() {
        "-".to_string()
    } else {
        values.join(" | ")
    }
}

fn status_text<'a>(status: WorkingStatus, theme: &'a WorkingTheme) -> &'a str {
    match status {
        WorkingStatus::Ready => &theme.state_ready,
        WorkingStatus::Running => &theme.state_running,
        WorkingStatus::Done => &theme.state_done,
    }
}

fn load_working_theme() -> WorkingTheme {
    let path = std::path::Path::new("configs/style.yaml");
    let fallback = WorkingTheme {
        primary: Color::Rgb(18, 16, 16),
        secondary: Color::Rgb(105, 86, 86),
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
