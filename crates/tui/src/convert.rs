//! Markdown utilities for the TUI.
//!
//! Canonical storage is Pandoc Markdown. The TUI edits it directly — no
//! conversion on load or save. Only the preview view renders markdown to
//! a stylized terminal output (handled by ui.rs render_markdown_as_lines).

/// Count words in markdown, skipping pure markdown punctuation tokens.
pub fn count_words(md: &str) -> usize {
    md.split_whitespace()
        .filter(|w| !w.chars().all(|c| matches!(c, '#' | '*' | '-' | '_' | '`' | '>' | '=' | '|')))
        .count()
}
