//! Compile: merge manuscript documents into a single output file.
//!
//! Walks the hierarchy in order, concatenates markdown content,
//! then uses Pandoc to convert to the target format.

use crate::core::project::reader;
use crate::models::{Project, TreeNode};
use crate::utils::error::ChiknError;
use crate::utils::process::{output_bounded, PANDOC_OUTPUT_LIMIT_BYTES, PANDOC_TIMEOUT};
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
    /// Override path to the pandoc binary. Falls back to `pandoc` on $PATH
    /// when None. The settings UI lets users point at a custom install when
    /// system pandoc isn't visible to the app sandbox.
    pub pandoc_path: Option<String>,
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
    // Repairs-disabled read: compiling is a pure export and must never
    // mutate the project folder (a Degraded project can be exported too).
    let project = reader::read_project_readonly(project_path)?;

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
    let doc_author = author.or(project.metadata.author.as_deref());

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
    let word_count: usize = sections.iter().map(|s| s.split_whitespace().count()).sum();

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

    // Join sections with markdown separator. The editor's footnote element
    // becomes a Pandoc reference footnote on the way (definitions collected
    // here, appended after the last section).
    let mut footnote_definitions: Vec<String> = Vec::new();
    let mut footnote_counter = 0usize;
    for (i, section) in sections.iter().enumerate() {
        let section =
            footnotes_to_references(section, &mut footnote_counter, &mut footnote_definitions);
        md.push_str(&section);
        if !section.ends_with('\n') {
            md.push('\n');
        }
        if i < sections.len() - 1 {
            md.push_str(&format!("\n\n{}\n\n", separator));
        }
    }
    if !footnote_definitions.is_empty() {
        md.push('\n');
        for definition in &footnote_definitions {
            md.push_str(definition);
            md.push_str("\n\n");
        }
    }

    let temp_dir = std::env::temp_dir();
    let temp_md = temp_dir.join(format!("compile_{}.md", uuid::Uuid::new_v4()));
    fs::write(&temp_md, &md)?;

    let pandoc_bin = opts
        .pandoc_path
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("pandoc");
    let mut cmd = Command::new(pandoc_bin);
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

    // PDF-specific. F-014: typography settings only land in PDF output;
    // DOCX/ODT/HTML use Pandoc defaults because applying them properly
    // requires reference templates (DOCX/ODT) or a CSS file (HTML), which
    // the user hasn't asked us to bundle yet. The settings UI continues
    // to advertise font/size/spacing/margins for all formats — this is
    // honest about what currently lands in each output (see comment
    // below for non-PDF behavior).
    if format == "pdf" {
        cmd.arg("--variable")
            .arg(format!("geometry:margin={}in", margin));
        cmd.arg("--variable")
            .arg(format!("fontsize={}pt", font_size as u32));
        if !opts.manuscript_format {
            cmd.arg("--variable").arg(format!("mainfont={}", font));
        }
        // Pandoc's PDF templates honour `linestretch` (1.0 = single, 2.0 =
        // double). Previously ignored entirely (`let _ = line_spacing;`),
        // so a user who picked single-spaced PDF still got double-spaced
        // output regardless of their setting.
        cmd.arg("--variable")
            .arg(format!("linestretch={}", line_spacing));
    }
    // Non-PDF formats: line_spacing/font/margins flow into the output only
    // when a Pandoc reference template is supplied (`--reference-doc=...`
    // for DOCX/ODT). Bundling defaults that respect these settings is
    // tracked separately; for now, DOCX/ODT/HTML use Pandoc's built-in
    // defaults and the settings are documented as PDF-effective only.

    cmd.arg(&temp_md);

    let output = output_bounded(&mut cmd, PANDOC_TIMEOUT, PANDOC_OUTPUT_LIMIT_BYTES)
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
/// The editor stores a footnote as `<sup class="footnote" data-body="…">●</sup>`
/// (`ui/src/components/editor/FootnoteNode.ts`). Pandoc never reads that
/// attribute, so until now every footnote vanished from compiled output
/// (PLAN_SCRIVENER_FIDELITY slice 4, review finding plan-3 R2-1). Each one
/// becomes a reference footnote `[^cs-fnN]`, N unique across the compiled
/// document, with its definition collected for the end. The body is plain
/// text and is written as literal text: every ASCII punctuation character
/// escaped, blank lines kept as separate (indented) paragraphs, so a `]`,
/// literal markup or a second paragraph survive. Pandoc's inline `^[…]`
/// form is not used: it is single-paragraph and a `]` ends it early
/// (plan-3 R3-1).
fn footnotes_to_references(
    section: &str,
    counter: &mut usize,
    definitions: &mut Vec<String>,
) -> String {
    let mut out = String::with_capacity(section.len());
    let mut rest = section;
    loop {
        let Some(start) = rest.find("<sup") else {
            out.push_str(rest);
            break;
        };
        let Some(tag_end_rel) = tag_end(&rest[start..]) else {
            out.push_str(rest);
            break;
        };
        let tag = &rest[start..start + tag_end_rel + 1];
        let is_footnote = tag.contains("class=\"footnote\"") || tag.contains("class='footnote'");
        let body = is_footnote.then(|| footnote_body_attribute(tag)).flatten();
        let Some(body) = body else {
            // Not a footnote element: copy the tag through and go on.
            out.push_str(&rest[..start + tag_end_rel + 1]);
            rest = &rest[start + tag_end_rel + 1..];
            continue;
        };
        let after_tag = &rest[start + tag_end_rel + 1..];
        let Some(close) = after_tag.find("</sup>") else {
            out.push_str(rest);
            break;
        };
        *counter += 1;
        let label = format!("cs-fn{}", *counter);
        out.push_str(&rest[..start]);
        out.push_str(&format!("[^{label}]"));
        definitions.push(footnote_definition(&label, &body));
        rest = &after_tag[close + "</sup>".len()..];
    }
    out
}

/// Byte index of the `>` that closes the tag starting at `s`, ignoring any
/// `>` inside a quoted attribute value (review finding s4-7: a footnote body
/// `x > y` ended the tag early and the footnote vanished from export).
fn tag_end(s: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    for (i, c) in s.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"') | (None, '\'') => quote = Some(c),
            (None, '>') => return Some(i),
            _ => {}
        }
    }
    None
}

/// The `data-body` attribute of a footnote tag, HTML-unescaped.
fn footnote_body_attribute(tag: &str) -> Option<String> {
    let idx = tag.find("data-body=")?;
    let after = &tag[idx + "data-body=".len()..];
    let quote = after.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let inner = &after[1..];
    let end = inner.find(quote)?;
    let raw = &inner[..end];
    Some(
        raw.replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&#10;", "\n")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&"),
    )
}

/// `[^label]: body` with the body as literal Markdown text; a blank line in
/// the body starts a new paragraph, indented so Pandoc keeps it in the note.
fn footnote_definition(label: &str, body: &str) -> String {
    let paragraphs: Vec<String> = body
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .map(|p| {
            p.lines()
                .map(|l| escape_markdown_literal(l.trim()))
                .collect::<Vec<_>>()
                .join("\n    ")
        })
        .collect();
    let mut def = format!("[^{label}]: ");
    if paragraphs.is_empty() {
        def.push_str("\\ ");
        return def;
    }
    def.push_str(&paragraphs[0]);
    for p in &paragraphs[1..] {
        def.push_str("\n\n    ");
        def.push_str(p);
    }
    def
}

/// Backslash-escapes every ASCII punctuation character, which Pandoc
/// Markdown reads back as that character.
fn escape_markdown_literal(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    for c in text.chars() {
        if c.is_ascii_punctuation() {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn convert(section: &str) -> (String, Vec<String>) {
        let mut counter = 0;
        let mut defs = Vec::new();
        let out = footnotes_to_references(section, &mut counter, &mut defs);
        (out, defs)
    }

    #[test]
    fn footnote_elements_become_reference_footnotes() {
        let (out, defs) = convert(
            "Text<sup class=\"footnote\" data-body=\"A] B\">●</sup> more<sup class=\"footnote\" data-body=\"&lt;redacted&gt; *x*\">●</sup>.",
        );
        assert_eq!(out, "Text[^cs-fn1] more[^cs-fn2].");
        assert_eq!(defs[0], "[^cs-fn1]: A\\] B");
        assert_eq!(defs[1], "[^cs-fn2]: \\<redacted\\> \\*x\\*");
    }

    #[test]
    fn two_paragraph_bodies_stay_two_paragraphs() {
        let (out, defs) = convert(
            "T<sup class=\"footnote\" data-body=\"First para.&#10;&#10;Second, with a ] bracket.\">●</sup>",
        );
        assert_eq!(out, "T[^cs-fn1]");
        assert_eq!(
            defs[0],
            "[^cs-fn1]: First para\\.\n\n    Second\\, with a \\] bracket\\."
        );
    }

    #[test]
    fn numbering_continues_across_sections_and_other_sup_tags_pass() {
        let mut counter = 0;
        let mut defs = Vec::new();
        let a = footnotes_to_references(
            "a<sup class=\"footnote\" data-body=\"one\">●</sup>",
            &mut counter,
            &mut defs,
        );
        let b = footnotes_to_references(
            "b<sup>2</sup> c<sup class=\"footnote\" data-body=\"two\">●</sup>",
            &mut counter,
            &mut defs,
        );
        assert_eq!(a, "a[^cs-fn1]");
        assert_eq!(b, "b<sup>2</sup> c[^cs-fn2]");
        assert_eq!(defs.len(), 2);
    }

    /// Review finding s4-7: a `>` inside the quoted body does not end the tag.
    #[test]
    fn a_quoted_greater_than_in_the_body_keeps_the_footnote() {
        let (out, defs) = convert("T<sup class=\"footnote\" data-body=\"x > y\">●</sup> end");
        assert_eq!(out, "T[^cs-fn1] end");
        assert_eq!(defs, vec!["[^cs-fn1]: x \\> y".to_string()]);
        let (out, defs) = convert("T<sup class='footnote' data-body='a > b'>●</sup>");
        assert_eq!(out, "T[^cs-fn1]");
        assert_eq!(defs, vec!["[^cs-fn1]: a \\> b".to_string()]);
    }

    #[test]
    fn sections_without_footnotes_are_unchanged() {
        let src = "Plain *text* with <span class=\"comment\" data-comment-id=\"x\">a note</span>.";
        let (out, defs) = convert(src);
        assert_eq!(out, src);
        assert!(defs.is_empty());
    }
}
