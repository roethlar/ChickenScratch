use chickenscratch_core::core::compile;
use chickenscratch_core::core::git;
use chickenscratch_core::core::project::hierarchy;
use chickenscratch_core::core::project::{reader, writer};
use chickenscratch_core::{ChiknError, Document, Project, TreeNode};
use std::fs;
use std::path::Path;
use std::process::Command;

#[tauri::command]
pub fn compile_project(
    project_path: String,
    output_path: String,
    format: String,
    title: Option<String>,
    author: Option<String>,
) -> Result<(), ChiknError> {
    let settings = super::settings::get_app_settings();
    let options = compile::CompileOptions {
        font: Some(settings.compile.font),
        font_size: Some(settings.compile.font_size),
        line_spacing: Some(settings.compile.line_spacing),
        margin_inches: Some(settings.compile.margin_inches),
    };

    compile::compile(
        Path::new(&project_path),
        Path::new(&output_path),
        &format,
        title.as_deref(),
        author.as_deref(),
        Some(options),
    )
}

#[tauri::command]
pub fn get_compile_formats() -> Vec<(String, String)> {
    compile::FORMATS
        .iter()
        .map(|(ext, desc)| (ext.to_string(), desc.to_string()))
        .collect()
}

/// File extensions that Pandoc can convert to HTML
const PANDOC_IMPORT_EXTENSIONS: &[&str] = &[
    "docx", "doc", "odt", "rtf", "epub", "latex", "tex", "md", "markdown",
    "rst", "org", "textile", "mediawiki", "html", "htm", "txt", "csv",
    "json", "fb2", "pptx", "xlsx",
];

/// Import a file into an existing project. Uses Pandoc for conversion when needed.
#[tauri::command]
pub fn import_file(
    project_path: String,
    file_path: String,
    parent_id: Option<String>,
) -> Result<Project, ChiknError> {
    let path = Path::new(&file_path);
    let mut project = reader::read_project(Path::new(&project_path))?;

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Imported")
        .to_string();

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Convert to HTML
    let html_content = match ext.as_str() {
        "html" | "htm" => fs::read_to_string(path)?,
        "txt" => {
            // Plain text: wrap paragraphs in <p> tags
            let content = fs::read_to_string(path)?;
            content
                .split("\n\n")
                .filter(|p| !p.trim().is_empty())
                .map(|p| format!("<p>{}</p>", p.trim()))
                .collect::<Vec<_>>()
                .join("\n")
        }
        _ => {
            // Use Pandoc to convert to HTML
            convert_to_html_via_pandoc(path, &ext)?
        }
    };

    let doc_id = uuid::Uuid::new_v4().to_string();
    let slug =
        chickenscratch_core::utils::slug::unique_slug(&name, "manuscript/", &project.documents);
    let doc_path = format!("manuscript/{}.html", slug);
    let now = chrono::Utc::now().to_rfc3339();

    let document = Document {
        id: doc_id.clone(),
        name: name.clone(),
        path: doc_path.clone(),
        content: html_content,
        parent_id: parent_id.clone(),
        created: now.clone(),
        modified: now,
        ..Default::default()
    };

    project.documents.insert(doc_id.clone(), document);

    let node = TreeNode::Document {
        id: doc_id,
        name,
        path: doc_path,
    };

    match parent_id {
        Some(pid) => hierarchy::add_child_to_folder(&mut project.hierarchy, &pid, node)?,
        None => hierarchy::add_document_to_hierarchy(&mut project.hierarchy, node),
    }

    writer::write_project(&mut project)?;
    Ok(project)
}

fn convert_to_html_via_pandoc(file_path: &Path, ext: &str) -> Result<String, ChiknError> {
    // Map file extensions to Pandoc input format names
    let format = match ext {
        "docx" | "doc" => "docx",
        "odt" => "odt",
        "rtf" => "rtf",
        "epub" => "epub",
        "latex" | "tex" => "latex",
        "md" | "markdown" => "markdown",
        "rst" => "rst",
        "org" => "org",
        "textile" => "textile",
        "mediawiki" => "mediawiki",
        "fb2" => "fb2",
        "pptx" => "pptx",
        _ => "markdown", // fallback
    };

    // Try common Pandoc paths
    let pandoc = find_pandoc()?;

    let output = Command::new(&pandoc)
        .arg("-f")
        .arg(format)
        .arg("-t")
        .arg("html")
        .arg("--wrap=none")
        .arg(file_path)
        .output()
        .map_err(|e| ChiknError::Unknown(format!("Failed to run Pandoc: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ChiknError::Unknown(format!(
            "Pandoc conversion failed: {}",
            stderr
        )));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| ChiknError::Unknown(format!("Invalid UTF-8 from Pandoc: {}", e)))
}

fn find_pandoc() -> Result<String, ChiknError> {
    // Check settings for custom path
    let settings = super::settings::get_app_settings();
    if let Some(ref p) = settings.general.pandoc_path {
        if !p.is_empty() {
            return Ok(p.clone());
        }
    }

    for candidate in &[
        "pandoc",
        "/usr/local/bin/pandoc",
        "/opt/homebrew/bin/pandoc",
        "/usr/bin/pandoc",
    ] {
        if Command::new(candidate)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(candidate.to_string());
        }
    }

    Err(ChiknError::Unknown(
        "Pandoc is not installed. Required for importing this file format.".to_string(),
    ))
}

/// Import a folder of files as a new project.
#[tauri::command]
pub fn import_markdown_folder(
    folder_path: String,
    output_path: String,
) -> Result<Project, ChiknError> {
    let folder = Path::new(&folder_path);
    let output = Path::new(&output_path);

    let name = folder
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Imported")
        .to_string();

    let mut project = writer::create_project(output, &name)?;

    let mut entries: Vec<_> = fs::read_dir(folder)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Skip files we can't import
        if !PANDOC_IMPORT_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }

        let doc_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();

        let html_content = match ext.as_str() {
            "html" | "htm" => fs::read_to_string(&path).unwrap_or_default(),
            "txt" => {
                let content = fs::read_to_string(&path).unwrap_or_default();
                content
                    .split("\n\n")
                    .filter(|p| !p.trim().is_empty())
                    .map(|p| format!("<p>{}</p>", p.trim()))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            _ => convert_to_html_via_pandoc(&path, &ext).unwrap_or_default(),
        };

        let doc_id = uuid::Uuid::new_v4().to_string();
        let slug = chickenscratch_core::utils::slug::unique_slug(
            &doc_name,
            "manuscript/",
            &project.documents,
        );
        let doc_path = format!("manuscript/{}.html", slug);
        let now = chrono::Utc::now().to_rfc3339();

        let document = Document {
            id: doc_id.clone(),
            name: doc_name.clone(),
            path: doc_path.clone(),
            content: html_content,
            parent_id: None,
            created: now.clone(),
            modified: now,
            ..Default::default()
        };

        project.documents.insert(doc_id.clone(), document);
        project.hierarchy.push(TreeNode::Document {
            id: doc_id,
            name: doc_name,
            path: doc_path,
        });
    }

    writer::write_project(&mut project)?;
    let _ = git::save_revision(output, &format!("Imported from: {}", name));

    Ok(project)
}
