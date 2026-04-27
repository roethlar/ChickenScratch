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
        let plain = strip_comment_spans(&doc.content).to_lowercase();
        if plain.contains(&q) {
            let match_count = plain.matches(&q).count();

            // Extract snippet around first match
            let snippet = if let Some(pos) = plain.find(&q) {
                let start = pos.saturating_sub(40);
                let end = (pos + q.len() + 40).min(plain.len());
                // Find word boundaries
                let s = if start > 0 {
                    format!("...{}", &plain[start..end])
                } else {
                    plain[start..end].to_string()
                };
                if end < plain.len() {
                    format!("{}...", s)
                } else {
                    s
                }
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
