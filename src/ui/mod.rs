mod dashboard;
mod sessions;
mod agents;
mod tasks;
mod stats;

use crate::app::{App, Tab};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph, Tabs},
};

// Lazygit-style colors
pub const BORDER_COLOR: Color = Color::Blue;
pub const BORDER_ACTIVE: Color = Color::Green;
pub const HEADER_BG: Color = Color::Blue;
pub const SELECTED_BG: Color = Color::DarkGray;
pub const SELECTED_FG: Color = Color::White;
pub const MUTED: Color = Color::DarkGray;
pub const SUCCESS: Color = Color::Green;
pub const WARNING: Color = Color::Yellow;
pub const ERROR: Color = Color::Red;
pub const INFO: Color = Color::Cyan;

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Layout: header (tabs) + main content + footer (help)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tabs
            Constraint::Min(0),    // Content
            Constraint::Length(1), // Help bar
        ])
        .split(size);

    // Draw tabs (lazygit style)
    draw_tabs(f, app, chunks[0]);

    // Draw content based on current tab
    match app.current_tab {
        Tab::Dashboard => dashboard::draw(f, app, chunks[1]),
        Tab::Sessions => sessions::draw(f, app, chunks[1]),
        Tab::Agents => agents::draw(f, app, chunks[1]),
        Tab::Tasks => tasks::draw(f, app, chunks[1]),
        Tab::Stats => stats::draw(f, app, chunks[1]),
    }

    // Draw help bar (lazygit style)
    draw_help_bar(f, app, chunks[2]);

    // Draw help popup if active
    if app.show_help {
        draw_help_popup(f, size);
    }
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Tab::all()
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let style = if *t == app.current_tab {
                Style::default().fg(Color::Green).bold()
            } else {
                Style::default().fg(Color::White)
            };
            Line::from(format!(" {} {} ", i + 1, t.title())).style(style)
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER_COLOR))
                .title(" Claude TUI ")
                .title_style(Style::default().fg(Color::Cyan).bold()),
        )
        .highlight_style(Style::default().fg(Color::Green).bold())
        .select(Tab::all().iter().position(|t| *t == app.current_tab).unwrap_or(0));

    f.render_widget(tabs, area);
}

fn draw_help_bar(f: &mut Frame, app: &App, area: Rect) {
    // Show status message if available, otherwise show help
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

    let help_text = match app.current_tab {
        Tab::Dashboard => "q: quit │ Tab: switch panel │ 1-5: tabs │ R: refresh │ ?: help",
        Tab::Sessions => "j/k: scroll │ o: open │ n: new │ ?: help",
        Tab::Agents => "j/k: nav │ Enter: expand │ Tab: panel │ R: refresh │ ?: help",
        Tab::Tasks => "j/k: nav │ Enter: details │ Tab: panel │ R: refresh │ ?: help",
        Tab::Stats => "Tab: switch panel │ R: refresh │ q: quit │ ?: help",
    };

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    f.render_widget(help, area);
}

// Helper to create a styled block (lazygit style)
pub fn styled_block(title: &str, is_active: bool) -> Block {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if is_active { BORDER_ACTIVE } else { BORDER_COLOR }))
        .title(format!(" {} ", title))
        .title_style(Style::default().fg(if is_active { Color::Green } else { Color::White }).bold())
}

// Format relative time (lazygit style)
pub fn relative_time(dt: &Option<chrono::DateTime<chrono::Utc>>) -> String {
    match dt {
        Some(dt) => {
            let now = chrono::Utc::now();
            let duration = now.signed_duration_since(*dt);

            if duration.num_seconds() < 60 {
                format!("{}s ago", duration.num_seconds())
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

// Truncate string with ellipsis (char-safe)
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

// Status badge with color
pub fn status_style(status: &str) -> Style {
    match status.to_lowercase().as_str() {
        "running" | "active" | "in_progress" => Style::default().fg(SUCCESS),
        "pending" | "waiting" => Style::default().fg(WARNING),
        "completed" | "done" => Style::default().fg(INFO),
        "failed" | "error" => Style::default().fg(ERROR),
        _ => Style::default().fg(MUTED),
    }
}

// Help popup (lazygit style)
fn draw_help_popup(f: &mut Frame, area: Rect) {
    // Center the popup
    let popup_width = 60.min(area.width.saturating_sub(4));
    let popup_height = 24.min(area.height.saturating_sub(4));
    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    // Clear the area behind the popup
    f.render_widget(Clear, popup_area);

    let help_content = vec![
        Line::from(Span::styled("NAVIGATION", Style::default().fg(INFO).bold())),
        Line::from(""),
        Line::from(vec![
            Span::styled("  1-5       ", Style::default().fg(Color::Yellow)),
            Span::raw("Jump to tab (Dashboard, Sessions, Agents, Tasks, Stats)"),
        ]),
        Line::from(vec![
            Span::styled("  Tab       ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle focus between list and detail panels"),
        ]),
        Line::from(vec![
            Span::styled("  h/l       ", Style::default().fg(Color::Yellow)),
            Span::raw("Switch panels or tabs"),
        ]),
        Line::from(vec![
            Span::styled("  j/k ↑/↓   ", Style::default().fg(Color::Yellow)),
            Span::raw("Navigate lists / Scroll chat"),
        ]),
        Line::from(vec![
            Span::styled("  g/G       ", Style::default().fg(Color::Yellow)),
            Span::raw("Go to top/bottom"),
        ]),
        Line::from(""),
        Line::from(Span::styled("SESSIONS", Style::default().fg(INFO).bold())),
        Line::from(""),
        Line::from(vec![
            Span::styled("  k/↑       ", Style::default().fg(Color::Yellow)),
            Span::raw("Scroll up (older messages)"),
        ]),
        Line::from(vec![
            Span::styled("  j/↓       ", Style::default().fg(Color::Yellow)),
            Span::raw("Scroll down (newer messages)"),
        ]),
        Line::from(vec![
            Span::styled("  g         ", Style::default().fg(Color::Yellow)),
            Span::raw("Go to top (oldest)"),
        ]),
        Line::from(vec![
            Span::styled("  G         ", Style::default().fg(Color::Yellow)),
            Span::raw("Go to bottom (latest)"),
        ]),
        Line::from(vec![
            Span::styled("  o         ", Style::default().fg(Color::Yellow)),
            Span::raw("Open session in embedded terminal"),
        ]),
        Line::from(vec![
            Span::styled("  n         ", Style::default().fg(Color::Yellow)),
            Span::raw("Start new Claude session"),
        ]),
        Line::from(""),
        Line::from(Span::styled("GENERAL", Style::default().fg(INFO).bold())),
        Line::from(""),
        Line::from(vec![
            Span::styled("  R         ", Style::default().fg(Color::Yellow)),
            Span::raw("Refresh data"),
        ]),
        Line::from(vec![
            Span::styled("  ?         ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle this help"),
        ]),
        Line::from(vec![
            Span::styled("  q         ", Style::default().fg(Color::Yellow)),
            Span::raw("Quit"),
        ]),
        Line::from(""),
        Line::from(Span::styled("Press ? or Esc to close", Style::default().fg(MUTED).italic())),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Keybindings ")
        .title_style(Style::default().fg(Color::Cyan).bold())
        .style(Style::default().bg(Color::Black));

    let help = Paragraph::new(help_content)
        .block(block)
        .alignment(Alignment::Left);

    f.render_widget(help, popup_area);
}
