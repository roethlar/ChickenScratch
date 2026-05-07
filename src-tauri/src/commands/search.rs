use chickenscratch_core::core::project::reader;
use chickenscratch_core::ChiknError;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub doc_id: String,
    pub doc_name: String,
    pub snippet: String,
    pub match_count: usize,
}

#[tauri::command]
pub fn search_project(
    project_path: String,
    query: String,
) -> Result<Vec<SearchResult>, ChiknError> {
    let project = reader::read_project(Path::new(&project_path))?;
    let q = query.to_lowercase();
    let mut results = Vec::new();

    for doc in project.documents.values() {
        let plain_lower = strip_comment_spans(&doc.content).to_lowercase();
        if plain_lower.contains(&q) {
            let match_count = plain_lower.matches(&q).count();

            // Build a 40-char window around the first match. The previous
            // implementation sliced by raw byte offsets (`pos.saturating_sub(40)`,
            // `pos + q.len() + 40`), which panics if the boundary lands inside
            // a multi-byte UTF-8 codepoint — and fiction full of curly quotes,
            // em dashes, and CJK content reliably triggers that. We work in
            // codepoints throughout so the slice can never split a character.
            // (F-010)
            let snippet = if let Some(pos) = plain_lower.find(&q) {
                snippet_around(&plain_lower, pos, q.chars().count(), 40)
            } else {
                String::new()
            };

            results.push(SearchResult {
                doc_id: doc.id.clone(),
                doc_name: doc.name.clone(),
                snippet,
                match_count,
            });
        }
    }

    // Sort by match count descending
    results.sort_by_key(|r| std::cmp::Reverse(r.match_count));
    Ok(results)
}

fn strip_comment_spans(html: &str) -> String {
    regex::Regex::new(r"<[^>]*>")
        .unwrap()
        .replace_all(html, "")
        .to_string()
}

/// Build a snippet of `padding` codepoints either side of a match starting at
/// `match_byte_pos` (a byte offset into `text`) and `match_chars` codepoints
/// long. The returned string has `…` markers when the window is truncated.
///
/// Operates in codepoints, so any byte boundaries we end up slicing at are
/// guaranteed valid UTF-8 boundaries — no panic on multi-byte content.
fn snippet_around(text: &str, match_byte_pos: usize, match_chars: usize, padding: usize) -> String {
    // Convert the byte position into a char index by counting chars before it.
    let match_char_index = text[..match_byte_pos].chars().count();
    let total_chars = text.chars().count();
    let start_char = match_char_index.saturating_sub(padding);
    let end_char = (match_char_index + match_chars + padding).min(total_chars);

    // Round back to byte indices using char_indices, which always yields
    // valid UTF-8 boundaries.
    let start_byte = text
        .char_indices()
        .nth(start_char)
        .map(|(i, _)| i)
        .unwrap_or(0);
    let end_byte = text
        .char_indices()
        .nth(end_char)
        .map(|(i, _)| i)
        .unwrap_or(text.len());

    let mut s = String::new();
    if start_char > 0 {
        s.push('…');
    }
    s.push_str(&text[start_byte..end_byte]);
    if end_char < total_chars {
        s.push('…');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippet_handles_curly_quotes_at_boundary() {
        // Build a string where char 39 is a 3-byte curly quote — the old
        // byte-slice math would land mid-codepoint and panic.
        let mut text = "x".repeat(39);
        text.push('“'); // U+201C, 3 bytes
        text.push_str(&"y".repeat(60));
        text.push_str("needle");
        text.push_str(&"z".repeat(60));
        let lower = text.to_lowercase();
        let pos = lower.find("needle").unwrap();
        let s = snippet_around(&lower, pos, "needle".chars().count(), 40);
        assert!(s.contains("needle"));
    }

    #[test]
    fn snippet_omits_leading_ellipsis_when_at_start() {
        let text = "needle in a haystack";
        let pos = text.find("needle").unwrap();
        let s = snippet_around(text, pos, "needle".chars().count(), 40);
        assert!(!s.starts_with('…'), "got {s:?}");
    }
}
