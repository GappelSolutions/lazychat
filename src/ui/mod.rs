mod sessions;

use crate::app::{App, Focus};
use crate::config::Theme;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph},
};

// Fallback colors when no config available
pub const BORDER_COLOR: Color = Color::Blue;
pub const BORDER_ACTIVE: Color = Color::Green;
pub const MUTED: Color = Color::DarkGray;
pub const SELECTED_BG: Color = Color::Rgb(30, 50, 80);
pub const SUCCESS: Color = Color::Green;
pub const WARNING: Color = Color::Yellow;
pub const ERROR: Color = Color::Red;
pub const INFO: Color = Color::Cyan;

// Theme-aware color helpers
pub fn border_color(theme: &Theme) -> Color {
    theme.border()
}

pub fn border_active(theme: &Theme) -> Color {
    theme.border_active()
}

pub fn selected_bg(theme: &Theme) -> Color {
    theme.selected_bg()
}

pub fn status_color(theme: &Theme, status: &str) -> Color {
    match status {
        "working" => theme.status_working(),
        "active" => theme.status_active(),
        "idle" => theme.status_idle(),
        "inactive" => theme.status_inactive(),
        "waiting" => theme.status_waiting(),
        _ => theme.status_inactive(),
    }
}

pub fn diff_add(theme: &Theme) -> Color {
    theme.diff_add()
}

pub fn diff_remove(theme: &Theme) -> Color {
    theme.diff_remove()
}

pub fn diff_hunk(theme: &Theme) -> Color {
    theme.diff_hunk()
}

pub fn text_muted(theme: &Theme) -> Color {
    theme.text_muted()
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Layout: main content + footer (help)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Content
            Constraint::Length(1), // Help bar
        ])
        .split(size);

    if app.fullscreen {
        // Fullscreen: only show detail view
        let is_detail_focused = app.focus == Focus::Detail;
        sessions::draw_detail_view(f, app, chunks[0], is_detail_focused);
    } else {
        // Main layout: left panel (40%) + detail (60%)
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Percentage(60),
            ])
            .split(chunks[0]);

        // Left side: sessions + files + todos
        let is_sessions_focused = app.focus == Focus::Sessions;
        draw_left_panel(f, app, main_chunks[0], is_sessions_focused);

        // Right side: chat or diff
        let is_detail_focused = app.focus == Focus::Detail;
        sessions::draw_detail_view(f, app, main_chunks[1], is_detail_focused);
    }

    // Draw help bar
    draw_help_bar(f, app, chunks[1]);

    // Draw help popup if active
    if app.show_help {
        draw_help_popup(f, app, size);
    }
}

fn draw_left_panel(f: &mut Frame, app: &mut App, area: Rect, _is_focused: bool) {
    // Get todos for the SELECTED session only
    let mut session_todos: Vec<(String, String, String)> = app.selected_session()
        .map(|s| s.todos.iter().map(|t| (t.id.clone(), t.content.clone(), t.status.clone())).collect())
        .unwrap_or_default();

    // Sort by state first (in_progress → pending → completed), then by ID within each state
    session_todos.sort_by(|a, b| {
        let status_order = |s: &str| match s {
            "in_progress" => 0,
            "pending" => 1,
            "completed" => 2,
            _ => 1,
        };
        let status_cmp = status_order(&a.2).cmp(&status_order(&b.2));
        if status_cmp == std::cmp::Ordering::Equal {
            let id_a: i64 = a.0.parse().unwrap_or(i64::MAX);
            let id_b: i64 = b.0.parse().unwrap_or(i64::MAX);
            id_a.cmp(&id_b)
        } else {
            status_cmp
        }
    });

    // Convert to (content, status) for display
    let session_todos_display: Vec<(String, String)> = session_todos
        .into_iter()
        .map(|(_, content, status)| (content, status))
        .collect();

    let has_todos = !session_todos_display.is_empty();
    let has_files = !app.current_file_changes.is_empty();

    // Calculate layout: equal height for all visible panels
    let panel_count = 1 + has_todos as usize + has_files as usize;
    let constraints: Vec<Constraint> = (0..panel_count)
        .map(|_| Constraint::Ratio(1, panel_count as u32))
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut chunk_idx = 0;

    // Sessions list (always shown)
    let sessions_focused = app.focus == Focus::Sessions;
    sessions::draw_session_list(f, app, chunks[chunk_idx], sessions_focused);
    chunk_idx += 1;

    // Files panel (middle)
    if has_files {
        let files_focused = app.focus == Focus::Files;
        draw_files_panel(f, app, chunks[chunk_idx], files_focused);
        chunk_idx += 1;
    }

    // Todos panel (bottom)
    if has_todos {
        let todos_focused = app.focus == Focus::Todos;
        draw_todos_panel(f, app, &session_todos_display, chunks[chunk_idx], todos_focused);
    }
}

fn draw_todos_panel(f: &mut Frame, app: &mut App, todos: &[(String, String)], area: Rect, is_focused: bool) {
    let title = format!("Todos ({})", todos.len());
    let block = styled_block(&title, is_focused);

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Build lines for ALL todos (scroll to see overflow)
    let lines: Vec<Line> = todos
        .iter()
        .map(|(content, status)| {
            let (icon, style) = match status.as_str() {
                "in_progress" => ("■", Style::default().fg(Color::Rgb(255, 180, 180))),
                "completed" => ("✓", Style::default().fg(MUTED)),
                _ => ("□", Style::default().fg(Color::Gray)),
            };

            Line::from(vec![
                Span::styled(icon, style),
                Span::raw(" "),
                Span::styled(
                    truncate(content, inner.width.saturating_sub(3) as usize),
                    style,
                ),
            ])
        })
        .collect();

    // Calculate scroll
    let total_lines = lines.len() as u16;
    let visible_lines = inner.height;
    app.todos_scroll_max = total_lines.saturating_sub(visible_lines);

    let scroll_offset = app.todos_scroll.min(app.todos_scroll_max);
    let visible: Vec<Line> = lines.into_iter().skip(scroll_offset as usize).take(visible_lines as usize).collect();

    let paragraph = Paragraph::new(visible);
    f.render_widget(paragraph, inner);
}

fn draw_files_panel(f: &mut Frame, app: &mut App, area: Rect, is_focused: bool) {
    let filtered = app.filtered_files();
    let total = app.current_file_changes.len();
    let mode_indicator = if app.file_tree_mode { "tree" } else { "flat" };
    let title = if app.file_filter.is_empty() {
        format!("Files ({}) [{}]", total, mode_indicator)
    } else {
        format!("Files ({}/{}) [{}] [{}]", filtered.len(), total, app.file_filter, mode_indicator)
    };
    let block = styled_block(&title, is_focused);

    // If filter is active, show input
    if app.file_filter_active {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(area);

        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Filter (Enter to apply, Esc to cancel) ")
            .title_style(Style::default().fg(Color::Yellow).bold());

        let input = Paragraph::new(app.file_filter.as_str())
            .block(input_block)
            .style(Style::default().fg(Color::White));
        f.render_widget(input, chunks[0]);

        // Position cursor
        let cursor_x = chunks[0].x + 1 + app.file_filter.chars().count() as u16;
        let cursor_y = chunks[0].y + 1;
        if cursor_x < chunks[0].x + chunks[0].width - 1 {
            f.set_cursor_position(ratatui::layout::Position::new(cursor_x, cursor_y));
        }

        draw_files_list(f, app, &filtered, chunks[1], is_focused);
        return;
    }

    let inner = block.inner(area);
    f.render_widget(block, area);
    draw_files_list_inner(f, app, &filtered, inner, is_focused);
}

fn draw_files_list(f: &mut Frame, app: &App, files: &[&crate::data::FileChange], area: Rect, is_focused: bool) {
    let block = styled_block("", is_focused);
    let inner = block.inner(area);
    f.render_widget(block, area);
    draw_files_list_inner(f, app, files, inner, is_focused);
}

fn draw_files_list_inner(f: &mut Frame, app: &App, files: &[&crate::data::FileChange], inner: Rect, is_focused: bool) {
    use crate::data::FileStatus;

    let mut lines: Vec<Line> = Vec::new();
    let mut selected_line: usize = 0;

    if app.file_tree_mode {
        // Tree view: group files by directory
        let mut sorted_files: Vec<(usize, &crate::data::FileChange)> = files.iter().enumerate().map(|(i, f)| (i, *f)).collect();
        sorted_files.sort_by(|a, b| a.1.path.cmp(&b.1.path));

        let mut last_dir: Option<String> = None;

        for (idx, file) in &sorted_files {
            let dir = std::path::Path::new(&file.path)
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or("")
                .to_string();

            // Show directory header if changed
            if last_dir.as_ref() != Some(&dir) && !dir.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(format!("{}/ ", dir), Style::default().fg(Color::Blue).bold()),
                ]));
                last_dir = Some(dir.clone());
            }

            let is_selected = is_focused && *idx == app.selected_file_idx;
            if is_selected {
                selected_line = lines.len();
            }
            let (icon, status_color) = match file.status {
                FileStatus::Modified => ("M", Color::Yellow),
                FileStatus::Added => ("A", Color::Green),
                FileStatus::Deleted => ("D", Color::Red),
                FileStatus::Renamed => ("R", Color::Magenta),
                FileStatus::Untracked => ("?", Color::Gray),
            };

            let max_name = inner.width.saturating_sub(20) as usize;
            let indent = if last_dir.is_some() { "  " } else { "" };

            let mut spans = vec![
                Span::styled(icon, Style::default().fg(status_color).bold()),
                Span::raw(" "),
                Span::styled(indent, Style::default()),
                Span::styled(
                    truncate(&file.filename, max_name),
                    if is_selected { Style::default().fg(Color::White).bold() } else { Style::default().fg(Color::Gray) },
                ),
            ];

            if file.additions > 0 {
                spans.push(Span::styled(format!(" +{}", file.additions), Style::default().fg(Color::Rgb(100, 180, 100))));
            }
            if file.deletions > 0 {
                spans.push(Span::styled(format!(" -{}", file.deletions), Style::default().fg(Color::Rgb(180, 100, 100))));
            }

            let line = Line::from(spans);
            if is_selected {
                lines.push(line.style(Style::default().bg(SELECTED_BG)));
            } else {
                lines.push(line);
            }
        }
    } else {
        // Flat view: simple list of filenames
        for (idx, file) in files.iter().enumerate() {
            let is_selected = is_focused && idx == app.selected_file_idx;
            if is_selected {
                selected_line = lines.len();
            }
            let (icon, status_color) = match file.status {
                FileStatus::Modified => ("M", Color::Yellow),
                FileStatus::Added => ("A", Color::Green),
                FileStatus::Deleted => ("D", Color::Red),
                FileStatus::Renamed => ("R", Color::Magenta),
                FileStatus::Untracked => ("?", Color::Gray),
            };

            let max_name = inner.width.saturating_sub(16) as usize;

            let mut spans = vec![
                Span::styled(icon, Style::default().fg(status_color).bold()),
                Span::raw(" "),
                Span::styled(
                    truncate(&file.filename, max_name),
                    if is_selected { Style::default().fg(Color::White).bold() } else { Style::default().fg(Color::Gray) },
                ),
            ];

            if file.additions > 0 {
                spans.push(Span::styled(format!(" +{}", file.additions), Style::default().fg(Color::Rgb(100, 180, 100))));
            }
            if file.deletions > 0 {
                spans.push(Span::styled(format!(" -{}", file.deletions), Style::default().fg(Color::Rgb(180, 100, 100))));
            }

            let line = Line::from(spans);
            if is_selected {
                lines.push(line.style(Style::default().bg(SELECTED_BG)));
            } else {
                lines.push(line);
            }
        }
    }

    // Calculate scroll to keep selected item visible
    let visible_height = inner.height as usize;
    let scroll = if selected_line >= visible_height {
        (selected_line - visible_height + 1) as u16
    } else {
        0
    };

    let paragraph = Paragraph::new(lines).scroll((scroll, 0));
    f.render_widget(paragraph, inner);
}

fn draw_help_bar(f: &mut Frame, app: &App, area: Rect) {
    if let Some(ref msg) = app.status_message {
        let style = if app.status_is_error {
            Style::default().fg(ERROR).bold()
        } else {
            Style::default().fg(SUCCESS).bold()
        };
        let status = Paragraph::new(msg.as_str())
            .style(style)
            .alignment(Alignment::Center);
        f.render_widget(status, area);
        return;
    }

    let help_text = match (app.focus, app.fullscreen) {
        (_, true) => "j/k: scroll │ h/l: hunks │ ^u/d: page │ ^f/Esc: exit │ g/G: top/bottom │ q: quit",
        (Focus::Sessions, _) => "j/k: nav │ l: files │ Enter: view │ r: rename │ o: open │ n: new │ ?: help │ q: quit",
        (Focus::Files, _) => "j/k: select │ f: filter │ t: tree/flat │ Enter: view │ Esc: back │ q: quit",
        (Focus::Todos, _) => "j/k: scroll │ h: files │ Enter: view │ Esc: back │ ?: help │ q: quit",
        (Focus::Detail, _) if app.diff_mode => "j/k: scroll │ h/l: hunks │ ^u/d: page │ Esc: back │ q: quit",
        (Focus::Detail, _) => "j/k: scroll │ ^u/d: page │ Esc: back │ g/G: top/bottom │ q: quit",
    };

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    f.render_widget(help, area);
}

pub fn styled_block(title: &str, is_active: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if is_active { BORDER_ACTIVE } else { BORDER_COLOR }))
        .title(format!(" {} ", title))
        .title_style(Style::default().fg(if is_active { Color::Green } else { Color::White }).bold())
}

pub fn styled_block_themed(title: &str, is_active: bool, theme: &Theme) -> Block<'static> {
    let border = if is_active { border_active(theme) } else { border_color(theme) };
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .title(format!(" {} ", title))
        .title_style(Style::default().fg(if is_active { border_active(theme) } else { Color::White }).bold())
}

pub fn relative_time(dt: &Option<chrono::DateTime<chrono::Utc>>) -> String {
    match dt {
        Some(dt) => {
            let now = chrono::Utc::now();
            let duration = now.signed_duration_since(*dt);

            if duration.num_seconds() < 60 {
                "<1m ago".to_string()
            } else if duration.num_minutes() < 60 {
                format!("{}m ago", duration.num_minutes())
            } else if duration.num_hours() < 24 {
                format!("{}h ago", duration.num_hours())
            } else {
                format!("{}d ago", duration.num_days())
            }
        }
        None => "—".to_string(),
    }
}

pub fn truncate(s: &str, max_len: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_len {
        s.to_string()
    } else if max_len > 3 {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    } else {
        s.chars().take(max_len).collect()
    }
}

#[allow(dead_code)]
pub fn status_style(status: &str) -> Style {
    match status.to_lowercase().as_str() {
        "running" | "active" | "in_progress" => Style::default().fg(SUCCESS),
        "pending" | "waiting" => Style::default().fg(WARNING),
        "completed" | "done" => Style::default().fg(INFO),
        "failed" | "error" => Style::default().fg(ERROR),
        _ => Style::default().fg(MUTED),
    }
}

fn draw_help_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_width = 36.min(area.width.saturating_sub(4));
    let popup_height = 23.min(area.height.saturating_sub(4));
    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    f.render_widget(Clear, popup_area);

    let help_content = vec![
        Line::from(Span::styled("─ Navigation", Style::default().fg(INFO).bold())),
        Line::from(vec![
            Span::styled("  j/k ", Style::default().fg(Color::Yellow)),
            Span::styled("Move down/up", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("  h/l ", Style::default().fg(Color::Yellow)),
            Span::styled("Switch panels / Jump hunks", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("  g/G ", Style::default().fg(Color::Yellow)),
            Span::styled("Top/bottom", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled(" ^u/d ", Style::default().fg(Color::Yellow)),
            Span::styled("Page up/down", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("  Tab ", Style::default().fg(Color::Yellow)),
            Span::styled("Toggle focus", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("Enter ", Style::default().fg(Color::Yellow)),
            Span::styled("Fullscreen", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("  Esc ", Style::default().fg(Color::Yellow)),
            Span::styled("Back", Style::default().fg(Color::Gray)),
        ]),
        Line::from(""),
        Line::from(Span::styled("─ Sessions", Style::default().fg(INFO).bold())),
        Line::from(vec![
            Span::styled("    o ", Style::default().fg(Color::Yellow)),
            Span::styled("Open in terminal", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("    n ", Style::default().fg(Color::Yellow)),
            Span::styled("New session", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("    r ", Style::default().fg(Color::Yellow)),
            Span::styled("Rename", Style::default().fg(Color::Gray)),
        ]),
        Line::from(""),
        Line::from(Span::styled("─ Files", Style::default().fg(INFO).bold())),
        Line::from(vec![
            Span::styled("    f ", Style::default().fg(Color::Yellow)),
            Span::styled("Filter", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("    t ", Style::default().fg(Color::Yellow)),
            Span::styled("Tree/flat", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("    y ", Style::default().fg(Color::Yellow)),
            Span::styled("Yank path", Style::default().fg(Color::Gray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("    ? ", Style::default().fg(Color::Yellow)),
            Span::styled("Help", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled("    q ", Style::default().fg(Color::Yellow)),
            Span::styled("Quit", Style::default().fg(Color::Gray)),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color(&app.config.theme)))
        .title(" Help ")
        .title_style(Style::default().fg(Color::White).bold());

    let help = Paragraph::new(help_content)
        .block(block)
        .alignment(Alignment::Left);

    f.render_widget(help, popup_area);
}
