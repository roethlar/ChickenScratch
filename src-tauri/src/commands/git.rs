use chickenscratch_core::core::git;
use chickenscratch_core::ChiknError;
use std::path::Path;
use tauri::State;

use super::ProjectWriteLocks;

#[tauri::command]
pub fn save_revision(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    message: String,
) -> Result<git::Revision, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let path = Path::new(&project_path);
        let rev = git::save_revision(path, &message)?;

        // After named revision: push to backup remote and remote-sync if configured.
        // Both are fire-and-forget — a failed push must not fail the revision.
        let settings = super::settings::get_app_settings_hydrated();
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
    })
}

#[tauri::command]
pub fn list_revisions(project_path: String) -> Result<Vec<git::Revision>, ChiknError> {
    git::list_revisions(Path::new(&project_path))
}

#[tauri::command]
pub fn restore_revision(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    commit_id: String,
) -> Result<git::Revision, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        git::restore_revision(Path::new(&project_path), &commit_id)
    })
}

#[tauri::command]
pub fn create_draft(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    name: String,
) -> Result<(), ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        git::create_draft(Path::new(&project_path), &name)
    })
}

#[tauri::command]
pub fn list_drafts(project_path: String) -> Result<Vec<git::DraftVersion>, ChiknError> {
    git::list_drafts(Path::new(&project_path))
}

#[tauri::command]
pub fn switch_draft(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    name: String,
) -> Result<(), ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        git::switch_draft(Path::new(&project_path), &name)
    })
}

#[tauri::command]
pub fn merge_draft(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    name: String,
) -> Result<git::MergeResult, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        git::merge_draft(Path::new(&project_path), &name)
    })
}

#[tauri::command]
pub fn push_backup(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    backup_dir: String,
) -> Result<(), ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        git::push_backup(Path::new(&project_path), Path::new(&backup_dir))
    })
}

/// Push the current branch to the remote URL configured in settings.
#[tauri::command]
pub fn sync_push(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
) -> Result<(), ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let (url, auth) = remote_from_settings()?;
        git::push_remote(Path::new(&project_path), &url, &auth)
    })
}

/// Fetch the current branch from the configured remote. Does not merge.
#[tauri::command]
pub fn sync_fetch(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
) -> Result<(), ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let (url, auth) = remote_from_settings()?;
        git::fetch_remote(Path::new(&project_path), &url, &auth)
    })
}

/// Ahead/behind counts relative to the last-fetched remote tracking ref.
#[tauri::command]
pub fn sync_status(project_path: String) -> Result<git::SyncStatus, ChiknError> {
    git::sync_status(Path::new(&project_path))
}

#[tauri::command]
pub fn document_history(
    project_path: String,
    doc_path: String,
) -> Result<Vec<git::Revision>, ChiknError> {
    git::document_history(Path::new(&project_path), &doc_path)
}

#[tauri::command]
pub fn restore_document(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    doc_path: String,
    commit_id: String,
) -> Result<git::Revision, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        git::restore_document(Path::new(&project_path), &doc_path, &commit_id)
    })
}

/// Pull (fetch + merge). Returns one of: up_to_date, fast_forward, merged,
/// conflicts (with file list). Conflicts leave the working tree with merge
/// markers; call `sync_abort_pull` to revert or `sync_pull_force` to discard
/// local changes.
#[tauri::command]
pub fn sync_pull(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
) -> Result<git::PullResult, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let (url, auth) = remote_from_settings()?;
        git::sync_pull(Path::new(&project_path), &url, &auth)
    })
}

#[tauri::command]
pub fn sync_abort_pull(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
) -> Result<(), ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        git::sync_abort_pull(Path::new(&project_path))
    })
}

#[tauri::command]
pub fn sync_pull_force(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
) -> Result<(), ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let (url, auth) = remote_from_settings()?;
        git::sync_pull_force(Path::new(&project_path), &url, &auth)
    })
}

fn remote_from_settings() -> Result<(String, git::RemoteAuth), ChiknError> {
    let settings = super::settings::get_app_settings_hydrated();
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
pub fn compare_drafts(
    project_path: String,
    draft_a: String,
    draft_b: String,
) -> Result<Vec<git::FileDiff>, ChiknError> {
    git::compare_drafts(Path::new(&project_path), &draft_a, &draft_b)
}

#[tauri::command]
pub fn word_diff_drafts(
    project_path: String,
    draft_a: String,
    draft_b: String,
    doc_path: String,
) -> Result<Vec<(String, String)>, ChiknError> {
    git::word_diff_drafts(Path::new(&project_path), &draft_a, &draft_b, &doc_path)
}

#[tauri::command]
pub fn has_changes(project_path: String) -> Result<bool, ChiknError> {
    git::has_changes(Path::new(&project_path))
}

/// Auto-backup: save any unsaved changes and push to backup remote if configured.
/// Called on app close. Non-fatal — failures are logged but don't block exit.
#[tauri::command]
pub fn backup_on_close(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
) -> Result<(), ChiknError> {
    write_locks.with_project_lock(&project_path, || {
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
    })
}
