use std::time::Duration;

use anyhow::Result;
use crossterm::event::DisableMouseCapture;
use crossterm::event::EnableMouseCapture;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use crossterm::execute;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Cell, Padding, Row, Table};
use serde::Deserialize;
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
    pane_width_percent: u16,
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
    pane_width_percent: u16,
    state_ready: String,
    state_running: String,
    state_done: String,
}

pub async fn stage_run_working_pane(
    worker_requests: Vec<String>,
    mut rx: UnboundedReceiver<WorkingPaneEvent>,
) -> Result<()> {
    let theme = load_working_theme();
    let mut rows = worker_requests
        .into_iter()
        .map(|request| WorkingRow {
            request,
            result: String::new(),
            status: WorkingStatus::Ready,
        })
        .collect::<Vec<_>>();

    let mut stdout = std::io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut should_finish = false;
    let mut tick = tokio::time::interval(Duration::from_millis(80));
    loop {
        terminal.draw(|frame| {
            let area = working_area(frame.area(), &theme);
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
                    .title("working")
                    .borders(Borders::ALL)
                    .padding(Padding::uniform(theme.padding))
                    .style(Style::default()),
            );
            frame.render_widget(table, area);
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
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    Ok(())
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
        pane_width_percent: 50,
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
        pane_width_percent: parsed.layout.pane_width_percent.clamp(30, 100),
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
    let outer = Rect::new(
        area.x.saturating_add(m),
        area.y.saturating_add(m),
        area.width.saturating_sub(m.saturating_mul(2)),
        area.height.saturating_sub(m.saturating_mul(2)),
    );
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - theme.pane_width_percent) / 2),
            Constraint::Percentage(theme.pane_width_percent),
            Constraint::Percentage((100 - theme.pane_width_percent) / 2),
        ])
        .split(outer);
    chunks[1]
}
