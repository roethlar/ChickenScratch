//! Compile: merge manuscript documents into a single output file.
//!
//! Walks the hierarchy in order, concatenates markdown content,
//! then uses Pandoc to convert to the target format.

use crate::core::project::reader;
use crate::models::{Project, TreeNode};
use crate::utils::error::ChiknError;
use crate::utils::process::{output_bounded, PANDOC_OUTPUT_LIMIT_BYTES, PANDOC_TIMEOUT};
use pulldown_cmark::{Event, Parser, Tag};
use std::fs;
use std::ops::Range;
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
    // Code is left alone (review finding s4-8: a footnote-shaped example in
    // a fenced block was rewritten). Code spans and code blocks are found
    // by a CommonMark parser with source offsets rather than by a line
    // scanner, which the re-review showed getting four things wrong: a
    // backtick inside the footnote's own quoted body was read as a code
    // span, a body spanning lines was cut at the line end, a fence inside a
    // blockquote was missed, and a one-line ```x``` code span was read as an
    // unclosed fence that swallowed the rest of the section. Raw HTML, the
    // footnote element included, is prose here; an HTML tag outranks the
    // backticks inside it, as in CommonMark.
    let mut protected = protected_code_ranges(section);
    protected.sort_by_key(|range| range.start);
    let mut out = String::with_capacity(section.len());
    let mut last = 0;
    for range in protected {
        if range.start < last {
            // Nested in a range already copied through.
            continue;
        }
        out.push_str(&footnotes_in_prose(
            &section[last..range.start],
            counter,
            definitions,
        ));
        out.push_str(&section[range.clone()]);
        last = range.end;
    }
    out.push_str(&footnotes_in_prose(&section[last..], counter, definitions));
    out
}

/// Appends the byte ranges of `text` that Pandoc's Markdown reader treats as
/// code, shifted by `base`: CommonMark code spans and code blocks, plus
/// what lies inside HTML blocks. CommonMark leaves an HTML block raw, but
/// Pandoc (`markdown_in_html_blocks`, on by default) reads the Markdown
/// between its tags, code included, so those stretches are searched again;
/// a `<pre>`, `<script>`, `<style>`, `<textarea>` block or a comment is
/// verbatim to Pandoc and is protected whole (review finding s4-8, second
/// re-review: a code span inside a `<div>` was rewritten on export).
fn protected_code_ranges(section: &str) -> Vec<Range<usize>> {
    let mut protected: Vec<Range<usize>> = Vec::new();
    // pulldown-cmark reports an HTML block one line at a time, and inside a
    // blockquote the `> ` prefixes sit between those lines; the lines are
    // joined back into one block (prefixes included) before it is read.
    let mut html_block: Option<Range<usize>> = None;
    let flush = |block: &mut Option<Range<usize>>, protected: &mut Vec<Range<usize>>| {
        if let Some(range) = block.take() {
            html_block_code_ranges(section, range, protected);
        }
    };
    for (event, range) in Parser::new(section).into_offset_iter() {
        match event {
            Event::Html(_) => match &mut html_block {
                Some(block)
                    if section[block.end..range.start]
                        .bytes()
                        .all(|b| matches!(b, b' ' | b'\t' | b'>')) =>
                {
                    block.end = range.end
                }
                _ => {
                    flush(&mut html_block, &mut protected);
                    html_block = Some(range);
                }
            },
            Event::Code(_) | Event::Start(Tag::CodeBlock(_)) => {
                flush(&mut html_block, &mut protected);
                protected.push(range);
            }
            _ => flush(&mut html_block, &mut protected),
        }
    }
    flush(&mut html_block, &mut protected);
    protected
}

/// The code ranges inside one HTML block of `section` (see
/// `protected_code_ranges`). The block's tags are raw and everything between
/// them is Markdown, so the tags are blanked out in a copy of the same
/// length and the copy is read as Markdown: offsets carry over one to one,
/// a blockquote's prefixes keep working, and no recursion is needed (the
/// third re-review found a run of 2000 sibling elements overflowing the
/// stack). A `<pre>`, `<script>`, `<style>` or `<textarea>` element and a
/// comment are verbatim to Pandoc: protected to their own closing delimiter
/// only, so prose after them on the same line is still prose.
fn html_block_code_ranges(section: &str, range: Range<usize>, protected: &mut Vec<Range<usize>>) {
    // Only a block-level tag at the start of a line is structure here, as in
    // Pandoc; a tag inside a line (an inline `<sup>`, a literal `1 < 2`, a
    // `` `<pre>` `` in a code span) is inline content for the Markdown parse
    // to judge (fourth re-review). The filler is a letter, not a space, so a
    // masked tag never turns the rest of its line into indented code.
    const FILLER: u8 = b'x';
    let block = &section[range.clone()];
    let bytes = block.as_bytes();
    let n = bytes.len();
    let mut masked = bytes.to_vec();
    let fill = |masked: &mut Vec<u8>, from: usize, to: usize| {
        for b in &mut masked[from..to] {
            if *b != b'\n' && *b != b'\r' {
                *b = FILLER;
            }
        }
    };
    let mut i = 0;
    while i < n {
        let line_end = block[i..].find('\n').map_or(n, |p| i + p + 1);
        // Quote prefixes and indentation before the line's first token.
        let mut j = i;
        while j < n && matches!(bytes[j], b' ' | b'\t' | b'>') {
            j += 1;
        }
        let rest = &block[j..];
        if rest.starts_with("<!--") {
            let end = rest.find("-->").map_or(n, |p| j + p + 3);
            protected.push(range.start + j..range.start + end);
            fill(&mut masked, j, end);
            i = end;
            continue;
        }
        if let Some((name, closing, tag_len)) = block_level_tag(rest) {
            if !closing && VERBATIM_ELEMENTS.contains(&name.as_str()) {
                let lower = rest.to_ascii_lowercase();
                let end = lower
                    .find(&format!("</{name}"))
                    .and_then(|p| tag_end(&rest[p..]).map(|t| j + p + t + 1))
                    .unwrap_or(n);
                protected.push(range.start + j..range.start + end);
                fill(&mut masked, j, end);
                i = end;
                continue;
            }
            fill(&mut masked, j, j + tag_len);
            // Another tag may follow on the same line (`</div></div>`).
            i = j + tag_len;
            continue;
        }
        i = line_end;
    }
    let masked = String::from_utf8(masked).unwrap_or_default();
    for (event, inner) in Parser::new(&masked).into_offset_iter() {
        if matches!(event, Event::Code(_) | Event::Start(Tag::CodeBlock(_))) {
            protected.push(range.start + inner.start..range.start + inner.end);
        }
    }
}

/// Elements Pandoc keeps verbatim inside an HTML block.
const VERBATIM_ELEMENTS: [&str; 4] = ["pre", "script", "style", "textarea"];

/// Elements whose tags open or close an HTML block (CommonMark's list, plus
/// the verbatim ones).
const BLOCK_ELEMENTS: [&str; 66] = [
    "address",
    "article",
    "aside",
    "base",
    "basefont",
    "blockquote",
    "body",
    "caption",
    "center",
    "col",
    "colgroup",
    "dd",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "frame",
    "frameset",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hr",
    "html",
    "iframe",
    "legend",
    "li",
    "link",
    "main",
    "menu",
    "menuitem",
    "nav",
    "noframes",
    "ol",
    "optgroup",
    "option",
    "p",
    "param",
    "pre",
    "script",
    "search",
    "section",
    "style",
    "summary",
    "table",
    "tbody",
    "td",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "track",
    "ul",
];

/// A block-level tag at the start of `rest`: its lower-case name, whether it
/// is a closing tag, and the tag's length in bytes.
fn block_level_tag(rest: &str) -> Option<(String, bool, usize)> {
    let after_lt = rest.strip_prefix('<')?;
    let (closing, after_slash) = match after_lt.strip_prefix('/') {
        Some(s) => (true, s),
        None => (false, after_lt),
    };
    let name: String = after_slash
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    if name.is_empty() || !BLOCK_ELEMENTS.contains(&name.as_str()) {
        return None;
    }
    // The name must end the tag or be followed by whitespace or `/`.
    let after_name = &after_slash[name.len()..];
    if !matches!(
        after_name.chars().next(),
        Some(' ' | '\t' | '\n' | '\r' | '>' | '/')
    ) {
        return None;
    }
    let close = tag_end(rest)?;
    Some((name, closing, close + 1))
}

/// Replaces every footnote element in `section` (prose only) with its
/// reference.
fn footnotes_in_prose(section: &str, counter: &mut usize, definitions: &mut Vec<String>) -> String {
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

    /// Review finding s4-8, re-review: code is recognised the way CommonMark
    /// does, not by a line scanner. Each case here failed with the scanner.
    #[test]
    fn code_detection_follows_commonmark() {
        let sup = |body: &str| format!("<sup class=\"footnote\" data-body=\"{body}\">●</sup>");

        // (1) A backtick inside the footnote's own body is not a code span:
        // the HTML tag outranks it.
        let (out, defs) = convert(&format!("T{} end", sup("Use `code` here")));
        assert_eq!(out, "T[^cs-fn1] end");
        assert_eq!(defs, vec!["[^cs-fn1]: Use \\`code\\` here".to_string()]);
        // Escaped backticks around a footnote are text, not a span.
        let (out, defs) = convert(&format!("\\`{}\\`", sup("x")));
        assert_eq!(out, "\\`[^cs-fn1]\\`");
        assert_eq!(defs.len(), 1);

        // (2) A body with a literal newline still converts; a code span that
        // crosses a line end still protects what is inside it.
        let (out, defs) = convert(&format!("T{} end", sup("first\nsecond")));
        assert_eq!(out, "T[^cs-fn1] end");
        // (the definition indents its continuation line, as footnotes need)
        assert_eq!(defs, vec!["[^cs-fn1]: first\n    second".to_string()]);
        let spanned = format!("`a\n{}` after", sup("x"));
        let (out, defs) = convert(&spanned);
        assert_eq!(out, spanned);
        assert!(defs.is_empty());

        // (3) A fence inside a blockquote is still a fence.
        let quoted = format!("> ```\n> {}\n> ```\n", sup("x"));
        let (out, defs) = convert(&quoted);
        assert_eq!(out, quoted);
        assert!(defs.is_empty());

        // (4) A one-line ```x``` is a code span, not an open fence: the
        // footnotes after it still convert.
        let (out, defs) = convert(&format!("```example```\n\nT{}\n", sup("one")));
        assert_eq!(out, "```example```\n\nT[^cs-fn1]\n");
        assert_eq!(defs.len(), 1);
    }

    /// Review finding s4-8, second re-review: Pandoc reads Markdown inside an
    /// HTML block, so code there is code; `<pre>` and comments are verbatim.
    #[test]
    fn code_inside_html_blocks_is_left_alone() {
        let sup = |body: &str| format!("<sup class=\"footnote\" data-body=\"{body}\">●</sup>");

        let inline = format!("<div>\nUse `{}` literally.\n</div>\n", sup("example"));
        let (out, defs) = convert(&inline);
        assert_eq!(out, inline);
        assert!(defs.is_empty());

        let fenced = format!("<div>\n```\n{}\n```\n</div>\n", sup("example"));
        let (out, defs) = convert(&fenced);
        assert_eq!(out, fenced);
        assert!(defs.is_empty());

        let pre = format!("<pre>\n{}\n</pre>\n", sup("example"));
        let (out, defs) = convert(&pre);
        assert_eq!(out, pre);
        assert!(defs.is_empty());

        let comment = format!("<!-- {} -->\n", sup("example"));
        let (out, defs) = convert(&comment);
        assert_eq!(out, comment);
        assert!(defs.is_empty());

        // A real footnote inside a wrapper still converts; a nested wrapper
        // keeps its code.
        let (out, defs) = convert(&format!("<div>\nT{} end\n</div>\n", sup("one")));
        assert_eq!(out, "<div>\nT[^cs-fn1] end\n</div>\n");
        assert_eq!(defs.len(), 1);
        let nested = format!(
            "<div>\n<div class=\"inner\">\nSee `{}` here, T{}.\n</div>\n</div>\n",
            sup("example"),
            sup("one")
        );
        let (out, defs) = convert(&nested);
        assert_eq!(
            out,
            format!(
                "<div>\n<div class=\"inner\">\nSee `{}` here, T[^cs-fn1].\n</div>\n</div>\n",
                sup("example")
            )
        );
        assert_eq!(defs.len(), 1);
    }

    /// Review finding s4-8, third re-review: HTML blocks are read without
    /// recursion (a long run of sibling elements overflowed the stack), a
    /// verbatim element or comment is protected only to its own end, and
    /// code inside a blockquoted wrapper is still code.
    #[test]
    fn html_blocks_are_read_flat_and_to_their_own_end() {
        let sup = |body: &str| format!("<sup class=\"footnote\" data-body=\"{body}\">●</sup>");

        // 2000 sibling elements: no recursion, no overflow, text unchanged.
        let siblings = "<div></div>\n".repeat(2000);
        let (out, defs) = convert(&siblings);
        assert_eq!(out, siblings);
        assert!(defs.is_empty());

        // Prose after a verbatim element or a comment on the same line is
        // still prose.
        let (out, defs) = convert(&format!("<pre>code</pre> Real{}\n", sup("note")));
        assert_eq!(out, "<pre>code</pre> Real[^cs-fn1]\n");
        assert_eq!(defs.len(), 1);
        let (out, defs) = convert(&format!("<!-- {} --> Real{}\n", sup("x"), sup("note")));
        assert_eq!(out, format!("<!-- {} --> Real[^cs-fn1]\n", sup("x")));
        assert_eq!(defs.len(), 1);

        // A wrapper inside a blockquote: the code span and the fence inside
        // it keep their examples.
        let quoted = format!(
            "> <div>\n> Use `{}` literally.\n>\n> ```\n> {}\n> ```\n> </div>\n",
            sup("example"),
            sup("example")
        );
        let (out, defs) = convert(&quoted);
        assert_eq!(out, quoted);
        assert!(defs.is_empty());
    }

    /// Review finding s4-8, fourth re-review: inside an HTML block only a
    /// block-level tag at the start of a line is structure; everything else
    /// on a line is inline content for the Markdown rules.
    #[test]
    fn html_block_content_lines_follow_inline_rules() {
        let sup = |body: &str| format!("<sup class=\"footnote\" data-body=\"{body}\">●</sup>");

        // (1) A real footnote on the wrapper's own line still converts: the
        // masked tag does not turn the line into indented code.
        let (out, defs) = convert(&format!("<div>Real{}</div>\n", sup("note")));
        assert_eq!(out, "<div>Real[^cs-fn1]</div>\n");
        assert_eq!(defs.len(), 1);

        // (2) A verbatim opener or a comment opener quoted in a code span is
        // code, not structure: the footnote after it still converts.
        let (out, defs) = convert(&format!(
            "<div>\nUse `<pre>` and `<!--` here, T{}.\n</div>\n",
            sup("note")
        ));
        assert_eq!(
            out,
            "<div>\nUse `<pre>` and `<!--` here, T[^cs-fn1].\n</div>\n"
        );
        assert_eq!(defs.len(), 1);

        // (3) A literal less-than sign after footnote-shaped markup inside a
        // code span does not break the span: the example stays.
        let example = format!("<div>\nUse `{} 1 < 2` here.\n</div>\n", sup("example"));
        let (out, defs) = convert(&example);
        assert_eq!(out, example);
        assert!(defs.is_empty());

        // A closing tag at the end of a content line and nested wrappers on
        // one line are fine too.
        let (out, defs) = convert(&format!(
            "<div><div class=\"a\">\nT{} end</div></div>\n",
            sup("note")
        ));
        assert_eq!(out, "<div><div class=\"a\">\nT[^cs-fn1] end</div></div>\n");
        assert_eq!(defs.len(), 1);
    }

    /// Review finding s4-8: code examples are not transformed.
    #[test]
    fn footnote_shaped_markup_inside_code_is_left_alone() {
        let fenced = "Real<sup class=\"footnote\" data-body=\"one\">●</sup>.\n\n```html\n<sup class=\"footnote\" data-body=\"example\">●</sup>\n```\n\nAfter.\n";
        let (out, defs) = convert(fenced);
        assert_eq!(
            out,
            "Real[^cs-fn1].\n\n```html\n<sup class=\"footnote\" data-body=\"example\">●</sup>\n```\n\nAfter.\n"
        );
        assert_eq!(defs.len(), 1);

        let tilde = "~~~\n<sup class=\"footnote\" data-body=\"x\">●</sup>\n~~~\n";
        let (out, defs) = convert(tilde);
        assert_eq!(out, tilde);
        assert!(defs.is_empty());

        let indented = "Text.\n\n    <sup class=\"footnote\" data-body=\"x\">●</sup>\n    more code\n\nTail<sup class=\"footnote\" data-body=\"two\">●</sup>\n";
        let (out, defs) = convert(indented);
        assert_eq!(
            out,
            "Text.\n\n    <sup class=\"footnote\" data-body=\"x\">●</sup>\n    more code\n\nTail[^cs-fn1]\n"
        );
        assert_eq!(defs.len(), 1);

        let inline = "Use `<sup class=\"footnote\" data-body=\"x\">●</sup>` like this<sup class=\"footnote\" data-body=\"two\">●</sup>, ``a ` b`` too.";
        let (out, defs) = convert(inline);
        assert_eq!(
            out,
            "Use `<sup class=\"footnote\" data-body=\"x\">●</sup>` like this[^cs-fn1], ``a ` b`` too."
        );
        assert_eq!(defs.len(), 1);

        // An unmatched backtick is ordinary text.
        let (out, _) = convert("a ` b<sup class=\"footnote\" data-body=\"n\">●</sup>");
        assert_eq!(out, "a ` b[^cs-fn1]");
    }

    #[test]
    fn sections_without_footnotes_are_unchanged() {
        let src = "Plain *text* with <span class=\"comment\" data-comment-id=\"x\">a note</span>.";
        let (out, defs) = convert(src);
        assert_eq!(out, src);
        assert!(defs.is_empty());
    }
}
