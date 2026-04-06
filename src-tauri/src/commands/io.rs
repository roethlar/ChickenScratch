use chickenscratch_core::core::compile;
use chickenscratch_core::core::git;
use chickenscratch_core::core::project::{reader, writer};
use chickenscratch_core::core::project::hierarchy;
use chickenscratch_core::{ChiknError, Document, Project, TreeNode};
use std::fs;
use std::path::Path;

#[tauri::command]
pub fn compile_project(
    project_path: String,
    output_path: String,
    format: String,
    title: Option<String>,
    author: Option<String>,
) -> Result<(), ChiknError> {
    // Read compile settings
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

/// Import a single .md or .txt file into an existing project as a new document.
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

    let content = fs::read_to_string(path)?;

    // Wrap plain text in <p> tags if it's not already HTML
    let html_content = if file_path.ends_with(".txt") || !content.trim_start().starts_with('<') {
        content
            .split("\n\n")
            .filter(|p| !p.trim().is_empty())
            .map(|p| format!("<p>{}</p>", p.trim()))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        content
    };

    let doc_id = uuid::Uuid::new_v4().to_string();
    let slug = chickenscratch_core::utils::slug::unique_slug(&name, "manuscript/", &project.documents);
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

/// Import a folder of .md/.txt files as a new project.
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

    // Read all .md and .txt files
    let mut entries: Vec<_> = fs::read_dir(folder)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            path.is_file()
                && matches!(
                    path.extension().and_then(|x| x.to_str()),
                    Some("md" | "txt" | "html")
                )
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let doc_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();

        let content = fs::read_to_string(&path).unwrap_or_default();

        let html_content =
            if path.extension().and_then(|x| x.to_str()) == Some("html") || content.trim_start().starts_with('<') {
                content
            } else {
                content
                    .split("\n\n")
                    .filter(|p| !p.trim().is_empty())
                    .map(|p| format!("<p>{}</p>", p.trim()))
                    .collect::<Vec<_>>()
                    .join("\n")
            };

        let doc_id = uuid::Uuid::new_v4().to_string();
        let slug =
            chickenscratch_core::utils::slug::unique_slug(&doc_name, "manuscript/", &project.documents);
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
