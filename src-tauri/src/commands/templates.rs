//! Thin Tauri wrappers over `chickenscratch_services::templates` — transport only.
use chickenscratch_services::templates as svc;
#[allow(unused_imports)]
use chickenscratch_services::templates::*;
use chickenscratch_services::ChiknError;
use chickenscratch_services::Project;
use chickenscratch_services::{ProjectTokens, ProjectWriteLocks};
use tauri::State;

#[tauri::command]
pub fn list_templates() -> Vec<Template> {
    svc::list_templates()
}

#[tauri::command]
pub fn create_from_template(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    template_id: String,
    name: String,
    parent_id: Option<String>,
) -> Result<Project, ChiknError> {
    svc::create_from_template(
        project_path,
        &write_locks,
        &tokens,
        template_id,
        name,
        parent_id,
    )
}

#[tauri::command]
pub fn save_as_template(project_path: String, doc_id: String) -> Result<Template, ChiknError> {
    svc::save_as_template(project_path, doc_id)
}
