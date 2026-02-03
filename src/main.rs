mod app;
mod config;
mod data;
mod events;
mod process;
mod terminal;
mod ui;

use anyhow::Result;
use app::App;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;

#[derive(Parser, Debug)]
#[command(name = "lazychat")]
#[command(about = "A TUI for AI coding assistants", long_about = None)]
struct Args {
    /// Watch for file changes and auto-refresh
    #[arg(short, long, default_value_t = true)]
    watch: bool,

    /// Refresh interval in seconds
    #[arg(short, long, default_value_t = 2)]
    refresh: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _args = Args::parse();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let mut app = App::new();
    app.load_data().await?;

    // Load presets and process registry (Phase 1 & 2)
    let _ = app.load_presets();
    let _ = app.load_process_registry();

    let result = events::run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {err:?}");
    }

    Ok(())
}
