mod app;
mod data;
mod events;
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
    let args = Args::parse();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let mut app = App::new(args.refresh);
    app.load_data().await?;

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
