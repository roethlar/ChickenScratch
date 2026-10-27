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

/// The editor stores a footnote as `<sup class="footnote" data-body="…">●</sup>`
/// (`ui/src/components/editor/FootnoteNode.ts`). Pandoc never reads that
/// attribute, so until PLAN_SCRIVENER_FIDELITY slice 4 every footnote
/// vanished from compiled output (review finding plan-3 R2-1). This Pandoc
/// Lua filter turns the element into a footnote inside Pandoc's own parse,
/// so whatever Pandoc reads as code or a verbatim block is never touched and
/// whatever it reads as prose is converted (review finding s4-8: five rounds
/// of re-implementing Markdown's code rules on the Markdown text each left a
/// case behind). The body is plain text; a blank line in it starts a new
/// paragraph.
const FOOTNOTE_FILTER: &str = include_str!("cs-footnotes.lua");

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

    // Join sections with markdown separator. The editor's footnote elements
    // are converted by the Lua filter inside Pandoc, not here.
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
    let run_id = uuid::Uuid::new_v4();
    let temp_md = temp_dir.join(format!("compile_{run_id}.md"));
    let temp_filter = temp_dir.join(format!("compile_{run_id}.lua"));
    fs::write(&temp_md, &md)?;
    fs::write(&temp_filter, FOOTNOTE_FILTER)?;

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
    cmd.arg("--lua-filter").arg(&temp_filter);
    // Images are stored project-relative (`research/figure.png`, the
    // converter's convention). Pandoc resolves a relative image against its
    // resource path, which defaults to the working directory; the combined
    // manuscript lives in the system temp folder, so without this every
    // project image was "not fetched" and replaced by its description
    // (PLAN_SCRIVENER_UPLOAD slice 3, review finding plan-4 R3-12).
    cmd.arg("--resource-path").arg(project_path);

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
    let _ = fs::remove_file(&temp_filter);

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

/// The footnote filter is exercised through Pandoc itself: these tests run
/// `pandoc -t json --lua-filter` on Markdown and inspect the document Pandoc
/// hands back, so what counts as code or prose is Pandoc's own reading.
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::io::Write;
    use std::process::Stdio;

    const FILTER_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/core/cs-footnotes.lua");

    fn pandoc_present() -> bool {
        Command::new("pandoc")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn pandoc_with_filter(markdown: &str, to: &str) -> String {
        let mut child = Command::new("pandoc")
            .args(["-f", "markdown", "-t", to, "--lua-filter", FILTER_PATH])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("pandoc starts");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(markdown.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(
            out.status.success(),
            "pandoc failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    }

    fn ast(markdown: &str) -> Value {
        serde_json::from_str(&pandoc_with_filter(markdown, "json")).expect("pandoc json")
    }

    /// Every node of type `t`, in document order.
    fn nodes<'a>(value: &'a Value, t: &str, found: &mut Vec<&'a Value>) {
        match value {
            Value::Array(items) => items.iter().for_each(|item| nodes(item, t, found)),
            Value::Object(map) => {
                if map.get("t").and_then(Value::as_str) == Some(t) {
                    found.push(value);
                }
                // The document root holds `blocks`; a node holds `c`.
                map.values().for_each(|child| nodes(child, t, found));
            }
            _ => {}
        }
    }

    fn count(value: &Value, t: &str) -> usize {
        let mut found = Vec::new();
        nodes(value, t, &mut found);
        found.len()
    }

    /// The text of every Code, CodeBlock and RawBlock, in document order.
    fn code_texts(value: &Value) -> Vec<String> {
        let mut out = Vec::new();
        for t in ["Code", "CodeBlock", "RawBlock"] {
            let mut found = Vec::new();
            nodes(value, t, &mut found);
            for node in found {
                let c = &node["c"];
                out.push(c[1].as_str().unwrap_or("").to_string());
            }
        }
        out
    }

    /// Each Note's body as text: paragraphs joined by a blank line, words by
    /// spaces, soft breaks by newlines.
    fn note_bodies(value: &Value) -> Vec<String> {
        let mut found = Vec::new();
        nodes(value, "Note", &mut found);
        found
            .iter()
            .map(|note| {
                let blocks = note["c"].as_array().cloned().unwrap_or_default();
                blocks
                    .iter()
                    .map(|block| {
                        let inlines = block["c"].as_array().cloned().unwrap_or_default();
                        inlines
                            .iter()
                            .map(|inline| match inline["t"].as_str() {
                                Some("Str") => inline["c"].as_str().unwrap_or("").to_string(),
                                Some("Space") => " ".to_string(),
                                Some("SoftBreak") => "\n".to_string(),
                                other => format!("<{other:?}>"),
                            })
                            .collect::<String>()
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n")
            })
            .collect()
    }

    /// RawInline nodes that still carry the footnote element's opening tag.
    fn unconverted(value: &Value) -> usize {
        let mut found = Vec::new();
        nodes(value, "RawInline", &mut found);
        found
            .iter()
            .filter(|node| {
                node["c"][1]
                    .as_str()
                    .is_some_and(|s| s.contains("class=\"footnote\""))
            })
            .count()
    }

    fn sup(body: &str) -> String {
        format!("<sup class=\"footnote\" data-body=\"{body}\">●</sup>")
    }

    #[test]
    fn footnote_elements_become_notes_with_literal_bodies() {
        if !pandoc_present() {
            eprintln!("Skipping test: Pandoc not installed");
            return;
        }
        let doc = ast(&format!(
            "Text{} more{}.",
            sup("A] B"),
            sup("&lt;redacted&gt; *x* and `code`")
        ));
        assert_eq!(count(&doc, "Note"), 2);
        assert_eq!(unconverted(&doc), 0);
        assert_eq!(
            note_bodies(&doc),
            vec!["A] B".to_string(), "<redacted> *x* and `code`".to_string()]
        );
        // The body is plain text: no Emph, no Code inside the note.
        assert_eq!(count(&doc, "Emph"), 0);
        assert_eq!(count(&doc, "Code"), 0);

        // And the HTML export carries the footnotes.
        let html = pandoc_with_filter(&format!("Text{} end.", sup("one")), "html");
        assert!(html.contains("footnote-ref"), "{html}");
        assert!(html.contains("one"), "{html}");
    }

    #[test]
    fn two_paragraph_bodies_stay_two_paragraphs() {
        if !pandoc_present() {
            eprintln!("Skipping test: Pandoc not installed");
            return;
        }
        let doc = ast(&format!(
            "T{}",
            sup("First para.&#10;&#10;Second, with a ] bracket.&#10;And a line.")
        ));
        let notes: Vec<&Value> = {
            let mut found = Vec::new();
            nodes(&doc, "Note", &mut found);
            found
        };
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0]["c"].as_array().unwrap().len(), 2, "two paragraphs");
        assert_eq!(
            note_bodies(&doc),
            vec!["First para.\n\nSecond, with a ] bracket.\nAnd a line.".to_string()]
        );
    }

    /// Review finding s4-7: a `>` inside the quoted body does not end the tag.
    #[test]
    fn a_quoted_greater_than_in_the_body_keeps_the_footnote() {
        if !pandoc_present() {
            eprintln!("Skipping test: Pandoc not installed");
            return;
        }
        let doc = ast(&format!(
            "T{} and U<sup class='footnote' data-body='a > b'>●</sup> end",
            sup("x &gt; y")
        ));
        assert_eq!(count(&doc, "Note"), 2);
        assert_eq!(unconverted(&doc), 0);
        assert_eq!(
            note_bodies(&doc),
            vec!["x > y".to_string(), "a > b".to_string()]
        );
    }

    /// Review finding s4-8, fifth re-review: a paragraph of many unclosed
    /// openers is scanned once, not once per opener.
    #[test]
    fn many_unclosed_elements_are_handled_in_one_pass() {
        if !pandoc_present() {
            eprintln!("Skipping test: Pandoc not installed");
            return;
        }
        let md = "<sup class=\"footnote\" data-body=\"x\">x ".repeat(10_000);
        let started = std::time::Instant::now();
        let doc = ast(&md);
        let elapsed = started.elapsed();
        assert_eq!(count(&doc, "Note"), 0);
        assert_eq!(unconverted(&doc), 10_000, "every opener kept as raw HTML");
        assert!(
            elapsed.as_secs() < 30,
            "10,000 unclosed openers took {elapsed:?}; the scan must be linear"
        );
    }

    #[test]
    fn other_sup_tags_and_unclosed_elements_are_left_alone() {
        if !pandoc_present() {
            eprintln!("Skipping test: Pandoc not installed");
            return;
        }
        let doc = ast("b<sup>2</sup> c<sup class=\"footnote\" data-body=\"x\">● never closed");
        assert_eq!(count(&doc, "Note"), 0);
        assert_eq!(count(&doc, "RawInline"), 3);
        assert_eq!(unconverted(&doc), 1);
    }

    /// Review finding s4-8 (five rounds): whatever Pandoc reads as code or a
    /// verbatim block keeps its footnote-shaped markup; whatever it reads as
    /// prose, wherever it sits, is converted.
    #[test]
    fn notes_follow_pandocs_own_reading_of_code_and_prose() {
        if !pandoc_present() {
            eprintln!("Skipping test: Pandoc not installed");
            return;
        }
        let s = sup("example");
        let real = |n: usize| sup(&format!("note {n}"));
        let md = format!(
            "Real{r1}.\n\n\
             ```html\n{s}\n```\n\n\
             ~~~\n{s}\n~~~\n\n\
             Text.\n\n    {s}\n    more code\n\nTail{r2}\n\n\
             Inline `{s}` and `{s} 1 < 2` and ```x``` then{r3}.\n\n\
             <div>\nUse `{s}` literally, Real{r4}.\n</div>\n\n\
             <div>\n```\n{s}\n```\n</div>\n\n\
             <div>\n    {s}\n</div>\n\n\
             <pre>{s}</pre> Real{r5}\n\n\
             <!-- {s} --> Real{r6}\n\n\
             > <div>\n> Use `{s}` quoted.\n>\n> ```\n> {s}\n> ```\n> </div>\n\n\
             <div>\n- <div>Use `{s}` in a list.</div>\n</div>\n\n\
             <div>Same{r7}</div>\n\n\
             <div>\nText <pre>{s}</pre> after{r8}\n</div>\n\n\
             <div>\nUse `one\n<pre>\ntwo` and Real{r9}\n</div>\n\n\
             With {r10} in it.\n",
            r1 = real(1),
            r2 = real(2),
            r3 = real(3),
            r4 = real(4),
            r5 = real(5),
            r6 = real(6),
            r7 = real(7),
            r8 = real(8),
            r9 = real(9),
            r10 = sup("Use `code` here"),
        );
        let doc = ast(&md);
        assert_eq!(count(&doc, "Note"), 10, "every real footnote converts");
        assert_eq!(unconverted(&doc), 0, "none left as raw HTML in prose");
        let bodies = note_bodies(&doc);
        assert_eq!(bodies[0], "note 1");
        assert_eq!(bodies[9], "Use `code` here");
        // Every code span, code block and verbatim block still holds its
        // example, untouched.
        let in_code: usize = code_texts(&doc)
            .iter()
            .map(|text| text.matches("<sup class=\"footnote\"").count())
            .sum();
        assert_eq!(in_code, 14, "examples inside code are left alone");
    }
}
