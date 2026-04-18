//! Compile: merge manuscript documents into a single output file.
//!
//! Walks the hierarchy in order, concatenates markdown content,
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
    /// Section separator between documents (e.g., "# # #", "* * *", "***")
    pub section_separator: Option<String>,
    /// Include a title page with title, author, word count
    pub include_title_page: bool,
    /// Use Shunn standard manuscript format (Courier 12pt, double-spaced, 1" margins)
    pub manuscript_format: bool,
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

    let opts = options.unwrap_or_default();

    // Apply manuscript format preset if requested
    let (font, font_size, line_spacing, margin) = if opts.manuscript_format {
        ("Courier New", 12.0_f32, 2.0_f32, 1.0_f32)
    } else {
        (
            opts.font.as_deref().unwrap_or("Times New Roman"),
            opts.font_size.unwrap_or(12.0),
            opts.line_spacing.unwrap_or(2.0),
            opts.margin_inches.unwrap_or(1.0),
        )
    };

    let separator = opts.section_separator.as_deref().unwrap_or("# # #");
    let doc_title = title.unwrap_or(&project.name);
    let doc_author = author.or_else(|| project.metadata.author.as_deref());

    // Collect manuscript sections in order, respecting compile_order
    let mut ordered_docs: Vec<(i32, usize, String)> = Vec::new(); // (compile_order, hierarchy_index, content)
    let mut idx = 0;
    collect_ordered_sections(&project.hierarchy, &project, &mut ordered_docs, &mut idx);
    ordered_docs.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let sections: Vec<String> = ordered_docs
        .into_iter()
        .map(|(_, _, content)| content)
        .collect();

    if sections.is_empty() {
        return Err(ChiknError::InvalidFormat(
            "No manuscript content to compile".to_string(),
        ));
    }

    // Calculate approximate word count for title page
    let word_count: usize = sections
        .iter()
        .map(|s| s.split_whitespace().count())
        .sum();

    // Build the markdown document
    let mut md = String::new();

    // Title page: rendered as simple centered markdown
    if opts.include_title_page {
        if let Some(a) = doc_author {
            md.push_str(&format!("{}\n\n", a));
        }
        md.push_str(&format!(
            "Approx. {} words\n\n",
            ((word_count + 50) / 100) * 100
        ));
        md.push_str(&format!("# {}\n\n", doc_title));
        if let Some(a) = doc_author {
            md.push_str(&format!("by {}\n\n", a));
        }
        // Page break before first section
        md.push_str("\\newpage\n\n");
    }

    // Join sections with markdown separator
    for (i, section) in sections.iter().enumerate() {
        md.push_str(section);
        if !section.ends_with('\n') {
            md.push('\n');
        }
        if i < sections.len() - 1 {
            md.push_str(&format!("\n\n{}\n\n", separator));
        }
    }

    let temp_dir = std::env::temp_dir();
    let temp_md = temp_dir.join(format!("compile_{}.md", uuid::Uuid::new_v4()));
    fs::write(&temp_md, &md)?;

    let mut cmd = Command::new("pandoc");
    cmd.arg("-f").arg("markdown");
    cmd.arg("-t").arg(pandoc_format(format));
    cmd.arg("-o").arg(output_path);
    cmd.arg("--standalone");

    if let Some(t) = title {
        cmd.arg("--metadata").arg(format!("title={}", t));
    }
    if let Some(a) = doc_author {
        cmd.arg("--metadata").arg(format!("author={}", a));
    }

    // PDF-specific
    if format == "pdf" {
        cmd.arg("--variable").arg(format!("geometry:margin={}in", margin));
        cmd.arg("--variable").arg(format!("fontsize={}pt", font_size as u32));
        if !opts.manuscript_format {
            cmd.arg("--variable").arg(format!("mainfont={}", font));
        }
    }

    let _ = line_spacing; // currently informational; pandoc defaults used

    cmd.arg(&temp_md);

    let output = cmd
        .output()
        .map_err(|e| ChiknError::Unknown(format!("Failed to run Pandoc: {}", e)))?;

    let _ = fs::remove_file(&temp_md);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ChiknError::Unknown(format!(
            "Pandoc compile failed: {}",
            stderr
        )));
    }

    Ok(())
}

/// Collect sections with compile_order for sorting.
fn collect_ordered_sections(
    nodes: &[TreeNode],
    project: &Project,
    sections: &mut Vec<(i32, usize, String)>,
    idx: &mut usize,
) {
    for node in nodes {
        match node {
            TreeNode::Document { id, path, .. } => {
                if path.starts_with("manuscript/") && path.ends_with(".md") {
                    if let Some(doc) = project.documents.get(id) {
                        if doc.include_in_compile && !doc.content.trim().is_empty() {
                            sections.push((doc.compile_order, *idx, doc.content.clone()));
                            *idx += 1;
                        }
                    }
                }
            }
            TreeNode::Folder { children, .. } => {
                collect_ordered_sections(children, project, sections, idx);
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
