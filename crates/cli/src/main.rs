use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process;

use chickenscratch_core::scrivener::converter::import_scriv;
use chickenscratch_core::scrivener::exporter::export_to_scriv;
use chickenscratch_core::core::project::reader::read_project;

#[derive(Parser)]
#[command(name = "chickenscratch")]
#[command(about = "Convert between Scrivener and Chicken Scratch formats")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Import a Scrivener project (.scriv) into .chikn format
    Import {
        /// Path to the .scriv project
        input: PathBuf,
        /// Output path for the .chikn project (default: same directory as input)
        output: Option<PathBuf>,
    },
    /// Export a .chikn project to Scrivener (.scriv) format
    Export {
        /// Path to the .chikn project
        input: PathBuf,
        /// Output path for the .scriv project (default: same directory as input)
        output: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Import { input, output } => do_import(&input, output.as_deref()),
        Command::Export { input, output } => do_export(&input, output.as_deref()),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

fn do_import(scriv_path: &Path, output: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    if !scriv_path.exists() {
        return Err(format!("Input path does not exist: {}", scriv_path.display()).into());
    }

    let output_path = match output {
        Some(p) => p.to_path_buf(),
        None => {
            let stem = scriv_path
                .file_stem()
                .ok_or("Cannot determine project name from input path")?;
            let parent = scriv_path.parent().unwrap_or(Path::new("."));
            parent.join(format!("{}.chikn", stem.to_string_lossy()))
        }
    };

    println!(
        "Importing {} -> {}",
        scriv_path.display(),
        output_path.display()
    );

    let project = import_scriv(scriv_path, &output_path)?;

    println!("Imported \"{}\"", project.name);
    println!(
        "  {} documents, {} top-level items",
        project.documents.len(),
        project.hierarchy.len()
    );
    println!("  Written to {}", output_path.display());

    Ok(())
}

fn do_export(chikn_path: &Path, output: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    if !chikn_path.exists() {
        return Err(format!("Input path does not exist: {}", chikn_path.display()).into());
    }

    let project = read_project(chikn_path)?;

    let output_path = match output {
        Some(p) => p.to_path_buf(),
        None => {
            let parent = chikn_path.parent().unwrap_or(Path::new("."));
            parent.join(format!("{}.scriv", project.name))
        }
    };

    println!(
        "Exporting {} -> {}",
        chikn_path.display(),
        output_path.display()
    );

    export_to_scriv(&project, &output_path)?;

    println!("Exported \"{}\"", project.name);
    println!(
        "  {} documents",
        project.documents.len()
    );
    println!("  Written to {}", output_path.display());

    Ok(())
}
