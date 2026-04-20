use clap::Parser;
use std::path::{Path, PathBuf};
use std::process;

use chickenscratch_core::core::project::reader::read_project;
use chickenscratch_core::scrivener::converter::import_scriv;
use chickenscratch_core::scrivener::exporter::export_to_scriv;

#[derive(Parser)]
#[command(name = "chikn-converter")]
#[command(about = "Convert between Scrivener (.scriv) and ChickenScratch (.chikn) formats")]
struct Cli {
    /// Input file (.scriv or .chikn) — direction is detected automatically
    input: PathBuf,
    /// Output path (default: same directory as input, opposite format)
    output: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    let result = match detect_format(&cli.input) {
        Some(Format::Scriv) => scriv_to_chikn(&cli.input, cli.output.as_deref()),
        Some(Format::Chikn) => chikn_to_scriv(&cli.input, cli.output.as_deref()),
        None => {
            eprintln!(
                "Cannot determine format of '{}'. Expected a .scriv or .chikn directory.",
                cli.input.display()
            );
            process::exit(1);
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

enum Format {
    Scriv,
    Chikn,
}

fn detect_format(path: &Path) -> Option<Format> {
    let ext = path.extension()?.to_str()?;
    match ext {
        "scriv" | "scrivx" => Some(Format::Scriv),
        "chikn" => Some(Format::Chikn),
        _ => None,
    }
}

fn scriv_to_chikn(
    scriv_path: &Path,
    output: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !scriv_path.exists() {
        return Err(format!("Path does not exist: {}", scriv_path.display()).into());
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

    println!("{} -> {}", scriv_path.display(), output_path.display());

    let project = import_scriv(scriv_path, &output_path)?;

    println!(
        "\"{}\" — {} documents, {} top-level items",
        project.name,
        project.documents.len(),
        project.hierarchy.len()
    );

    Ok(())
}

fn chikn_to_scriv(
    chikn_path: &Path,
    output: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !chikn_path.exists() {
        return Err(format!("Path does not exist: {}", chikn_path.display()).into());
    }

    let project = read_project(chikn_path)?;

    let output_path = match output {
        Some(p) => p.to_path_buf(),
        None => {
            let parent = chikn_path.parent().unwrap_or(Path::new("."));
            parent.join(format!("{}.scriv", project.name))
        }
    };

    println!("{} -> {}", chikn_path.display(), output_path.display());

    export_to_scriv(&project, &output_path)?;

    println!(
        "\"{}\" — {} documents",
        project.name,
        project.documents.len()
    );

    Ok(())
}
