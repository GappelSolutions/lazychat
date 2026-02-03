use crate::app::{App, InputMode, Focus, Tab};
use crate::data::Session;
use crate::ui;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use std::process::Command;
use std::time::Duration;

/// Convert a key event to bytes for the terminal
fn key_to_bytes(key: KeyEvent) -> Vec<u8> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    match key.code {
        KeyCode::Char(c) if ctrl => {
            // Ctrl+A = 1, Ctrl+B = 2, etc.
            let ctrl_code = (c.to_ascii_lowercase() as u8).wrapping_sub(b'a').wrapping_add(1);
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

/// Check if running inside Zellij
fn is_zellij() -> bool {
    std::env::var("ZELLIJ").is_ok()
}

/// Get project directory from session
fn get_project_dir(session: &Session) -> String {
    if session.project.starts_with('/') {
        session.project.clone()
    } else {
        format!("/{}", session.project.replace('-', "/"))
    }
}

/// Spawn Claude in a Zellij pane or fallback to terminal
fn spawn_claude_session(session: &Session) -> Result<()> {
    let project_dir = get_project_dir(session);

    if is_zellij() {
        // Use floating pane with --close-on-exit
        // The --floating flag reuses the floating layer
        // If a floating pane is already visible, it replaces it
        let claude_cmd = format!(
            "cd '{}' 2>/dev/null || cd ~; claude --resume {} --dangerously-skip-permissions",
            project_dir,
            session.id
        );

        Command::new("zellij")
            .args(["run", "--floating", "--close-on-exit", "-c", &project_dir, "--", "bash", "-c", &claude_cmd])
            .spawn()?;

        return Ok(());
    }

    // Fallback to terminal
    let claude_cmd = format!(
        "cd '{}' 2>/dev/null || cd ~; claude --resume {} --dangerously-skip-permissions",
        project_dir,
        session.id
    );
    spawn_in_terminal(&claude_cmd)
}

/// Spawn a new Claude session
fn spawn_new_claude() -> Result<()> {
    if is_zellij() {
        Command::new("zellij")
            .args(["run", "--floating", "--close-on-exit", "--", "claude", "--dangerously-skip-permissions"])
            .spawn()?;
        return Ok(());
    }

    spawn_in_terminal("claude --dangerously-skip-permissions")
}

/// Helper to spawn a command in a terminal (fallback when not in Zellij)
fn spawn_in_terminal(cmd: &str) -> Result<()> {
    // Try kitty
    if Command::new("kitty")
        .args(["--", "bash", "-c", cmd])
        .spawn()
        .is_ok()
    {
        return Ok(());
    }

    // Try alacritty
    if Command::new("alacritty")
        .args(["-e", "bash", "-c", cmd])
        .spawn()
        .is_ok()
    {
        return Ok(());
    }

    // Fallback to macOS Terminal.app
    let apple_script = format!(
        r#"tell application "Terminal"
            do script "{}"
            activate
        end tell"#,
        cmd.replace('\\', "\\\\").replace('"', "\\\"")
    );

    Command::new("osascript")
        .args(["-e", &apple_script])
        .spawn()?;

    Ok(())
}

pub async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    let mut last_selected_session: Option<usize> = None;

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        // Check if session selection changed, load messages
        if app.current_tab == Tab::Sessions {
            let current_selection = app.session_list_state.selected();
            if current_selection != last_selected_session {
                last_selected_session = current_selection;
                let _ = app.load_session_messages().await;
            }
        }

        // Poll for events with timeout for refresh
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
    // Terminal mode - forward most keys to embedded terminal
    if app.terminal_mode {
        // Multiple ways to exit terminal mode:
        // - Ctrl+\ (traditional)
        // - Ctrl+] (alternative)
        // - Double Escape (user-friendly)
        let exit_keys = matches!(
            (key.code, key.modifiers.contains(KeyModifiers::CONTROL)),
            (KeyCode::Char('\\'), true) |
            (KeyCode::Char(']'), true) |
            (KeyCode::Char('q'), true)
        );

        if exit_keys {
            app.close_embedded_terminal();
            app.set_status("Exited terminal mode");
            return Ok(false);
        }

        // Forward key to terminal
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

    // Input mode handling
    if app.input_mode == InputMode::Input {
        match key.code {
            KeyCode::Esc => {
                app.exit_input_mode();
                app.clear_status();
            }
            KeyCode::Enter => {
                // Just clear and exit input mode (input feature not fully implemented)
                app.submit_input();
                app.set_status("Input submitted (use 'o' to open interactive Claude session)");
            }
            KeyCode::Backspace => {
                app.input_backspace();
            }
            KeyCode::Char(c) => {
                app.input_char(c);
            }
            _ => {}
        }
        return Ok(false);
    }

    // Clear status on any key press (in normal mode)
    app.clear_status();

    // Normal mode - global keys (lazygit style)
    match key.code {
        // Quit
        KeyCode::Char('q') => {
            app.should_quit = true;
            return Ok(true);
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
            return Ok(true);
        }

        // Tab navigation (1-5 or h/l)
        KeyCode::Char('1') => app.select_tab(0),
        KeyCode::Char('2') => app.select_tab(1),
        KeyCode::Char('3') => app.select_tab(2),
        KeyCode::Char('4') => app.select_tab(3),
        KeyCode::Char('5') => app.select_tab(4),

        // h/l = switch between tabs
        KeyCode::Char('h') => app.prev_tab(),
        KeyCode::Char('l') => app.next_tab(),
        KeyCode::Char('[') => app.prev_tab(),
        KeyCode::Char(']') => app.next_tab(),

        // Tab = switch between panes within a tab
        KeyCode::Tab => app.toggle_focus(),
        KeyCode::BackTab => app.toggle_focus(),

        // List navigation (vim style: j/k or arrows)
        KeyCode::Char('j') | KeyCode::Down => {
            if app.focus == Focus::Detail {
                app.scroll_down();
            } else {
                app.list_next();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.focus == Focus::Detail {
                app.scroll_up();
            } else {
                app.list_prev();
            }
        }

        // Page up/down for scrolling (Ctrl+d/u like vim)
        KeyCode::PageDown | KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            for _ in 0..10 {
                app.scroll_down();
            }
        }
        KeyCode::PageUp | KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            for _ in 0..10 {
                app.scroll_up();
            }
        }

        // Expand/collapse or open
        KeyCode::Enter => {
            match app.current_tab {
                Tab::Agents => app.toggle_expand(),
                Tab::Sessions => {
                    if app.focus == Focus::List {
                        app.focus = Focus::Detail;
                    }
                }
                _ => {}
            }
        }

        // Refresh data
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.load_data().await?;
        }
        KeyCode::Char('R') => {
            app.load_data().await?;
            if app.current_tab == Tab::Sessions {
                app.load_session_messages().await?;
            }
        }

        // Home/End for lists
        KeyCode::Char('g') => {
            if app.focus == Focus::Detail {
                app.scroll_top();
            } else {
                match app.current_tab {
                    Tab::Sessions => app.session_list_state.select(Some(0)),
                    Tab::Agents => app.agent_list_state.select(Some(0)),
                    Tab::Tasks => app.task_list_state.select(Some(0)),
                    _ => {}
                }
            }
        }
        KeyCode::Char('G') => {
            if app.focus == Focus::Detail {
                app.scroll_bottom();
            } else {
                match app.current_tab {
                    Tab::Sessions => {
                        let len = app.sessions.len();
                        if len > 0 {
                            app.session_list_state.select(Some(len - 1));
                        }
                    }
                    Tab::Agents => {
                        let len = app.agents.len();
                        if len > 0 {
                            app.agent_list_state.select(Some(len - 1));
                        }
                    }
                    Tab::Tasks => {
                        let len = app.tasks.len();
                        if len > 0 {
                            app.task_list_state.select(Some(len - 1));
                        }
                    }
                    _ => {}
                }
            }
        }

        // Open embedded Claude terminal
        KeyCode::Char('o') => {
            if app.current_tab == Tab::Sessions {
                if app.selected_session().is_some() {
                    // Use embedded terminal (80x24 default, will be resized on render)
                    match app.open_embedded_terminal(80, 24) {
                        Ok(_) => app.set_status("Opening Claude... (Ctrl+q to exit)"),
                        Err(e) => app.set_error(&format!("Failed: {}", e)),
                    }
                } else {
                    app.set_error("No session selected");
                }
            }
        }

        // New embedded Claude session
        KeyCode::Char('n') => {
            if app.current_tab == Tab::Sessions {
                match app.open_new_embedded_terminal(80, 24) {
                    Ok(_) => app.set_status("Starting new Claude... (Ctrl+q to exit)"),
                    Err(e) => app.set_error(&format!("Failed: {}", e)),
                }
            }
        }

        // Help
        KeyCode::Char('?') => {
            app.toggle_help();
        }

        _ => {}
    }

    Ok(false)
}
