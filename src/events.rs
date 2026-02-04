use crate::app::{App, Focus};
use crate::ui;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use std::time::Duration;

/// Convert a key event to bytes for the terminal
fn key_to_bytes(key: KeyEvent) -> Vec<u8> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    match key.code {
        KeyCode::Char(c) if ctrl => {
            let ctrl_code = (c.to_ascii_lowercase() as u8)
                .wrapping_sub(b'a')
                .wrapping_add(1);
            vec![ctrl_code]
        }
        KeyCode::Char(c) => c.to_string().into_bytes(),
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => vec![127],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::Esc => vec![27],
        KeyCode::Up => vec![27, b'[', b'A'],
        KeyCode::Down => vec![27, b'[', b'B'],
        KeyCode::Right => vec![27, b'[', b'C'],
        KeyCode::Left => vec![27, b'[', b'D'],
        KeyCode::Home => vec![27, b'[', b'H'],
        KeyCode::End => vec![27, b'[', b'F'],
        KeyCode::PageUp => vec![27, b'[', b'5', b'~'],
        KeyCode::PageDown => vec![27, b'[', b'6', b'~'],
        KeyCode::Delete => vec![27, b'[', b'3', b'~'],
        _ => vec![],
    }
}

pub async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    let mut last_selected_session: Option<usize> = None;
    let mut last_refresh = std::time::Instant::now();

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        // Auto-refresh session data every second
        if last_refresh.elapsed() >= Duration::from_secs(1) {
            let _ = app.load_data().await;
            last_refresh = std::time::Instant::now();
        }

        // Check if session selection changed, load messages
        let current_selection = app.session_list_state.selected();
        if current_selection != last_selected_session {
            last_selected_session = current_selection;
            let _ = app.load_session_messages().await;
        }

        // Poll for events with timeout
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if handle_key(app, key).await? {
                    return Ok(());
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

async fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    // Terminal mode - forward keys to embedded terminal
    if app.terminal_mode {
        let exit_keys = matches!(
            (key.code, key.modifiers.contains(KeyModifiers::CONTROL)),
            (KeyCode::Char('\\'), true) | (KeyCode::Char(']'), true) | (KeyCode::Char('q'), true)
        );

        if exit_keys {
            app.close_embedded_terminal();
            app.set_status("Exited terminal mode");
            return Ok(false);
        }

        let data = key_to_bytes(key);
        if !data.is_empty() {
            let _ = app.send_to_terminal(&data);
        }
        return Ok(false);
    }

    // Help popup handling
    if app.show_help {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                app.show_help = false;
            }
            _ => {}
        }
        return Ok(false);
    }

    // Rename input mode
    if app.renaming {
        match key.code {
            KeyCode::Esc => app.cancel_rename(),
            KeyCode::Enter => app.confirm_rename(),
            KeyCode::Backspace => app.rename_backspace(),
            KeyCode::Char(c) => app.rename_input(c),
            _ => {}
        }
        return Ok(false);
    }

    // File filter input mode
    if app.file_filter_active {
        match key.code {
            KeyCode::Esc => app.cancel_file_filter(),
            KeyCode::Backspace => app.file_filter_backspace(),
            KeyCode::Enter => {
                // Just close filter mode but keep the filter
                app.file_filter_active = false;
            }
            KeyCode::Char(c) => app.file_filter_input(c),
            _ => {}
        }
        return Ok(false);
    }

    // Clear status on any key press
    app.clear_status();

    // Normal mode
    match key.code {
        // Ctrl+Q = fully exit detail view back to sidebar (must be before regular 'q')
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.focus == Focus::Detail || app.fullscreen {
                app.fullscreen = false;
                if app.diff_mode {
                    app.diff_mode = false;
                    app.focus = Focus::Files;
                } else {
                    app.focus = Focus::Sessions;
                }
            }
        }

        // Quit AND kill all processes (Shift+Q)
        KeyCode::Char('Q') => {
            let _ = app.kill_all_processes();
            app.should_quit = true;
            return Ok(true);
        }

        // Quit
        KeyCode::Char('q') => {
            app.should_quit = true;
            return Ok(true);
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
            return Ok(true);
        }

        // Tab = switch focus between left and detail
        KeyCode::Tab | KeyCode::BackTab => app.toggle_focus(),

        // h = go UP in left sidebar, or previous hunk in diff mode
        // Sidebar order: Presets -> Sessions -> Files -> Todos
        KeyCode::Char('h') => match app.focus {
            Focus::Detail if app.diff_mode => {
                app.jump_to_prev_hunk();
            }
            Focus::Todos if !app.current_file_changes.is_empty() => app.focus = Focus::Files,
            Focus::Todos => {
                app.focus = Focus::Sessions;
                app.diff_mode = false;
            }
            Focus::Files => {
                app.focus = Focus::Sessions;
                app.diff_mode = false;
            }
            Focus::Sessions => {
                // Navigate up to Presets panel
                app.focus = Focus::Presets;
            }
            _ => {}
        },

        // l = go DOWN in left sidebar, or next hunk in diff mode
        KeyCode::Char('l') => match app.focus {
            Focus::Detail if app.diff_mode => {
                app.jump_to_next_hunk();
            }
            Focus::Presets => app.focus = Focus::Sessions,
            Focus::Sessions if !app.current_file_changes.is_empty() => {
                app.focus = Focus::Files;
                app.load_file_diff().await;
            }
            Focus::Sessions if app.selected_session_todos_count() > 0 => {
                app.focus = Focus::Todos;
            }
            Focus::Files if app.selected_session_todos_count() > 0 => {
                app.focus = Focus::Todos;
            }
            _ => {}
        },

        // j/k = navigate within current panel (j=down, k=up)
        KeyCode::Char('j') | KeyCode::Down => {
            match app.focus {
                Focus::Presets => {
                    if app.selected_preset_idx + 1 < app.presets.len() {
                        app.selected_preset_idx += 1;
                    }
                }
                Focus::Sessions => app.list_next(),
                Focus::Todos => app.todos_scroll_down(),
                Focus::Files => {
                    app.files_select_next();
                    app.load_file_diff().await;
                }
                Focus::Detail if app.diff_mode => app.scroll_up(), // diff: scroll_up = view moves down
                Focus::Detail => app.scroll_down(), // chat: scroll_down = view moves down
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            match app.focus {
                Focus::Presets => {
                    if app.selected_preset_idx > 0 {
                        app.selected_preset_idx -= 1;
                    }
                }
                Focus::Sessions => app.list_prev(),
                Focus::Todos => app.todos_scroll_up(),
                Focus::Files => {
                    app.files_select_prev();
                    app.load_file_diff().await;
                }
                Focus::Detail if app.diff_mode => app.scroll_down(), // diff: scroll_down = view moves up
                Focus::Detail => app.scroll_up(), // chat: scroll_up = view moves up
            }
        }

        // Page up/down (Ctrl+U = up, Ctrl+D = down)
        KeyCode::PageUp | KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.diff_mode || app.focus == Focus::Files {
                // Diff view: scroll_down moves view up (towards top)
                for _ in 0..10 {
                    app.scroll_down();
                }
            } else {
                // Chat view: scroll_up moves view up (towards earlier messages)
                for _ in 0..10 {
                    app.scroll_up();
                }
            }
        }
        KeyCode::PageDown => {
            if app.diff_mode || app.focus == Focus::Files {
                // Diff view: scroll_up moves view down (towards bottom)
                for _ in 0..10 {
                    app.scroll_up();
                }
            } else {
                // Chat view: scroll_down moves view down (towards newer messages)
                for _ in 0..10 {
                    app.scroll_down();
                }
            }
        }

        // Ctrl+D = page down OR kill all processes
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.diff_mode || app.focus == Focus::Files {
                // Diff view: scroll_up moves view down (towards bottom)
                for _ in 0..10 {
                    app.scroll_up();
                }
            } else {
                // Chat view: scroll_down moves view down (towards newer messages)
                for _ in 0..10 {
                    app.scroll_down();
                }
            }
        }

        // Enter = fullscreen detail view (from any left panel)
        KeyCode::Enter => match app.focus {
            Focus::Presets => {
                // TODO: Launch preset instances
            }
            Focus::Files => {
                app.focus = Focus::Detail;
                app.diff_mode = true;
                app.fullscreen = true;
            }
            Focus::Sessions | Focus::Todos => {
                app.focus = Focus::Detail;
                app.diff_mode = false;
                app.fullscreen = true;
            }
            Focus::Detail => {}
        },

        // Esc = exit fullscreen first, then go back
        KeyCode::Esc => {
            if app.fullscreen {
                app.fullscreen = false;
            } else {
                match app.focus {
                    Focus::Detail if app.diff_mode => {
                        app.focus = Focus::Files;
                        app.diff_mode = false;
                    }
                    Focus::Detail | Focus::Todos | Focus::Files => {
                        app.focus = Focus::Sessions;
                        app.diff_mode = false;
                    }
                    Focus::Presets => {
                        app.focus = Focus::Sessions;
                    }
                    Focus::Sessions => {}
                }
            }
        }

        // Top/bottom
        KeyCode::Char('g') => match app.focus {
            Focus::Presets => app.selected_preset_idx = 0,
            Focus::Sessions => app.session_list_state.select(Some(0)),
            Focus::Todos => app.todos_scroll = 0,
            Focus::Files => app.files_scroll = 0,
            Focus::Detail => app.scroll_top(),
        },
        KeyCode::Char('G') => match app.focus {
            Focus::Presets => {
                let len = app.presets.len();
                if len > 0 {
                    app.selected_preset_idx = len - 1;
                }
            }
            Focus::Sessions => {
                let len = app.sessions.len();
                if len > 0 {
                    app.session_list_state.select(Some(len - 1));
                }
            }
            Focus::Todos => app.todos_scroll = app.todos_scroll_max,
            Focus::Files => app.files_scroll = app.files_scroll_max,
            Focus::Detail => app.scroll_bottom(),
        },

        // Open session in embedded terminal (only from Sessions panel)
        KeyCode::Char('o') => {
            if app.focus == Focus::Files || app.diff_mode {
                // Disabled in diff view for now
            } else if app.selected_session().is_some() {
                match app.open_embedded_terminal(80, 24) {
                    Ok(_) => app.set_status("Opening Claude... (Ctrl+q to exit)"),
                    Err(e) => app.set_error(&format!("Failed: {}", e)),
                }
            } else {
                app.set_error("No session selected");
            }
        }

        // New session OR spawn preset
        KeyCode::Char('n') => {
            if app.focus == Focus::Presets {
                // Spawn instances from selected preset
                if let Err(e) = app.spawn_preset() {
                    app.set_error(&format!("Failed to spawn preset: {e}"));
                }
            } else {
                // Existing new session logic
                match app.open_new_embedded_terminal(80, 24) {
                    Ok(_) => app.set_status("Starting new Claude... (Ctrl+q to exit)"),
                    Err(e) => app.set_error(&format!("Failed: {}", e)),
                }
            }
        }

        // Ctrl+F = exit fullscreen (Enter to enter)
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.fullscreen {
                app.fullscreen = false;
            }
        }

        // Help
        KeyCode::Char('?') => app.toggle_help(),

        // Rename session
        KeyCode::Char('r') => {
            if app.focus == Focus::Sessions {
                app.start_rename();
            }
        }

        // File filter
        KeyCode::Char('f') => {
            if app.focus == Focus::Files {
                app.start_file_filter();
            }
        }

        // Toggle file tree view
        KeyCode::Char('t') => {
            if app.focus == Focus::Files {
                app.toggle_file_tree_mode();
            }
        }

        // Yank (copy) file path to clipboard
        KeyCode::Char('y') => {
            if app.focus == Focus::Files {
                if app.yank_file_path() {
                    if let Some(path) = app.selected_file_path() {
                        app.set_status(&format!("Copied: {}", path));
                    }
                } else {
                    app.set_error("Failed to copy to clipboard");
                }
            }
        }

        // Edit file in $EDITOR (default: nvim) - works from Files panel or diff view
        KeyCode::Char('e') => {
            let can_edit = (app.focus == Focus::Files
                || (app.focus == Focus::Detail && app.diff_mode))
                && !app.current_file_changes.is_empty();
            if can_edit {
                // Get terminal size from crossterm
                let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
                match app.open_editor(cols, rows) {
                    Ok(_) => app.set_status("Opening editor... (Ctrl+q to exit)"),
                    Err(e) => app.set_error(&format!("Failed: {e}")),
                }
            }
        }

        // Kill process (d)
        KeyCode::Char('d') => {
            if app.focus == Focus::Sessions {
                // TODO: Add confirmation dialog
                // For now, just show message that kill is not yet implemented for sessions
                app.set_status("Kill process: select from process list (coming in Phase 5)");
            }
        }

        // Kill all processes (D)
        KeyCode::Char('D') => {
            if let Err(e) = app.kill_all_processes() {
                app.set_error(&format!("Failed: {e}"));
            }
        }

        _ => {}
    }

    Ok(false)
}
