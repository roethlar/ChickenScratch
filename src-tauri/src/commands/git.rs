use chickenscratch_core::core::git;
use chickenscratch_core::ChiknError;
use std::path::Path;

#[tauri::command]
pub fn save_revision(project_path: String, message: String) -> Result<git::Revision, ChiknError> {
    let path = Path::new(&project_path);
    let rev = git::save_revision(path, &message)?;

    // After named revision: push to backup remote if configured. Fire-and-forget:
    // a failed push should not fail the revision.
    let settings = super::settings::get_app_settings();
    if let Some(ref backup_dir) = settings.backup.backup_directory {
        let _ = git::push_backup(path, Path::new(backup_dir));
    }

    Ok(rev)
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
pub fn revision_diff(
    project_path: String,
    commit_id: String,
) -> Result<Vec<git::FileDiff>, ChiknError> {
    git::revision_diff(Path::new(&project_path), &commit_id)
}

#[tauri::command]
pub fn word_diff(
    project_path: String,
    commit_id: String,
    doc_path: String,
) -> Result<Vec<(String, String)>, ChiknError> {
    git::word_diff(Path::new(&project_path), &commit_id, &doc_path)
}

#[tauri::command]
pub fn has_changes(project_path: String) -> Result<bool, ChiknError> {
    git::has_changes(Path::new(&project_path))
}

/// Auto-backup: save any unsaved changes and push to backup remote if configured.
/// Called on app close. Non-fatal — failures are logged but don't block exit.
#[tauri::command]
pub fn backup_on_close(project_path: String) -> Result<(), ChiknError> {
    let path = Path::new(&project_path);
    let settings = super::settings::get_app_settings();

    // Auto-commit any uncommitted changes
    if git::has_changes(path).unwrap_or(false) {
        let _ = git::save_revision(path, "Auto-save on close");
    }

    // Push to backup remote if configured
    if settings.backup.auto_backup_on_close {
        if let Some(ref backup_dir) = settings.backup.backup_directory {
            let _ = git::push_backup(path, Path::new(backup_dir));
        }
    }

    Ok(())
}
