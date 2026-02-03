use super::{relative_time, styled_block, truncate, INFO, MUTED, SELECTED_BG, SUCCESS, WARNING};
use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::*,
    widgets::{
        Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};

pub fn draw_session_list(f: &mut Frame, app: &mut App, area: Rect, is_focused: bool) {
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

    // If renaming, show input at top
    if app.renaming {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Rename input
        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Rename (Enter to save, Esc to cancel) ")
            .title_style(Style::default().fg(Color::Yellow).bold());

        let input = Paragraph::new(app.rename_buffer.as_str())
            .block(input_block)
            .style(Style::default().fg(Color::White));
        f.render_widget(input, chunks[0]);

        // Position cursor at end of input
        let cursor_x = chunks[0].x + 1 + app.rename_buffer.chars().count() as u16;
        let cursor_y = chunks[0].y + 1;
        if cursor_x < chunks[0].x + chunks[0].width - 1 {
            f.set_cursor_position(ratatui::layout::Position::new(cursor_x, cursor_y));
        }

        // Draw session list in remaining space
        draw_session_list_inner(f, app, chunks[1], is_focused);
        return;
    }

    draw_session_list_inner(f, app, area, is_focused);
}

fn draw_session_list_inner(f: &mut Frame, app: &mut App, area: Rect, is_focused: bool) {
    let block = styled_block("Sessions", is_focused);
    let max_name_width = (area.width as usize).saturating_sub(4).min(25);

    let items: Vec<ListItem> = app
        .sessions
        .iter()
        .enumerate()
        .map(|(i, session)| {
            let is_selected = app.session_list_state.selected() == Some(i);

            // More distinct status indicators
            let (status_char, status_color) = match session.status.as_str() {
                "working" => ("⟳", Color::Cyan), // Cyan spinner = actively processing (<10s)
                "active" => ("▶", Color::Green), // Green play = recent activity (<2 min)
                "idle" => ("●", Color::Yellow),  // Yellow dot = waiting (2-30 min)
                "inactive" => ("○", Color::DarkGray), // Gray circle = old (>30 min)
                "waiting" => ("◆", Color::Magenta), // Magenta = waiting for user (from hook)
                _ => ("○", Color::DarkGray),
            };

            // Use custom_name > description > project_name
            let display_name = session
                .custom_name
                .as_ref()
                .or(session.description.as_ref())
                .map(|d| truncate(d, max_name_width))
                .unwrap_or_else(|| session.project_name.clone());

            let content = Line::from(vec![
                Span::styled(status_char, Style::default().fg(status_color)),
                Span::raw(" "),
                Span::styled(
                    truncate(&display_name, max_name_width),
                    Style::default().fg(if is_selected {
                        Color::White
                    } else {
                        Color::Gray
                    }),
                ),
            ]);

            let time_line = Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    relative_time(&session.last_activity),
                    Style::default().fg(MUTED).italic(),
                ),
                Span::styled(
                    format!(" {} msgs", session.message_count),
                    Style::default().fg(MUTED),
                ),
            ]);

            ListItem::new(vec![content, time_line]).style(if is_selected {
                Style::default().bg(SELECTED_BG)
            } else {
                Style::default()
            })
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(SELECTED_BG));

    f.render_stateful_widget(list, area, &mut app.session_list_state);
}

pub fn draw_detail_view(f: &mut Frame, app: &mut App, area: Rect, is_focused: bool) {
    // If embedded terminal is active, show it full screen
    if app.terminal_mode && app.embedded_terminal.is_some() {
        draw_embedded_terminal(f, app, area);
        return;
    }

    // Show diff view when in diff mode OR when Files is focused (preview)
    if app.diff_mode || app.focus == crate::app::Focus::Files {
        draw_diff_view(f, app, area, is_focused);
    } else if app.focus == crate::app::Focus::Todos {
        // Show todos preview when Todos panel is focused
        draw_todos_preview(f, app, area);
    } else {
        // Layout: header + chat
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header (with border)
                Constraint::Min(10),   // Chat
            ])
            .split(area);

        draw_session_header(f, app, chunks[0]);
        draw_messages(f, app, chunks[1], is_focused);
    }
}

fn draw_todos_preview(f: &mut Frame, app: &mut App, area: Rect) {
    let todos: Vec<_> = app
        .selected_session()
        .map(|s| s.todos.iter().collect())
        .unwrap_or_default();

    let title = format!("Todos ({})", todos.len());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(super::BORDER_COLOR))
        .title(format!(" {} ", title))
        .title_style(Style::default().fg(Color::White).bold());

    let inner = block.inner(area);
    f.render_widget(block, area);

    if todos.is_empty() {
        let empty = Paragraph::new("No todos")
            .style(Style::default().fg(MUTED))
            .alignment(Alignment::Center);
        f.render_widget(empty, inner);
        return;
    }

    // Sort: in_progress → pending → completed
    let mut sorted_todos = todos;
    sorted_todos.sort_by(|a, b| {
        let status_order = |s: &str| match s {
            "in_progress" => 0,
            "pending" => 1,
            "completed" => 2,
            _ => 1,
        };
        status_order(&a.status).cmp(&status_order(&b.status))
    });

    let max_width = inner.width.saturating_sub(4) as usize;
    let lines: Vec<Line> = sorted_todos
        .iter()
        .map(|todo| {
            let (icon, style) = match todo.status.as_str() {
                "in_progress" => ("▶", Style::default().fg(Color::Cyan)),
                "completed" => ("✓", Style::default().fg(MUTED)),
                _ => ("○", Style::default().fg(Color::Gray)),
            };

            Line::from(vec![
                Span::styled(format!(" {} ", icon), style),
                Span::styled(super::truncate(&todo.content, max_width), style),
            ])
        })
        .collect();

    // Calculate scroll
    let total_lines = lines.len() as u16;
    let visible_lines = inner.height;
    app.todos_scroll_max = total_lines.saturating_sub(visible_lines);

    let scroll_offset = app.todos_scroll.min(app.todos_scroll_max);
    let visible: Vec<Line> = lines
        .into_iter()
        .skip(scroll_offset as usize)
        .take(visible_lines as usize)
        .collect();

    let paragraph = Paragraph::new(visible);
    f.render_widget(paragraph, inner);
}

fn draw_diff_view(f: &mut Frame, app: &mut App, area: Rect, is_focused: bool) {
    let file = app.current_file_changes.get(app.selected_file_idx);
    let title = file
        .map(|f| f.path.clone())
        .unwrap_or_else(|| "No file selected".to_string());

    // Show active border only when actually in diff_mode (entered with Enter)
    let show_active = is_focused && app.diff_mode;

    let border_color = if show_active {
        super::BORDER_ACTIVE
    } else {
        super::BORDER_COLOR
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(format!(" {} ", title))
        .title_style(
            Style::default()
                .fg(if show_active {
                    Color::Green
                } else {
                    Color::White
                })
                .bold(),
        );

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.current_diff.is_empty() {
        let empty = Paragraph::new("No diff available\n\nSelect a file with j/k")
            .style(Style::default().fg(MUTED))
            .alignment(Alignment::Center);
        f.render_widget(empty, inner);
        return;
    }

    // Parse and colorize diff with line wrapping
    let max_width = inner.width as usize;
    let mut lines: Vec<Line> = Vec::new();

    for line in app.current_diff.lines() {
        let style = if line.starts_with('+') && !line.starts_with("+++") {
            Style::default().fg(Color::Green)
        } else if line.starts_with('-') && !line.starts_with("---") {
            Style::default().fg(Color::Red)
        } else if line.starts_with("@@") {
            Style::default().fg(Color::Cyan)
        } else if line.starts_with("diff") || line.starts_with("index") {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };

        // Wrap long lines
        if line.chars().count() <= max_width {
            lines.push(Line::from(Span::styled(line, style)));
        } else {
            let mut remaining = line;
            while !remaining.is_empty() {
                let (chunk, rest) = if remaining.chars().count() <= max_width {
                    (remaining, "")
                } else {
                    let byte_idx = remaining
                        .char_indices()
                        .nth(max_width)
                        .map(|(i, _)| i)
                        .unwrap_or(remaining.len());
                    (&remaining[..byte_idx], &remaining[byte_idx..])
                };
                lines.push(Line::from(Span::styled(chunk, style)));
                remaining = rest;
            }
        }
    }

    // Calculate scroll
    let total_lines = lines.len() as u16;
    let visible_lines = inner.height;
    app.chat_scroll_max = total_lines.saturating_sub(visible_lines);

    let scroll_offset = app.chat_scroll.min(app.chat_scroll_max);
    let visible: Vec<Line> = lines
        .into_iter()
        .skip(scroll_offset as usize)
        .take(visible_lines as usize)
        .collect();

    let paragraph = Paragraph::new(visible);
    f.render_widget(paragraph, inner);
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
            let mut spans = vec![
                Span::styled(" ", Style::default()),
                Span::styled(&s.project, Style::default().fg(Color::White).bold()),
                Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
                Span::styled(truncate(&s.id, 10), Style::default().fg(Color::DarkGray)),
                Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{} msgs", s.message_count),
                    Style::default().fg(INFO),
                ),
            ];

            if !s.todos.is_empty() {
                spans.push(Span::styled("  │  ", Style::default().fg(Color::DarkGray)));
                spans.push(Span::styled(
                    format!("{} todos", s.todos.len()),
                    Style::default().fg(WARNING),
                ));
            }

            spans.push(Span::styled(" ", Style::default()));
            Line::from(spans)
        }
        None => Line::from(Span::styled(
            " No session selected ",
            Style::default().fg(MUTED),
        )),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .border_type(ratatui::widgets::BorderType::Rounded);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let paragraph = Paragraph::new(content).alignment(Alignment::Left);
    f.render_widget(paragraph, inner);
}

fn draw_messages(f: &mut Frame, app: &mut App, area: Rect, is_focused: bool) {
    let session = app.selected_session();
    let title = match session {
        Some(s) => format!("Chat - {}", s.project_name),
        None => "Chat".to_string(),
    };

    let block = styled_block(&title, is_focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.messages_loading {
        let loading = Paragraph::new("Loading...")
            .style(Style::default().fg(MUTED))
            .alignment(Alignment::Center);
        f.render_widget(loading, inner);
        return;
    }

    if app.current_messages.is_empty() {
        let empty = Paragraph::new("No messages\n\nPress 'o' to open Claude")
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
                msg.timestamp
                    .map(|t| t.format("%H:%M").to_string())
                    .unwrap_or_default(),
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
            lines.push(Line::from(vec![Span::raw("  "), Span::styled(line, style)]));
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

    let visible: Vec<Line> = lines
        .into_iter()
        .skip(start_line)
        .take(visible_lines as usize)
        .collect();

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
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}
