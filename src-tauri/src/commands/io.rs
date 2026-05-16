use chickenscratch_core::core::compile;
use chickenscratch_core::core::git;
use chickenscratch_core::core::project::hierarchy;
use chickenscratch_core::core::project::{reader, writer};
use chickenscratch_core::{ChiknError, Document, Project, TreeNode};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;
use tauri::State;

use super::ProjectWriteLocks;

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn compile_project(
    project_path: String,
    output_path: String,
    format: String,
    title: Option<String>,
    author: Option<String>,
    section_separator: Option<String>,
    include_title_page: Option<bool>,
    manuscript_format: Option<bool>,
) -> Result<(), ChiknError> {
    let settings = super::settings::get_app_settings();
    let ms_format = manuscript_format.unwrap_or(false);
    let options = compile::CompileOptions {
        font: Some(settings.compile.font),
        font_size: Some(settings.compile.font_size),
        line_spacing: Some(settings.compile.line_spacing),
        margin_inches: Some(settings.compile.margin_inches),
        section_separator,
        include_title_page: include_title_page.unwrap_or(false),
        manuscript_format: ms_format,
        pandoc_path: settings.general.pandoc_path.clone(),
    };

    compile::compile(
        Path::new(&project_path),
        Path::new(&output_path),
        &format,
        title.as_deref(),
        author.as_deref(),
        Some(options),
    )
}

#[tauri::command]
pub fn get_compile_formats() -> Vec<(String, String)> {
    compile::FORMATS
        .iter()
        .map(|(ext, desc)| (ext.to_string(), desc.to_string()))
        .collect()
}

/// File extensions that Pandoc can convert to HTML
const PANDOC_IMPORT_EXTENSIONS: &[&str] = &[
    "docx",
    "doc",
    "odt",
    "rtf",
    "epub",
    "latex",
    "tex",
    "md",
    "markdown",
    "rst",
    "org",
    "textile",
    "mediawiki",
    "html",
    "htm",
    "txt",
    "csv",
    "json",
    "fb2",
    "pptx",
    "xlsx",
];

/// Import a file into an existing project. Uses Pandoc for conversion when needed.
#[tauri::command]
pub fn import_file(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    file_path: String,
    parent_id: Option<String>,
) -> Result<Project, ChiknError> {
    let path = Path::new(&file_path);
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Imported")
        .to_string();

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Convert to markdown (canonical content format)
    let content = match ext.as_str() {
        "md" | "markdown" => fs::read_to_string(path)?,
        "txt" => fs::read_to_string(path)?, // plain text is valid markdown
        _ => {
            // Use Pandoc to convert to markdown
            convert_to_markdown_via_pandoc(path, &ext)?
        }
    };

    write_locks.with_project_lock(&project_path, || {
        let mut project = reader::read_project(Path::new(&project_path))?;

        let doc_id = uuid::Uuid::new_v4().to_string();
        let slug =
            chickenscratch_core::utils::slug::unique_slug(&name, "manuscript/", &project.documents);
        let doc_path = format!("manuscript/{}.md", slug);
        let now = chrono::Utc::now().to_rfc3339();

        let document = Document {
            id: doc_id.clone(),
            name: name.clone(),
            path: doc_path.clone(),
            content,
            parent_id: parent_id.clone(),
            created: now.clone(),
            modified: now,
            ..Default::default()
        };

        project.documents.insert(doc_id.clone(), document);

        let node = TreeNode::Document {
            id: doc_id,
            name,
            path: doc_path,
        };

        match parent_id {
            Some(pid) => hierarchy::add_child_to_folder(&mut project.hierarchy, &pid, node)?,
            None => hierarchy::add_document_to_hierarchy(&mut project.hierarchy, node),
        }

        writer::write_project(&mut project)?;
        Ok(project)
    })
}

fn convert_to_markdown_via_pandoc(file_path: &Path, ext: &str) -> Result<String, ChiknError> {
    // Map file extensions to Pandoc input format names
    let format = match ext {
        "docx" | "doc" => "docx",
        "odt" => "odt",
        "rtf" => "rtf",
        "epub" => "epub",
        "latex" | "tex" => "latex",
        "md" | "markdown" => "markdown",
        "rst" => "rst",
        "org" => "org",
        "textile" => "textile",
        "mediawiki" => "mediawiki",
        "fb2" => "fb2",
        "pptx" => "pptx",
        _ => "markdown", // fallback
    };

    // Try common Pandoc paths
    let pandoc = find_pandoc()?;

    let output = Command::new(&pandoc)
        .arg("-f")
        .arg(format)
        .arg("-t")
        .arg("markdown")
        .arg("--wrap=none")
        .arg(file_path)
        .output()
        .map_err(|e| ChiknError::Unknown(format!("Failed to run Pandoc: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ChiknError::Unknown(format!(
            "Pandoc conversion failed: {}",
            stderr
        )));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| ChiknError::Unknown(format!("Invalid UTF-8 from Pandoc: {}", e)))
}

fn find_pandoc() -> Result<String, ChiknError> {
    // Check settings for custom path
    let settings = super::settings::get_app_settings();
    if let Some(ref p) = settings.general.pandoc_path {
        if !p.is_empty() {
            return Ok(p.clone());
        }
    }

    #[cfg(target_os = "windows")]
    let candidates: &[&str] = &["pandoc", "pandoc.exe"];

    #[cfg(not(target_os = "windows"))]
    let candidates: &[&str] = &[
        "pandoc",
        "/usr/local/bin/pandoc",
        "/opt/homebrew/bin/pandoc",
        "/usr/bin/pandoc",
    ];

    for candidate in candidates {
        if Command::new(candidate)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(candidate.to_string());
        }
    }

    Err(ChiknError::Unknown(
        "Pandoc is not installed. Required for importing this file format.".to_string(),
    ))
}

/// Import a folder of files as a new project.
#[tauri::command]
pub fn import_markdown_folder(
    folder_path: String,
    output_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
) -> Result<Project, ChiknError> {
    let lock_path = output_path.clone();
    write_locks.with_project_lock(lock_path, || {
        import_markdown_folder_impl(folder_path, output_path)
    })
}

fn import_markdown_folder_impl(
    folder_path: String,
    output_path: String,
) -> Result<Project, ChiknError> {
    let folder = Path::new(&folder_path);
    let output = Path::new(&output_path);

    let name = folder
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Imported")
        .to_string();

    let mut project = writer::create_project(output, &name)?;

    let mut entries: Vec<_> = fs::read_dir(folder)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    // Per-file failures collected so we can either fail the whole import (if
    // every file failed) or surface a partial-import warning. Listing failed
    // paths is far more useful than the previous silent empty-document
    // outcome (F-015).
    let mut failures: Vec<String> = Vec::new();
    let mut imported_count: usize = 0;

    for entry in entries {
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Skip files we can't import
        if !PANDOC_IMPORT_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }

        let doc_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();

        // F-015: previously `unwrap_or_default()` quietly turned read /
        // pandoc failures into empty Document content — the project then
        // committed with a real-looking entry whose body was "" and the
        // user's original file silently lost. Skip the entry on failure
        // and collect the error so we can report a partial import at the
        // end of the loop instead of pretending success.
        let content = match ext.as_str() {
            "md" | "markdown" | "txt" => match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    failures.push(format!("{}: {}", path.display(), e));
                    continue;
                }
            },
            _ => match convert_to_markdown_via_pandoc(&path, &ext) {
                Ok(c) => c,
                Err(e) => {
                    failures.push(format!("{}: {}", path.display(), e));
                    continue;
                }
            },
        };

        let doc_id = uuid::Uuid::new_v4().to_string();
        let slug = chickenscratch_core::utils::slug::unique_slug(
            &doc_name,
            "manuscript/",
            &project.documents,
        );
        let doc_path = format!("manuscript/{}.md", slug);
        let now = chrono::Utc::now().to_rfc3339();

        let document = Document {
            id: doc_id.clone(),
            name: doc_name.clone(),
            path: doc_path.clone(),
            content,
            parent_id: None,
            created: now.clone(),
            modified: now,
            ..Default::default()
        };

        project.documents.insert(doc_id.clone(), document);
        project.hierarchy.push(TreeNode::Document {
            id: doc_id,
            name: doc_name,
            path: doc_path,
        });
        imported_count += 1;
    }

    // Fail the whole transaction when nothing imported AND we hit failures.
    // An empty source folder (no failures, no imports) is still a successful
    // — if useless — empty project, matching prior behavior for that case.
    if imported_count == 0 && !failures.is_empty() {
        return Err(ChiknError::Unknown(format!(
            "Import failed for all {} file(s):\n{}",
            failures.len(),
            failures.join("\n")
        )));
    }

    writer::write_project(&mut project)?;
    let _ = git::save_revision(output, &format!("Imported from: {}", name));

    // Partial success: log skipped files to stderr so the operator sees them
    // even though the import returned the project. A future API revision
    // could fold this into a structured `ImportResult { project, skipped }`,
    // but at this scale (and given Tauri's command return is JSON) the
    // current shape is fine; the alternative was silent data loss.
    if !failures.is_empty() {
        eprintln!(
            "Imported {} file(s); skipped {} that failed to read/convert:\n{}",
            imported_count,
            failures.len(),
            failures.join("\n")
        );
    }

    Ok(project)
}

#[derive(Debug, Clone, Serialize)]
pub struct DocStats {
    pub id: String,
    pub name: String,
    pub words: usize,
    pub include_in_compile: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectStats {
    pub total_words: usize,
    pub manuscript_words: usize,
    pub total_docs: usize,
    pub docs: Vec<DocStats>,
}

/// Count words in markdown content. Strips inline HTML tags defensively
/// (pandoc markdown allows raw HTML and we don't want tags counted as words).
fn count_words_md(md: &str) -> usize {
    let mut text = String::with_capacity(md.len());
    let mut in_tag = false;
    for ch in md.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                text.push(' ');
            }
            _ if !in_tag => text.push(ch),
            _ => {}
        }
    }
    // Also skip pure markdown punctuation tokens (#, *, -, etc.)
    text.split_whitespace()
        .filter(|w| {
            !w.chars()
                .all(|c| matches!(c, '#' | '*' | '-' | '_' | '`' | '>' | '=' | '|'))
        })
        .count()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WritingHistory {
    pub entries: Vec<DayEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayEntry {
    pub date: String, // YYYY-MM-DD
    pub words: usize,
    /// Total project word count at the *first* record_daily_words call today.
    /// Net words written today = words - start_words. Older entries from
    /// before the field existed deserialize as None and the session badge
    /// falls back to "0 today" for that day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_words: Option<usize>,
}

#[tauri::command]
pub fn get_writing_history(project_path: String) -> Result<WritingHistory, ChiknError> {
    let path = Path::new(&project_path)
        .join("settings")
        .join("writing-history.json");
    if !path.exists() {
        return Ok(WritingHistory::default());
    }
    let data = fs::read_to_string(&path)?;
    parse_writing_history(&path, &data)
}

fn parse_writing_history(path: &Path, data: &str) -> Result<WritingHistory, ChiknError> {
    serde_json::from_str(data).map_err(|e| {
        ChiknError::InvalidFormat(format!(
            "Failed to parse writing history at {}: {}",
            path.display(),
            e
        ))
    })
}

#[tauri::command]
pub fn record_daily_words(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    words: usize,
) -> Result<(), ChiknError> {
    let lock_path = project_path.clone();
    write_locks.with_project_lock(lock_path, || record_daily_words_impl(project_path, words))
}

fn record_daily_words_impl(project_path: String, words: usize) -> Result<(), ChiknError> {
    let dir = Path::new(&project_path).join("settings");
    fs::create_dir_all(&dir)?;
    let path = dir.join("writing-history.json");

    let mut history: WritingHistory = if path.exists() {
        let data = fs::read_to_string(&path)?;
        parse_writing_history(&path, &data)?
    } else {
        WritingHistory::default()
    };

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    if let Some(entry) = history.entries.iter_mut().find(|e| e.date == today) {
        if entry.start_words.is_none() {
            entry.start_words = Some(entry.words);
        }
        entry.words = words;
    } else {
        history.entries.push(DayEntry {
            date: today,
            words,
            start_words: Some(words),
        });
    }

    // Keep last 90 days
    if history.entries.len() > 90 {
        history.entries = history.entries.split_off(history.entries.len() - 90);
    }

    let json = serde_json::to_string_pretty(&history)
        .map_err(|e| ChiknError::Unknown(format!("Failed to serialize history: {}", e)))?;
    fs::write(&path, json)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionProgress {
    pub today_words: i64,
    pub words_per_session: Option<u32>,
    pub total_target: Option<u32>,
    pub deadline: Option<String>,
    pub days_remaining: Option<i64>,
    pub current_total: usize,
    /// `(total_target - current_total) / days_remaining`, rounded up.
    /// None when no target/deadline configured or deadline already passed.
    pub needed_per_day: Option<u32>,
}

#[tauri::command]
pub fn get_session_progress(project_path: String) -> Result<SessionProgress, ChiknError> {
    let project = reader::read_project(Path::new(&project_path))?;
    let target = project.metadata.session_target.clone().unwrap_or_default();

    // Current manuscript word count (only documents under manuscript/, like the badge expects).
    let current_total: usize = project
        .documents
        .values()
        .filter(|d| d.path.starts_with("manuscript/") && d.path.ends_with(".md"))
        .map(|d| count_words_md(&d.content))
        .sum();

    // Today's net words from writing-history.json
    let history_path = Path::new(&project_path)
        .join("settings")
        .join("writing-history.json");
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let today_words: i64 = if history_path.exists() {
        let data = fs::read_to_string(&history_path)?;
        let history = parse_writing_history(&history_path, &data)?;
        history
            .entries
            .iter()
            .find(|e| e.date == today)
            .and_then(|e| e.start_words.map(|sw| current_total as i64 - sw as i64))
            .unwrap_or(0)
    } else {
        0
    };

    let days_remaining = target.deadline.as_ref().and_then(|d| {
        chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d")
            .ok()
            .map(|deadline| {
                let now = chrono::Utc::now().date_naive();
                (deadline - now).num_days()
            })
    });

    let needed_per_day = match (target.total_target, days_remaining) {
        (Some(total), Some(days)) if days > 0 && (total as i64) > current_total as i64 => {
            let remaining = total as i64 - current_total as i64;
            Some(((remaining + days - 1) / days) as u32)
        }
        _ => None,
    };

    Ok(SessionProgress {
        today_words,
        words_per_session: target.words_per_session,
        total_target: target.total_target,
        deadline: target.deadline,
        days_remaining,
        current_total,
        needed_per_day,
    })
}

#[tauri::command]
pub fn get_project_stats(project_path: String) -> Result<ProjectStats, ChiknError> {
    let project = reader::read_project(Path::new(&project_path))?;
    let mut docs = Vec::new();
    let mut total_words = 0;
    let mut manuscript_words = 0;

    for doc in project.documents.values() {
        if !doc.path.ends_with(".md") {
            continue;
        }
        let words = count_words_md(&doc.content);
        total_words += words;
        if doc.path.starts_with("manuscript/") {
            manuscript_words += words;
        }
        docs.push(DocStats {
            id: doc.id.clone(),
            name: doc.name.clone(),
            words,
            include_in_compile: doc.include_in_compile,
        });
    }
    docs.sort_by_key(|d| std::cmp::Reverse(d.words));

    Ok(ProjectStats {
        total_words,
        manuscript_words,
        total_docs: docs.len(),
        docs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writing_history_parser_rejects_corrupt_json() {
        let path = Path::new("/tmp/writing-history.json");
        let result = parse_writing_history(path, "{\"entries\":[");

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
    }

    #[test]
    fn writing_history_parser_accepts_valid_json() {
        let path = Path::new("/tmp/writing-history.json");
        let history = parse_writing_history(
            path,
            r#"{"entries":[{"date":"2026-05-16","words":1200,"start_words":900}]}"#,
        )
        .unwrap();

        assert_eq!(history.entries.len(), 1);
        assert_eq!(history.entries[0].date, "2026-05-16");
        assert_eq!(history.entries[0].words, 1200);
        assert_eq!(history.entries[0].start_words, Some(900));
    }

    #[test]
    fn session_progress_rejects_corrupt_writing_history() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let project_path = temp_dir.path().join("CorruptHistory.chikn");
        chickenscratch_core::core::project::writer::create_project(
            &project_path,
            "Corrupt History",
        )
        .unwrap();

        let settings_path = project_path.join("settings");
        fs::create_dir_all(&settings_path).unwrap();
        fs::write(settings_path.join("writing-history.json"), "{\"entries\":[").unwrap();

        let result = get_session_progress(project_path.to_string_lossy().to_string());

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
    }
}
