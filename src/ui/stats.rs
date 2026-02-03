use crate::app::App;
use ratatui::{
    prelude::*,
    widgets::{Axis, Chart, Dataset, Paragraph, Row, Table, Cell},
    symbols,
};
use super::{styled_block, MUTED, INFO, SUCCESS};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    // Layout: chart on top, table on bottom
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    draw_activity_chart(f, app, chunks[0]);
    draw_stats_table(f, app, chunks[1]);
}

fn draw_activity_chart(f: &mut Frame, app: &App, area: Rect) {
    let block = styled_block("Activity Over Time", true);

    if app.daily_stats.len() < 2 {
        let empty = Paragraph::new("Not enough data for chart")
            .block(block)
            .style(Style::default().fg(MUTED))
            .alignment(Alignment::Center);
        f.render_widget(empty, area);
        return;
    }

    // Prepare data points (last 14 days)
    let data: Vec<(f64, f64)> = app.daily_stats
        .iter()
        .rev()
        .take(14)
        .enumerate()
        .map(|(i, s)| (i as f64, s.message_count as f64))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    let max_y = data.iter().map(|(_, y)| *y).fold(0.0_f64, f64::max);

    let datasets = vec![
        Dataset::default()
            .name("Messages")
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Green))
            .graph_type(ratatui::widgets::GraphType::Line)
            .data(&data),
    ];

    let chart = Chart::new(datasets)
        .block(block)
        .x_axis(
            Axis::default()
                .title("Days")
                .style(Style::default().fg(MUTED))
                .bounds([0.0, 14.0])
                .labels(vec![
                    Span::raw("14d"),
                    Span::raw("7d"),
                    Span::raw("today"),
                ]),
        )
        .y_axis(
            Axis::default()
                .title("Messages")
                .style(Style::default().fg(MUTED))
                .bounds([0.0, max_y * 1.1])
                .labels(vec![
                    Span::raw("0"),
                    Span::raw(format!("{:.0}", max_y / 2.0)),
                    Span::raw(format!("{:.0}", max_y)),
                ]),
        );

    f.render_widget(chart, area);
}

fn draw_stats_table(f: &mut Frame, app: &App, area: Rect) {
    let block = styled_block("Daily Statistics", false);

    if app.daily_stats.is_empty() {
        let empty = Paragraph::new("No statistics available")
            .block(block)
            .style(Style::default().fg(MUTED))
            .alignment(Alignment::Center);
        f.render_widget(empty, area);
        return;
    }

    let header_cells = ["Date", "Messages", "Sessions", "Tool Calls"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(INFO).bold()));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows: Vec<Row> = app.daily_stats
        .iter()
        .rev()
        .take(10)
        .map(|stat| {
            let cells = vec![
                Cell::from(stat.date.clone()).style(Style::default().fg(Color::White)),
                Cell::from(stat.message_count.to_string()).style(Style::default().fg(SUCCESS)),
                Cell::from(stat.session_count.to_string()).style(Style::default().fg(Color::Gray)),
                Cell::from(stat.tool_call_count.to_string()).style(Style::default().fg(Color::Gray)),
            ];
            Row::new(cells).height(1)
        })
        .collect();

    let widths = [
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(Style::default().bg(Color::DarkGray));

    f.render_widget(table, area);
}
