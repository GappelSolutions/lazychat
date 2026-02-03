use crate::app::App;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Sparkline},
};
use super::{styled_block, BORDER_COLOR, INFO, SUCCESS, WARNING, MUTED};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    // Layout: 2 columns
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left column: stats cards
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // Sessions
            Constraint::Length(5),  // Agents
            Constraint::Length(5),  // Tasks
            Constraint::Min(0),     // Activity graph
        ])
        .split(main_chunks[0]);

    // Right column: recent activity & info
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),  // Today's stats
            Constraint::Min(0),     // Recent sessions
        ])
        .split(main_chunks[1]);

    // Sessions card
    draw_stat_card(
        f,
        left_chunks[0],
        "Sessions",
        &app.total_sessions().to_string(),
        "total sessions",
        INFO,
    );

    // Agents card
    let active_agents = app.active_agents();
    let total_agents = app.agents.len();
    draw_stat_card(
        f,
        left_chunks[1],
        "Agents",
        &format!("{}/{}", active_agents, total_agents),
        "active / total",
        if active_agents > 0 { SUCCESS } else { MUTED },
    );

    // Tasks card
    let pending = app.pending_tasks();
    let completed = app.completed_tasks();
    draw_stat_card(
        f,
        left_chunks[2],
        "Tasks",
        &format!("{}/{}", completed, app.tasks.len()),
        &format!("{} pending", pending),
        if pending > 0 { WARNING } else { SUCCESS },
    );

    // Activity sparkline
    draw_activity_graph(f, app, left_chunks[3]);

    // Today's stats
    draw_today_stats(f, app, right_chunks[0]);

    // Recent sessions
    draw_recent_sessions(f, app, right_chunks[1]);
}

fn draw_stat_card(f: &mut Frame, area: Rect, title: &str, value: &str, subtitle: &str, color: Color) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_COLOR))
        .title(format!(" {} ", title))
        .title_style(Style::default().fg(Color::White).bold());

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(1)])
        .margin(1)
        .split(inner);

    let value_widget = Paragraph::new(value)
        .style(Style::default().fg(color).bold().add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(value_widget, chunks[0]);

    let subtitle_widget = Paragraph::new(subtitle)
        .style(Style::default().fg(MUTED))
        .alignment(Alignment::Center);
    f.render_widget(subtitle_widget, chunks[1]);
}

fn draw_activity_graph(f: &mut Frame, app: &App, area: Rect) {
    let block = styled_block("Activity (messages/day)", false);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.daily_stats.is_empty() {
        let empty = Paragraph::new("No activity data")
            .style(Style::default().fg(MUTED))
            .alignment(Alignment::Center);
        f.render_widget(empty, inner);
        return;
    }

    // Get last 14 days of data
    let data: Vec<u64> = app.daily_stats
        .iter()
        .rev()
        .take(14)
        .map(|s| s.message_count)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    let sparkline = Sparkline::default()
        .data(&data)
        .style(Style::default().fg(Color::Green));

    let sparkline_area = Rect {
        x: inner.x + 1,
        y: inner.y + 1,
        width: inner.width.saturating_sub(2),
        height: inner.height.saturating_sub(2),
    };
    f.render_widget(sparkline, sparkline_area);
}

fn draw_today_stats(f: &mut Frame, app: &App, area: Rect) {
    let block = styled_block("Today", false);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let messages = app.today_messages();
    let tool_calls = app.today_tool_calls();

    let text = vec![
        Line::from(vec![
            Span::styled("Messages:    ", Style::default().fg(MUTED)),
            Span::styled(format!("{}", messages), Style::default().fg(INFO).bold()),
        ]),
        Line::from(vec![
            Span::styled("Tool Calls:  ", Style::default().fg(MUTED)),
            Span::styled(format!("{}", tool_calls), Style::default().fg(INFO).bold()),
        ]),
        Line::from(vec![
            Span::styled("Sessions:    ", Style::default().fg(MUTED)),
            Span::styled(
                format!("{}", app.daily_stats.last().map(|s| s.session_count).unwrap_or(0)),
                Style::default().fg(INFO).bold()
            ),
        ]),
    ];

    let paragraph = Paragraph::new(text).block(Block::default().padding(ratatui::widgets::Padding::horizontal(1)));
    f.render_widget(paragraph, inner);
}

fn draw_recent_sessions(f: &mut Frame, app: &App, area: Rect) {
    let block = styled_block("Recent Sessions", false);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.sessions.is_empty() {
        let empty = Paragraph::new("No sessions yet")
            .style(Style::default().fg(MUTED))
            .alignment(Alignment::Center);
        f.render_widget(empty, inner);
        return;
    }

    let items: Vec<Line> = app.sessions
        .iter()
        .take(inner.height as usize)
        .map(|s| {
            Line::from(vec![
                Span::styled(
                    super::truncate(&s.project_name, 20),
                    Style::default().fg(Color::White),
                ),
                Span::raw(" "),
                Span::styled(
                    super::relative_time(&s.last_activity),
                    Style::default().fg(MUTED),
                ),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(items)
        .block(Block::default().padding(ratatui::widgets::Padding::horizontal(1)));
    f.render_widget(paragraph, inner);
}
