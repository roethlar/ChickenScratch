use clap::Parser;
use std::path::{Path, PathBuf};
use std::process;

use chickenscratch_core::core::project::reader::read_project;
use chikn_converter::{export_to_scriv, import_scriv};

#[derive(Parser)]
#[command(name = "chikn-converter")]
#[command(about = "Convert between Scrivener (.scriv) and ChickenScratch (.chikn) formats")]
struct Cli {
    /// Input file (.scriv or .chikn) — direction is detected automatically
    input: PathBuf,
    /// Output path (default: same directory as input, opposite format)
    output: Option<PathBuf>,
    /// Path to pandoc executable (for portable or non-system installs)
    #[arg(long, value_name = "PATH")]
    pandoc: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    let pandoc = cli.pandoc.as_deref();
    let result = match detect_format(&cli.input) {
        Some(Format::Scriv) => scriv_to_chikn(&cli.input, cli.output.as_deref(), pandoc),
        Some(Format::Chikn) => chikn_to_scriv(&cli.input, cli.output.as_deref(), pandoc),
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
    pandoc_path: Option<&Path>,
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

    let project = import_scriv(scriv_path, &output_path, pandoc_path)?;

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
    pandoc_path: Option<&Path>,
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

    export_to_scriv(&project, &output_path, pandoc_path)?;

    println!(
        "\"{}\" — {} documents",
        project.name,
        project.documents.len()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::fs;
    use tempfile::TempDir;

    fn tree_snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
        fn visit(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
            for entry in fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                let metadata = fs::symlink_metadata(&path).unwrap();
                if metadata.is_dir() {
                    out.insert(format!("{relative}/"), Vec::new());
                    visit(root, &path, out);
                } else if metadata.is_file() {
                    out.insert(relative, fs::read(&path).unwrap());
                }
            }
        }

        let mut snapshot = BTreeMap::new();
        visit(root, root, &mut snapshot);
        snapshot
    }

    #[test]
    fn chikn_export_does_not_mutate_corrupt_source() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("Corrupt.chikn");
        fs::create_dir(&project).unwrap();
        for folder in ["manuscript", "research", "templates", "settings"] {
            fs::create_dir(project.join(folder)).unwrap();
        }
        fs::write(
            project.join("project.yaml"),
            r#"format_version: '1.2'
id: "project"
name: "Corrupt"
created: "2025-01-01T00:00:00Z"
modified: "2025-01-01T00:00:00Z"
hierarchy:
  - type: Document
    id: "chapter"
    name: "Chapter"
    path: "manuscript/chapter.md"
"#,
        )
        .unwrap();
        fs::write(project.join("manuscript/chapter.md"), "# Chapter\n").unwrap();
        let sidecar = project.join("manuscript/chapter.meta");
        fs::write(&sidecar, "id: [").unwrap();

        let before = tree_snapshot(&project);
        // An existing file makes Scrivener output-directory creation fail
        // before Pandoc is needed. Source loading must nevertheless remain
        // a byte-for-byte read-only operation.
        let blocked_output = temp.path().join("blocked.scriv");
        fs::write(&blocked_output, "not a directory").unwrap();

        let result = chikn_to_scriv(&project, Some(&blocked_output), None);

        assert!(result.is_err(), "blocked output path must fail export");
        assert_eq!(
            before,
            tree_snapshot(&project),
            "export must not create folders or quarantine corrupt metadata in its source"
        );
        assert_eq!(fs::read(&sidecar).unwrap(), b"id: [");
    }
}
