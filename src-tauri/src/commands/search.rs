//! Thin Tauri wrappers over `chickenscratch_services::search` — transport only.
use chickenscratch_services::search as svc;
#[allow(unused_imports)]
use chickenscratch_services::search::*;
use chickenscratch_services::ChiknError;

#[tauri::command]
pub fn search_project(
    project_path: String,
    query: String,
) -> Result<Vec<SearchResult>, ChiknError> {
    svc::search_project(project_path, query)
}
