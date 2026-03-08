use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

pub(crate) fn render_tab_header(
    f: &mut ratatui::Frame,
    area: Rect,
    tab_index: usize,
    active_color: Color,
    inactive_color: Color,
    border_color: Color,
    right_hint: &str,
) {
    let header = Line::from(vec![
        Span::styled(
            "Project",
            if tab_index == 0 {
                Style::default()
                    .fg(active_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(inactive_color)
            },
        ),
        " | ".into(),
        Span::styled(
            "Detail",
            if tab_index == 1 {
                Style::default()
                    .fg(active_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(inactive_color)
            },
        ),
    ]);
    let header_block = Block::default()
        .title("Current Pane")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let header_inner = header_block.inner(area);
    f.render_widget(header_block, area);
    let header_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(header_inner);
    f.render_widget(Paragraph::new(header), header_layout[0]);
    f.render_widget(
        Paragraph::new(right_hint).alignment(Alignment::Right),
        header_layout[1],
    );
}

pub(crate) fn render_confirm_buttons_bottom_right(
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

pub(crate) fn render_confirm_cancel_wrapper(
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
    render_confirm_buttons_bottom_right(
        f,
        inner,
        confirm_label,
        cancel_label,
        confirm_selected,
    );
}

pub(crate) fn render_busy_modal(f: &mut ratatui::Frame, area: Rect, message: &str) {
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

pub(crate) fn render_alarm_modal(f: &mut ratatui::Frame, area: Rect, message: &str) {
    f.render_widget(Clear, area);
    let block = Block::default().title("Alarm").borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    f.render_widget(
        Paragraph::new(vec![Line::from(message.to_string())])
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false }),
        rows[0],
    );
    let button = Span::styled(
        "[확인]",
        Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED),
    );
    f.render_widget(
        Paragraph::new(Line::from(button)).alignment(Alignment::Center),
        rows[1],
    );
}

pub(crate) struct LlmChatPaneView<'a> {
    pub project_name: &'a str,
    pub history: &'a [String],
    pub streaming: bool,
    pub warmup_inflight: bool,
    pub response_scroll: u16,
    pub hint: &'a str,
    pub input: &'a str,
    pub input_border_style: Style,
    pub close_button_focused: bool,
    pub input_active_for_cursor: bool,
}

pub(crate) fn render_llm_chat_pane(
    f: &mut ratatui::Frame,
    area: Rect,
    view: &LlmChatPaneView<'_>,
) -> Option<Rect> {
    f.render_widget(Clear, area);
    let block = Block::default()
        .title(format!("AI Detail - {}", view.project_name))
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

    let mut lines: Vec<Line> = view
        .history
        .iter()
        .flat_map(|msg| {
            let out = vec![Line::from(msg.clone()), Line::from("")];
            out
        })
        .collect();
    if view.streaming && !view.warmup_inflight {
        lines.push(Line::from("AI 응답 생성중..."));
    }
    if lines.is_empty() {
        lines.push(Line::from(""));
    }
    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().title("Response").borders(Borders::ALL))
            .scroll((view.response_scroll, 0))
            .wrap(Wrap { trim: false }),
        split[0],
    );

    f.render_widget(
        Paragraph::new(view.input.to_string())
            .block(
                Block::default()
                    .title(format!("Input | {}", view.hint))
                    .borders(Borders::ALL)
                    .border_style(view.input_border_style),
            )
            .wrap(Wrap { trim: false }),
        split[1],
    );

    let button_style = if view.close_button_focused {
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

    if view.input_active_for_cursor {
        Some(split[1])
    } else {
        None
    }
}
