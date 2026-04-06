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

/// Compile settings passed from the frontend
#[derive(Debug, Clone, Default)]
pub struct CompileOptions {
    pub font: Option<String>,
    pub font_size: Option<f32>,
    pub line_spacing: Option<f32>,
    pub margin_inches: Option<f32>,
}

/// Compile the manuscript into a single output file.
pub fn compile(
    project_path: &Path,
    output_path: &Path,
    format: &str,
    title: Option<&str>,
    author: Option<&str>,
    options: Option<CompileOptions>,
) -> Result<(), ChiknError> {
    let project = reader::read_project(project_path)?;

    let mut html = String::new();
    collect_manuscript_html(&project.hierarchy, &project, &mut html);

    if html.trim().is_empty() {
        return Err(ChiknError::InvalidFormat(
            "No manuscript content to compile".to_string(),
        ));
    }

    let opts = options.unwrap_or_default();
    let doc_title = title.unwrap_or(&project.name);
    let font = opts.font.as_deref().unwrap_or("Times New Roman");
    let font_size = opts.font_size.unwrap_or(12.0);
    let line_spacing = opts.line_spacing.unwrap_or(2.0);
    let margin = opts.margin_inches.unwrap_or(1.0);

    // Embed CSS for formatting
    let css = format!(
        "body {{ font-family: '{}', serif; font-size: {}pt; line-height: {}; margin: {}in; }}\n\
         p {{ text-indent: 0.5in; margin: 0; }}\n\
         p:first-child {{ text-indent: 0; }}\n\
         h1, h2, h3 {{ text-indent: 0; margin-top: 2em; }}",
        font, font_size, line_spacing, margin
    );

    let full_html = format!(
        "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n\
         <title>{}</title>\n<style>\n{}\n</style>\n</head>\n<body>\n{}\n</body>\n</html>",
        doc_title, css, html
    );

    let temp_dir = std::env::temp_dir();
    let temp_html = temp_dir.join(format!("compile_{}.html", uuid::Uuid::new_v4()));
    fs::write(&temp_html, &full_html)?;

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

    // PDF-specific: set margins via Pandoc variables
    if format == "pdf" {
        cmd.arg("--variable").arg(format!("geometry:margin={}in", margin));
        cmd.arg("--variable").arg(format!("fontsize={}pt", font_size as u32));
        cmd.arg("--variable").arg(format!("mainfont={}", font));
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

fn collect_manuscript_html(nodes: &[TreeNode], project: &Project, html: &mut String) {
    for node in nodes {
        match node {
            TreeNode::Document { id, path, .. } => {
                if path.starts_with("manuscript/") && path.ends_with(".html") {
                    if let Some(doc) = project.documents.get(id) {
                        if doc.include_in_compile && !doc.content.trim().is_empty() {
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
