use crate::app::{App, Focus};
use ratatui::{
    prelude::*,
    widgets::{Block, List, ListItem, Paragraph, Wrap},
};
use super::{styled_block, truncate, status_style, MUTED, SELECTED_BG, INFO};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    // Layout: list on left (40%), details on right (60%)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let is_list_focused = app.focus == Focus::List;
    draw_task_list(f, app, chunks[0], is_list_focused);
    draw_task_details(f, app, chunks[1], !is_list_focused);
}

fn draw_task_list(f: &mut Frame, app: &mut App, area: Rect, is_focused: bool) {
    // Group counts
    let pending = app.tasks.iter().filter(|t| t.status == "pending").count();
    let in_progress = app.tasks.iter().filter(|t| t.status == "in_progress").count();
    let completed = app.tasks.iter().filter(|t| t.status == "completed").count();

    let title = format!(
        "Tasks ({} pending, {} active, {} done)",
        pending, in_progress, completed
    );
    let block = styled_block(&title, is_focused);

    if app.tasks.is_empty() {
        let empty = Paragraph::new("No tasks found")
            .block(block)
            .style(Style::default().fg(MUTED))
            .alignment(Alignment::Center);
        f.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app
        .tasks
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let is_selected = app.task_list_state.selected() == Some(i);

            // Status icon (checkbox style like lazygit)
            let status_icon = match task.status.as_str() {
                "completed" | "done" => "✓",
                "in_progress" => "◐",
                "pending" => "○",
                _ => "?",
            };

            let content = Line::from(vec![
                Span::styled(status_icon, status_style(&task.status)),
                Span::raw(" "),
                Span::styled(
                    truncate(&task.subject, 35),
                    Style::default().fg(if is_selected { Color::White } else { Color::Gray }),
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

    f.render_stateful_widget(list, area, &mut app.task_list_state);
}

fn draw_task_details(f: &mut Frame, app: &App, area: Rect, is_focused: bool) {
    let block = styled_block("Task Details", is_focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let task = match app.task_list_state.selected() {
        Some(i) => app.tasks.get(i),
        None => None,
    };

    let content = match task {
        Some(t) => {
            let mut lines = vec![
                Line::from(vec![
                    Span::styled("Subject: ", Style::default().fg(MUTED)),
                ]),
                Line::from(vec![
                    Span::styled(&t.subject, Style::default().fg(INFO).bold()),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Status:  ", Style::default().fg(MUTED)),
                    Span::styled(&t.status, status_style(&t.status)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("ID:      ", Style::default().fg(MUTED)),
                    Span::styled(truncate(&t.id, 20), Style::default().fg(Color::Gray)),
                ]),
            ];

            if let Some(ref agent_id) = t.agent_id {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Agent:   ", Style::default().fg(MUTED)),
                    Span::styled(truncate(agent_id, 16), Style::default().fg(Color::Gray)),
                ]));
            }

            if !t.description.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Description:", Style::default().fg(MUTED)),
                ]));
                lines.push(Line::from(""));
                // Word wrap description
                for line in t.description.lines().take(10) {
                    lines.push(Line::from(Span::styled(line, Style::default().fg(Color::White))));
                }
            }

            lines
        }
        None => {
            vec![Line::from(Span::styled(
                "Select a task to view details",
                Style::default().fg(MUTED),
            ))]
        }
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().padding(ratatui::widgets::Padding::uniform(1)))
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, inner);
}
