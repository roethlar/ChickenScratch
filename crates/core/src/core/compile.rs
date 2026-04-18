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
        .map(|(_, _, content)| strip_comments(&content))
        .collect();

    // Transform footnotes across all sections with continuous numbering.
    // Inputs: `<sup class="footnote" data-body="...">●</sup>`
    // Output: per-section, numbered `<sup class="footnote-ref"><a href="#fnN">N</a></sup>`
    //         plus a single footnotes section appended at the end.
    let (sections, footnotes_html) = transform_footnotes(sections);

    if sections.is_empty() {
        return Err(ChiknError::InvalidFormat(
            "No manuscript content to compile".to_string(),
        ));
    }

    // Calculate approximate word count for title page
    let word_count: usize = sections.iter().map(|s| {
        let text = s.chars().fold(String::new(), |mut acc, c| {
            if c == '<' || c == '>' { acc.push(' '); acc } else { acc.push(c); acc }
        });
        text.split_whitespace().count()
    }).sum();

    // Build the HTML
    let mut html = String::new();

    // Title page
    if opts.include_title_page {
        html.push_str("<div class=\"title-page\">\n");
        if let Some(a) = doc_author {
            html.push_str(&format!("<p class=\"tp-contact\">{}</p>\n", a));
        }
        html.push_str(&format!("<p class=\"tp-wordcount\">Approx. {} words</p>\n",
            ((word_count + 50) / 100) * 100)); // round to nearest 100
        html.push_str(&format!("<h1 class=\"tp-title\">{}</h1>\n", doc_title));
        if let Some(a) = doc_author {
            html.push_str(&format!("<p class=\"tp-author\">by {}</p>\n", a));
        }
        html.push_str("</div>\n<div style=\"page-break-after:always\"></div>\n\n");
    }

    // Join sections with separator
    let separator_html = format!(
        "<p class=\"section-break\" style=\"text-align:center;margin:2em 0;\">{}</p>\n",
        separator
    );

    for (i, section) in sections.iter().enumerate() {
        html.push_str(section);
        html.push('\n');
        if i < sections.len() - 1 {
            html.push_str(&separator_html);
        }
    }

    // Append collected footnotes section
    if !footnotes_html.is_empty() {
        html.push_str(&footnotes_html);
    }

    // Manuscript format CSS
    let css = if opts.manuscript_format {
        format!(
            "body {{ font-family: 'Courier New', Courier, monospace; font-size: 12pt; line-height: 2; margin: 1in; }}\n\
             p {{ text-indent: 0.5in; margin: 0; }}\n\
             p:first-child {{ text-indent: 0; }}\n\
             h1, h2, h3 {{ text-indent: 0; margin-top: 2em; font-family: 'Courier New', Courier, monospace; }}\n\
             .title-page {{ text-align: center; padding-top: 33%; }}\n\
             .tp-contact {{ text-align: left; position: absolute; top: 1in; left: 1in; }}\n\
             .tp-wordcount {{ text-align: right; position: absolute; top: 1in; right: 1in; }}\n\
             .tp-title {{ font-size: 24pt; margin-top: 2em; text-transform: uppercase; }}\n\
             .tp-author {{ font-size: 14pt; }}\n\
             .section-break {{ font-family: 'Courier New', Courier, monospace; }}")
    } else {
        format!(
            "body {{ font-family: '{}', serif; font-size: {}pt; line-height: {}; margin: {}in; }}\n\
             p {{ text-indent: 0.5in; margin: 0; }}\n\
             p:first-child {{ text-indent: 0; }}\n\
             h1, h2, h3 {{ text-indent: 0; margin-top: 2em; }}\n\
             .title-page {{ text-align: center; padding-top: 20%; }}\n\
             .tp-title {{ font-size: 2em; margin-bottom: 0.5em; }}\n\
             .tp-author {{ font-size: 1.2em; color: #555; }}\n\
             .tp-contact, .tp-wordcount {{ font-size: 0.9em; color: #777; }}",
            font, font_size, line_spacing, margin
        )
    };

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
                if path.starts_with("manuscript/") && path.ends_with(".html") {
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


/// Strip `<span class="comment" data-comment-id="...">...</span>` wrappers on
/// compile. Preserves inner text. Tracks depth so non-comment spans pass through.
fn strip_comments(html: &str) -> String {
    // Walk through, tracking depth of comment spans we've opened.
    let mut out = String::with_capacity(html.len());
    let mut depth: i32 = 0;
    let mut i = 0;
    let s = html;
    while i < s.len() {
        if s[i..].starts_with("<span") {
            // Find end of tag
            if let Some(end_rel) = s[i..].find('>') {
                let tag = &s[i..i + end_rel + 1];
                if tag.contains("class=\"comment\"") || tag.contains("class='comment'") {
                    depth += 1;
                    i += end_rel + 1;
                    continue;
                }
                // non-comment span — pass through
                out.push_str(tag);
                i += end_rel + 1;
                continue;
            }
        }
        if s[i..].starts_with("</span>") {
            if depth > 0 {
                depth -= 1;
                i += 7; // len("</span>")
                continue;
            }
            out.push_str("</span>");
            i += 7;
            continue;
        }
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Transform editor-stored footnotes `<sup class="footnote" data-body="...">●</sup>`
/// into pandoc-native footnote HTML with continuous numbering across sections.
/// Returns (transformed_sections, trailing_footnotes_section_html).
fn transform_footnotes(sections: Vec<String>) -> (Vec<String>, String) {
    let mut bodies: Vec<String> = Vec::new();
    let mut transformed: Vec<String> = Vec::with_capacity(sections.len());

    for section in sections {
        let mut out = String::with_capacity(section.len());
        let mut rest = section.as_str();
        loop {
            // Find next footnote opening tag
            let idx = match rest.find("<sup class=\"footnote\"") {
                Some(i) => i,
                None => { out.push_str(rest); break; }
            };
            out.push_str(&rest[..idx]);
            // Find end of opening tag
            let tag_end_rel = match rest[idx..].find('>') {
                Some(e) => e,
                None => { out.push_str(rest); break; }
            };
            let tag = &rest[idx..idx + tag_end_rel + 1];

            // Extract body from data-body attribute
            let body = extract_data_body(tag).unwrap_or_default();

            // Skip to the closing </sup>
            let inner_start = idx + tag_end_rel + 1;
            let close_rel = match rest[inner_start..].find("</sup>") {
                Some(e) => e,
                None => { out.push_str(&rest[idx..]); break; }
            };
            let after_close = inner_start + close_rel + "</sup>".len();

            // Emit numbered reference
            bodies.push(body);
            let n = bodies.len();
            out.push_str(&format!(
                "<sup class=\"footnote-ref\"><a href=\"#fn{}\" id=\"fnref{}\">{}</a></sup>",
                n, n, n
            ));

            rest = &rest[after_close..];
        }
        transformed.push(out);
    }

    let footnotes_html = if bodies.is_empty() {
        String::new()
    } else {
        let mut s = String::from("\n<section class=\"footnotes\">\n<hr/>\n<ol>\n");
        for (i, body) in bodies.iter().enumerate() {
            let n = i + 1;
            s.push_str(&format!(
                "<li id=\"fn{}\"><p>{} <a href=\"#fnref{}\">↩</a></p></li>\n",
                n, html_escape(body), n
            ));
        }
        s.push_str("</ol>\n</section>\n");
        s
    };

    (transformed, footnotes_html)
}

fn extract_data_body(tag: &str) -> Option<String> {
    let key = "data-body=\"";
    let start = tag.find(key)? + key.len();
    let end = tag[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
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
