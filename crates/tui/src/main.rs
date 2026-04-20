mod app;
mod convert;
mod ui;

use anyhow::{Context, Result};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{env, io, path::PathBuf};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let project_path = args.get(1).cloned().map(PathBuf::from);

    if project_path.is_none() {
        eprintln!("Usage: chikn <path-to-project.chikn>");
        eprintln!();
        eprintln!("A terminal UI for ChickenScratch writing projects.");
        std::process::exit(1);
    }

    let mut app = app::App::new(project_path.unwrap()).context("Failed to load project")?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = app.run(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
