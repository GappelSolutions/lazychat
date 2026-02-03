//! Preset panel rendering

use crate::app::App;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use super::{BORDER_ACTIVE, BORDER_COLOR, MUTED, SELECTED_BG};

/// Draw the presets panel
pub fn draw_presets_panel(f: &mut Frame, app: &App, area: Rect, focused: bool) {
    let border_color = if focused { BORDER_ACTIVE } else { BORDER_COLOR };

    let block = Block::default()
        .title(" Presets ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if app.presets.is_empty() {
        let empty_msg = Paragraph::new("No presets configured.\nEdit ~/.config/lazychat/presets.toml")
            .style(Style::default().fg(MUTED))
            .block(block);
        f.render_widget(empty_msg, area);
        return;
    }

    // Build list items
    let items: Vec<ListItem> = app.presets.iter().enumerate().map(|(i, preset)| {
        let instances = preset.instances;
        let shortcut = preset.shortcut.as_deref().unwrap_or("");

        let line = if shortcut.is_empty() {
            format!("{} ({})", preset.name, instances)
        } else {
            format!("{} [{}] ({})", preset.name, shortcut, instances)
        };

        let style = if i == app.selected_preset_idx && focused {
            Style::default().bg(SELECTED_BG).fg(Color::White)
        } else if i == app.selected_preset_idx {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(MUTED)
        };

        ListItem::new(line).style(style)
    }).collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(SELECTED_BG));

    let mut state = ListState::default();
    state.select(Some(app.selected_preset_idx));

    f.render_stateful_widget(list, area, &mut state);
}

/// Draw preset detail (when a preset is selected)
pub fn draw_preset_detail(f: &mut Frame, app: &App, area: Rect) {
    let Some(preset) = app.selected_preset() else {
        return;
    };

    let block = Block::default()
        .title(format!(" {} ", preset.name))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_COLOR));

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Directory: ", Style::default().fg(MUTED)),
            Span::raw(&preset.cwd),
        ]),
        Line::from(vec![
            Span::styled("Instances: ", Style::default().fg(MUTED)),
            Span::raw(preset.instances.to_string()),
        ]),
    ];

    if let Some(shortcut) = &preset.shortcut {
        lines.push(Line::from(vec![
            Span::styled("Shortcut: ", Style::default().fg(MUTED)),
            Span::raw(shortcut),
        ]));
    }

    if !preset.add_dirs.is_empty() {
        lines.push(Line::from(Span::styled("Add dirs:", Style::default().fg(MUTED))));
        for dir in &preset.add_dirs {
            lines.push(Line::from(format!("  {}", dir)));
        }
    }

    let para = Paragraph::new(lines).block(block);
    f.render_widget(para, area);
}
