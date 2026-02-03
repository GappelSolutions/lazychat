use crate::app::{App, Focus};
use ratatui::{
    prelude::*,
    widgets::{Block, List, ListItem, Paragraph},
};
use super::{styled_block, truncate, status_style, MUTED, SELECTED_BG, INFO, SUCCESS, WARNING};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    // Layout: tree on left (40%), details on right (60%)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let is_list_focused = app.focus == Focus::List;
    draw_agent_tree(f, app, chunks[0], is_list_focused);
    draw_agent_details(f, app, chunks[1], !is_list_focused);
}

fn draw_agent_tree(f: &mut Frame, app: &mut App, area: Rect, is_focused: bool) {
    let title = format!("Agents ({})", app.agents.len());
    let block = styled_block(&title, is_focused);

    if app.agents.is_empty() {
        let empty = Paragraph::new("No agents found")
            .block(block)
            .style(Style::default().fg(MUTED))
            .alignment(Alignment::Center);
        f.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app
        .agents
        .iter()
        .enumerate()
        .map(|(i, agent)| {
            let is_selected = app.agent_list_state.selected() == Some(i);
            let is_expanded = app.expanded_agents.contains(&agent.id);

            // Tree-style prefix (lazygit style)
            let prefix = if agent.parent_id.is_some() {
                if is_expanded { "├─▼ " } else { "├─▸ " }
            } else {
                if is_expanded { "▼ " } else { "▸ " }
            };

            // Status indicator and text
            let status_char = match agent.status.as_str() {
                "running" => "●",
                "active" => "◉",
                "idle" => "○",
                _ => "◌",
            };

            // Format status badge
            let status_badge = format!("[{}]", agent.status);

            // Todo count
            let todo_count = if !agent.todos.is_empty() {
                let done = agent.todos.iter().filter(|t| t.status == "completed").count();
                format!(" ({}/{})", done, agent.todos.len())
            } else {
                String::new()
            };

            let content = Line::from(vec![
                Span::styled(prefix, Style::default().fg(MUTED)),
                Span::styled(
                    status_char,
                    status_style(&agent.status),
                ),
                Span::raw(" "),
                Span::styled(
                    truncate(&agent.description, 18),
                    Style::default().fg(if is_selected { Color::White } else { Color::Gray }),
                ),
                Span::raw(" "),
                Span::styled(
                    status_badge,
                    status_style(&agent.status),
                ),
                Span::styled(
                    todo_count,
                    Style::default().fg(MUTED),
                ),
            ]);

            ListItem::new(content).style(
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

    f.render_stateful_widget(list, area, &mut app.agent_list_state);
}

fn draw_agent_details(f: &mut Frame, app: &App, area: Rect, is_focused: bool) {
    let agent = match app.agent_list_state.selected() {
        Some(i) => app.agents.get(i),
        None => None,
    };

    let title = match agent {
        Some(a) => format!("Agent: {} ({} todos)", truncate(&a.id, 8), a.todos.len()),
        None => "Agent Details".to_string(),
    };
    let block = styled_block(&title, is_focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    match agent {
        Some(a) => {
            // Header info
            lines.push(Line::from(vec![
                Span::styled("Status: ", Style::default().fg(MUTED)),
                Span::styled(&a.status, status_style(&a.status)),
                Span::raw("  "),
                Span::styled("Type: ", Style::default().fg(MUTED)),
                Span::styled(&a.agent_type, Style::default().fg(INFO)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Session: ", Style::default().fg(MUTED)),
                Span::styled(truncate(&a.session_id, 20), Style::default().fg(Color::Gray)),
            ]));
            lines.push(Line::from(""));

            // Todos section
            if a.todos.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("No todos for this agent", Style::default().fg(MUTED).italic()),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("━━━ TODOS ━━━", Style::default().fg(INFO).bold()),
                ]));
                lines.push(Line::from(""));

                for todo in &a.todos {
                    // Status icon
                    let (icon, style) = match todo.status.as_str() {
                        "completed" => ("✓", Style::default().fg(SUCCESS)),
                        "in_progress" => ("●", Style::default().fg(WARNING)),
                        _ => ("○", Style::default().fg(MUTED)),
                    };

                    lines.push(Line::from(vec![
                        Span::styled(icon, style),
                        Span::raw(" "),
                        Span::styled(
                            &todo.content,
                            if todo.status == "completed" {
                                Style::default().fg(MUTED)
                            } else {
                                Style::default().fg(Color::White)
                            },
                        ),
                    ]));
                }
            }

            // Children section
            if !a.children.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("━━━ SUB-AGENTS ━━━", Style::default().fg(INFO).bold()),
                ]));
                lines.push(Line::from(""));

                for child in &a.children {
                    let status_badge = format!("[{}]", child.status);
                    lines.push(Line::from(vec![
                        Span::styled("├─ ", Style::default().fg(MUTED)),
                        Span::styled(&child.description, Style::default().fg(Color::Gray)),
                        Span::raw(" "),
                        Span::styled(status_badge, status_style(&child.status)),
                    ]));
                }
            }
        }
        None => {
            lines.push(Line::from(Span::styled(
                "Select an agent to view details",
                Style::default().fg(MUTED).italic(),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Navigation:",
                Style::default().fg(INFO),
            )));
            lines.push(Line::from(Span::styled(
                "  j/k or ↑/↓  - Move through agents",
                Style::default().fg(MUTED),
            )));
            lines.push(Line::from(Span::styled(
                "  Enter       - Expand/collapse",
                Style::default().fg(MUTED),
            )));
            lines.push(Line::from(Span::styled(
                "  Tab         - Switch panels",
                Style::default().fg(MUTED),
            )));
        }
    };

    let paragraph = Paragraph::new(lines)
        .block(Block::default().padding(ratatui::widgets::Padding::uniform(1)));
    f.render_widget(paragraph, inner);
}
