//! Thin Tauri wrappers over `chickenscratch_services::threads` — transport only.
use chickenscratch_services::threads as svc;
#[allow(unused_imports)]
use chickenscratch_services::threads::*;
use chickenscratch_services::ChiknError;
use chickenscratch_services::{Project, Thread};
use chickenscratch_services::{ProjectTokens, ProjectWriteLocks};
use tauri::State;

#[tauri::command]
pub fn validate_references(project_path: String) -> Result<Vec<DanglingRef>, ChiknError> {
    svc::validate_references(project_path)
}

#[tauri::command]
pub fn list_threads(project_path: String) -> Result<Vec<Thread>, ChiknError> {
    svc::list_threads(project_path)
}

#[tauri::command]
pub fn create_thread(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    name: String,
    color: Option<String>,
    description: Option<String>,
) -> Result<Project, ChiknError> {
    svc::create_thread(
        project_path,
        &write_locks,
        &tokens,
        name,
        color,
        description,
    )
}

#[tauri::command]
pub fn update_thread(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    id: String,
    name: Option<String>,
    color: Option<String>,
    description: Option<String>,
) -> Result<Project, ChiknError> {
    svc::update_thread(
        project_path,
        &write_locks,
        &tokens,
        id,
        name,
        color,
        description,
    )
}

#[tauri::command]
pub fn delete_thread(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    id: String,
) -> Result<Project, ChiknError> {
    svc::delete_thread(project_path, &write_locks, &tokens, id)
}
