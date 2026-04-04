use chickenscratch_core::core::git;
use chickenscratch_core::ChiknError;
use std::path::Path;

#[tauri::command]
pub fn save_revision(
    project_path: String,
    message: String,
) -> Result<git::Revision, ChiknError> {
    git::save_revision(Path::new(&project_path), &message)
}

#[tauri::command]
pub fn list_revisions(project_path: String) -> Result<Vec<git::Revision>, ChiknError> {
    git::list_revisions(Path::new(&project_path))
}

#[tauri::command]
pub fn restore_revision(
    project_path: String,
    commit_id: String,
) -> Result<git::Revision, ChiknError> {
    git::restore_revision(Path::new(&project_path), &commit_id)
}

#[tauri::command]
pub fn create_draft(project_path: String, name: String) -> Result<(), ChiknError> {
    git::create_draft(Path::new(&project_path), &name)
}

#[tauri::command]
pub fn list_drafts(project_path: String) -> Result<Vec<git::DraftVersion>, ChiknError> {
    git::list_drafts(Path::new(&project_path))
}

#[tauri::command]
pub fn switch_draft(project_path: String, name: String) -> Result<(), ChiknError> {
    git::switch_draft(Path::new(&project_path), &name)
}

#[tauri::command]
pub fn merge_draft(project_path: String, name: String) -> Result<(), ChiknError> {
    git::merge_draft(Path::new(&project_path), &name)
}

#[tauri::command]
pub fn push_backup(project_path: String, backup_dir: String) -> Result<(), ChiknError> {
    git::push_backup(Path::new(&project_path), Path::new(&backup_dir))
}

#[tauri::command]
pub fn has_changes(project_path: String) -> Result<bool, ChiknError> {
    git::has_changes(Path::new(&project_path))
}
