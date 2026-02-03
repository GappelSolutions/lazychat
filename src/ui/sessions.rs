use crate::app::{App, Focus, InputMode};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use super::{styled_block, relative_time, truncate, status_style, MUTED, SELECTED_BG, INFO, SUCCESS, WARNING};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(area);

    draw_session_list(f, app, chunks[0]);
    draw_chat_view(f, app, chunks[1]);
}

fn draw_session_list(f: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.focus == Focus::List;
    let title = format!("Sessions ({})", app.sessions.len());
    let block = styled_block(&title, is_focused);

    if app.sessions.is_empty() {
        let empty = Paragraph::new("No sessions found")
            .block(block)
            .style(Style::default().fg(MUTED))
            .alignment(Alignment::Center);
        f.render_widget(empty, area);
        return;
    }

    let max_name_width = (area.width as usize).saturating_sub(12).min(20);

    let items: Vec<ListItem> = app
        .sessions
        .iter()
        .enumerate()
        .map(|(i, session)| {
            let is_selected = app.session_list_state.selected() == Some(i);

            let (status_char, status_text) = match session.status.as_str() {
                "active" => ("●", "[active]"),
                "idle" => ("○", "[idle]"),
                _ => ("◌", ""),
            };

            let content = Line::from(vec![
                Span::styled(status_char, status_style(&session.status)),
                Span::raw(" "),
                Span::styled(
                    truncate(&session.project_name, max_name_width.saturating_sub(10)),
                    Style::default().fg(if is_selected { Color::White } else { Color::Gray }),
                ),
                Span::raw(" "),
                Span::styled(status_text, status_style(&session.status)),
            ]);

            let time_line = Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    relative_time(&session.last_activity),
                    Style::default().fg(MUTED).italic(),
                ),
                Span::styled(
                    format!(" • {} msgs", session.message_count),
                    Style::default().fg(MUTED),
                ),
            ]);

            ListItem::new(vec![content, time_line]).style(
                if is_selected {
                    Style::default().bg(SELECTED_BG)
                } else {
                    Style::default()
                }
            )
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(SELECTED_BG).fg(Color::White));

    f.render_stateful_widget(list, area, &mut app.session_list_state);
}

fn draw_chat_view(f: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.focus == Focus::Detail;

    // If embedded terminal is active, show it
    if app.terminal_mode && app.embedded_terminal.is_some() {
        draw_embedded_terminal(f, app, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    draw_session_header(f, app, chunks[0]);
    draw_messages(f, app, chunks[1], is_focused);
}

fn draw_embedded_terminal(f: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(" Claude (Ctrl+q to exit) ")
        .title_style(Style::default().fg(Color::Green).bold());

    let inner = block.inner(area);
    f.render_widget(block, area);

    let _ = app.resize_terminal(inner.width, inner.height);

    if let Some(ref term) = app.embedded_terminal {
        if let Some(screen) = term.get_screen_with_styles() {
            let lines: Vec<Line> = screen
                .iter()
                .take(inner.height as usize)
                .map(|row| {
                    let spans: Vec<Span> = row
                        .iter()
                        .take(inner.width as usize)
                        .map(|(ch, fg, bg, bold)| {
                            let fg_color = vt100_to_ratatui_color(*fg);
                            let bg_color = vt100_to_ratatui_color(*bg);
                            let mut style = Style::default().fg(fg_color).bg(bg_color);
                            if *bold {
                                style = style.bold();
                            }
                            Span::styled(ch.to_string(), style)
                        })
                        .collect();
                    Line::from(spans)
                })
                .collect();

            let paragraph = Paragraph::new(lines);
            f.render_widget(paragraph, inner);

            if let Some((row, col)) = term.cursor_position() {
                let cursor_x = inner.x + col;
                let cursor_y = inner.y + row;
                if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
                    f.set_cursor_position(ratatui::layout::Position::new(cursor_x, cursor_y));
                }
            }
        }
    }
}

fn vt100_to_ratatui_color(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(0) => Color::Black,
        vt100::Color::Idx(1) => Color::Red,
        vt100::Color::Idx(2) => Color::Green,
        vt100::Color::Idx(3) => Color::Yellow,
        vt100::Color::Idx(4) => Color::Blue,
        vt100::Color::Idx(5) => Color::Magenta,
        vt100::Color::Idx(6) => Color::Cyan,
        vt100::Color::Idx(7) => Color::Gray,
        vt100::Color::Idx(8) => Color::DarkGray,
        vt100::Color::Idx(9) => Color::LightRed,
        vt100::Color::Idx(10) => Color::LightGreen,
        vt100::Color::Idx(11) => Color::LightYellow,
        vt100::Color::Idx(12) => Color::LightBlue,
        vt100::Color::Idx(13) => Color::LightMagenta,
        vt100::Color::Idx(14) => Color::LightCyan,
        vt100::Color::Idx(15) => Color::White,
        vt100::Color::Idx(n) => Color::Indexed(n),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

fn draw_session_header(f: &mut Frame, app: &App, area: Rect) {
    let session = app.selected_session();

    let content = match session {
        Some(s) => {
            Line::from(vec![
                Span::styled("📁 ", Style::default()),
                Span::styled(&s.project, Style::default().fg(Color::White).bold()),
                Span::raw("  "),
                Span::styled("ID: ", Style::default().fg(MUTED)),
                Span::styled(truncate(&s.id, 12), Style::default().fg(Color::Gray)),
                Span::raw("  "),
                Span::styled("Messages: ", Style::default().fg(MUTED)),
                Span::styled(s.message_count.to_string(), Style::default().fg(INFO)),
                Span::raw("  "),
                Span::styled(&s.status, status_style(&s.status)),
            ])
        }
        None => {
            Line::from(Span::styled("No session selected", Style::default().fg(MUTED)))
        }
    };

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

fn draw_messages(f: &mut Frame, app: &mut App, area: Rect, is_focused: bool) {
    let session = app.selected_session();
    let title = match session {
        Some(s) => format!("Chat - {} ({} msgs)", s.project_name, app.current_messages.len()),
        None => "Chat".to_string(),
    };

    let block = styled_block(&title, is_focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.messages_loading {
        let loading = Paragraph::new("Loading messages...")
            .style(Style::default().fg(MUTED))
            .alignment(Alignment::Center);
        f.render_widget(loading, inner);
        return;
    }

    if app.current_messages.is_empty() {
        let empty = Paragraph::new("No messages yet\n\nPress 'o' to open Claude session")
            .style(Style::default().fg(MUTED))
            .alignment(Alignment::Center);
        f.render_widget(empty, inner);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    let content_width = inner.width.saturating_sub(4) as usize;

    for msg in &app.current_messages {
        let (role_style, prefix) = if msg.role == "user" {
            (Style::default().fg(Color::Cyan).bold(), "▶ You")
        } else {
            (Style::default().fg(Color::Green).bold(), "◀ Claude")
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, role_style),
            Span::raw(" "),
            Span::styled(
                msg.timestamp.map(|t| t.format("%H:%M").to_string()).unwrap_or_default(),
                Style::default().fg(MUTED),
            ),
        ]));

        let display_lines = msg.display_content(content_width);
        for line in display_lines {
            let style = if msg.role == "user" {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::Gray)
            };
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(line, style),
            ]));
        }

        for tool in &msg.tool_calls {
            let tool_style = match tool.status.as_str() {
                "completed" => Style::default().fg(SUCCESS),
                "error" => Style::default().fg(Color::Red),
                _ => Style::default().fg(WARNING),
            };
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("└─ ", Style::default().fg(MUTED)),
                Span::styled(&tool.tool_name, tool_style),
            ]));
        }

        lines.push(Line::from(""));
    }

    let total_lines = lines.len() as u16;
    let visible_lines = inner.height;
    app.chat_scroll_max = total_lines.saturating_sub(visible_lines);

    let scroll_offset = app.chat_scroll;
    let start_line = if total_lines <= visible_lines {
        0
    } else {
        (total_lines - visible_lines).saturating_sub(scroll_offset) as usize
    };

    let visible: Vec<Line> = lines.into_iter().skip(start_line).take(visible_lines as usize).collect();

    let paragraph = Paragraph::new(visible);
    f.render_widget(paragraph, inner);

    if app.chat_scroll_max > 0 {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        let mut scrollbar_state = ScrollbarState::new(app.chat_scroll_max as usize)
            .position((app.chat_scroll_max - app.chat_scroll) as usize);

        f.render_stateful_widget(
            scrollbar,
            area.inner(Margin { vertical: 1, horizontal: 0 }),
            &mut scrollbar_state,
        );
    }
}
