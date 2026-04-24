mod app;
mod convert;
mod ui;

use anyhow::{Context, Result};
use chickenscratch_core::core::{git, project::writer};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{env, io, path::PathBuf};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    // Subcommand: chikn new <parent-dir> <name>
    if args.get(1).map(|s| s.as_str()) == Some("new") {
        let dir = args.get(2).map(PathBuf::from).unwrap_or_else(|| {
            eprintln!("Usage: chikn new <parent-dir> <name>");
            std::process::exit(1);
        });
        let name = args.get(3).cloned().unwrap_or_else(|| {
            eprintln!("Usage: chikn new <parent-dir> <name>");
            std::process::exit(1);
        });
        let project_path = dir.join(format!("{}.chikn", name));
        let mut project = writer::create_project(&project_path, &name)
            .with_context(|| format!("Failed to create project at {:?}", project_path))?;
        writer::write_project(&mut project).context("Failed to write project")?;
        let _ = git::save_revision(&project_path, &format!("Created project: {}", name));
        println!("Created: {}", project_path.display());
        return Ok(());
    }

    let project_path = args.get(1).cloned().map(PathBuf::from);

    if project_path.is_none() {
        eprintln!("Usage:");
        eprintln!("  chikn <path-to-project.chikn>          open a project");
        eprintln!("  chikn new <parent-dir> <name>          create a new project");
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
