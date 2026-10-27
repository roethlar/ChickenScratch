//! Thin Tauri wrappers over `chickenscratch_services::io` — transport only.
use chickenscratch_services::io as svc;
#[allow(unused_imports)]
use chickenscratch_services::io::*;
use chickenscratch_services::ChiknError;
use chickenscratch_services::Project;
use chickenscratch_services::{ProjectTokens, ProjectWriteLocks};
use tauri::State;

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
) -> Result<Vec<String>, ChiknError> {
    svc::compile_project(
        project_path,
        output_path,
        format,
        title,
        author,
        section_separator,
        include_title_page,
        manuscript_format,
    )
}

#[tauri::command]
pub fn get_compile_formats() -> Vec<(String, String)> {
    svc::get_compile_formats()
}

#[tauri::command]
pub fn import_file(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    file_path: String,
    parent_id: Option<String>,
) -> Result<Project, ChiknError> {
    svc::import_file(project_path, &write_locks, &tokens, file_path, parent_id)
}

#[tauri::command]
pub fn import_markdown_folder(
    folder_path: String,
    output_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
) -> Result<Project, ChiknError> {
    svc::import_markdown_folder(folder_path, output_path, &write_locks, &tokens)
}

#[tauri::command]
pub fn get_writing_history(project_path: String) -> Result<WritingHistory, ChiknError> {
    svc::get_writing_history(project_path)
}

#[tauri::command]
pub fn record_daily_words(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    words: usize,
) -> Result<(), ChiknError> {
    svc::record_daily_words(project_path, &write_locks, &tokens, words)
}

#[tauri::command]
pub fn get_session_progress(project_path: String) -> Result<SessionProgress, ChiknError> {
    svc::get_session_progress(project_path)
}

#[tauri::command]
pub fn get_project_stats(project_path: String) -> Result<ProjectStats, ChiknError> {
    svc::get_project_stats(project_path)
}
