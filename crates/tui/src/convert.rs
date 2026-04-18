//! HTML ↔ Markdown conversion for TUI editing.
//!
//! The .chikn format stores HTML. The TUI edits markdown.
//! We convert on load and save.

use pulldown_cmark::{html, Parser, Options};

/// Convert HTML document content to markdown for editing.
pub fn html_to_markdown(html_str: &str) -> String {
    if html_str.trim().is_empty() {
        return String::new();
    }
    html2md::parse_html(html_str)
}

/// Convert markdown back to HTML for storage.
pub fn markdown_to_html(md: &str) -> String {
    if md.trim().is_empty() {
        return String::new();
    }
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(md, options);
    let mut output = String::new();
    html::push_html(&mut output, parser);
    output
}

/// Count words in markdown (strips markers).
pub fn count_words(md: &str) -> usize {
    md.split_whitespace()
        .filter(|w| {
            // Skip pure markdown punctuation
            !w.chars().all(|c| matches!(c, '#' | '*' | '-' | '_' | '`' | '>' | '='))
        })
        .count()
}
