use chickenscratch_core::core::git;
use chickenscratch_core::ChiknError;
use std::path::Path;

#[tauri::command]
pub fn save_revision(project_path: String, message: String) -> Result<git::Revision, ChiknError> {
    let path = Path::new(&project_path);
    let rev = git::save_revision(path, &message)?;

    // After named revision: push to backup remote and remote-sync if configured.
    // Both are fire-and-forget — a failed push must not fail the revision.
    let settings = super::settings::get_app_settings();
    if let Some(ref backup_dir) = settings.backup.backup_directory {
        let _ = git::push_backup(path, Path::new(backup_dir));
    }
    if settings.remote.auto_push_on_revision {
        if let Some(ref url) = settings.remote.url {
            let auth = git::RemoteAuth {
                username: settings.remote.username.clone(),
                token: settings.remote.token.clone(),
            };
            let _ = git::push_remote(path, url, &auth);
        }
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

/// Push the current branch to the remote URL configured in settings.
#[tauri::command]
pub fn sync_push(project_path: String) -> Result<(), ChiknError> {
    let (url, auth) = remote_from_settings()?;
    git::push_remote(Path::new(&project_path), &url, &auth)
}

/// Fetch the current branch from the configured remote. Does not merge.
#[tauri::command]
pub fn sync_fetch(project_path: String) -> Result<(), ChiknError> {
    let (url, auth) = remote_from_settings()?;
    git::fetch_remote(Path::new(&project_path), &url, &auth)
}

/// Ahead/behind counts relative to the last-fetched remote tracking ref.
#[tauri::command]
pub fn sync_status(project_path: String) -> Result<git::SyncStatus, ChiknError> {
    git::sync_status(Path::new(&project_path))
}

fn remote_from_settings() -> Result<(String, git::RemoteAuth), ChiknError> {
    let settings = super::settings::get_app_settings();
    let url = settings.remote.url.clone().ok_or_else(|| {
        ChiknError::Unknown(
            "No remote URL configured. Set one in Settings > Remote.".to_string(),
        )
    })?;
    let auth = git::RemoteAuth {
        username: settings.remote.username.clone(),
        token: settings.remote.token.clone(),
    };
    Ok((url, auth))
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
