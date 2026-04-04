//! Compile: merge manuscript documents into a single output file.
//!
//! Walks the hierarchy in order, concatenates HTML content,
//! then uses Pandoc to convert to the target format.

use crate::core::project::reader;
use crate::models::{Project, TreeNode};
use crate::utils::error::ChiknError;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Supported output formats
pub const FORMATS: &[(&str, &str)] = &[
    ("docx", "Word Document (.docx)"),
    ("pdf", "PDF (.pdf)"),
    ("epub", "EPUB (.epub)"),
    ("html", "HTML (.html)"),
    ("odt", "OpenDocument (.odt)"),
];

/// Compile the manuscript into a single output file.
///
/// Merges all manuscript documents in hierarchy order,
/// then converts via Pandoc to the target format.
pub fn compile(
    project_path: &Path,
    output_path: &Path,
    format: &str,
    title: Option<&str>,
    author: Option<&str>,
) -> Result<(), ChiknError> {
    let project = reader::read_project(project_path)?;

    // Collect manuscript content in hierarchy order
    let mut html = String::new();
    collect_manuscript_html(&project.hierarchy, &project, &mut html);

    if html.trim().is_empty() {
        return Err(ChiknError::InvalidFormat(
            "No manuscript content to compile".to_string(),
        ));
    }

    // Wrap in a basic HTML document
    let doc_title = title.unwrap_or(&project.name);
    let full_html = format!(
        "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<title>{}</title>\n</head>\n<body>\n{}\n</body>\n</html>",
        doc_title, html
    );

    // Write to temp file
    let temp_dir = std::env::temp_dir();
    let temp_html = temp_dir.join(format!("compile_{}.html", uuid::Uuid::new_v4()));
    fs::write(&temp_html, &full_html)?;

    // Build Pandoc command
    let mut cmd = Command::new("pandoc");
    cmd.arg("-f").arg("html");
    cmd.arg("-t").arg(pandoc_format(format));
    cmd.arg("-o").arg(output_path);
    cmd.arg("--standalone");

    if let Some(t) = title {
        cmd.arg("--metadata").arg(format!("title={}", t));
    }
    if let Some(a) = author {
        cmd.arg("--metadata").arg(format!("author={}", a));
    }

    cmd.arg(&temp_html);

    let output = cmd
        .output()
        .map_err(|e| ChiknError::Unknown(format!("Failed to run Pandoc: {}", e)))?;

    let _ = fs::remove_file(&temp_html);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ChiknError::Unknown(format!(
            "Pandoc compile failed: {}",
            stderr
        )));
    }

    Ok(())
}

/// Recursively collect HTML content from manuscript documents in hierarchy order.
fn collect_manuscript_html(nodes: &[TreeNode], project: &Project, html: &mut String) {
    for node in nodes {
        match node {
            TreeNode::Document { id, path, .. } => {
                if path.starts_with("manuscript/") && path.ends_with(".html") {
                    if let Some(doc) = project.documents.get(id) {
                        if !doc.content.trim().is_empty() {
                            html.push_str(&doc.content);
                            html.push('\n');
                        }
                    }
                }
            }
            TreeNode::Folder { children, .. } => {
                collect_manuscript_html(children, project, html);
            }
        }
    }
}

fn pandoc_format(format: &str) -> &str {
    match format {
        "docx" => "docx",
        "pdf" => "pdf",
        "epub" => "epub3",
        "html" => "html",
        "odt" => "odt",
        _ => "docx",
    }
}
