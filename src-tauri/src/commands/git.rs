//! Thin Tauri wrappers over `chickenscratch_services::git` — transport only.
use chickenscratch_core::core::git;
use chickenscratch_services::git as svc;
#[allow(unused_imports)]
use chickenscratch_services::git::*;
use chickenscratch_services::ChiknError;
use chickenscratch_services::{ProjectTokens, ProjectWriteLocks};
use tauri::State;

#[tauri::command]
pub fn save_revision(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    message: String,
) -> Result<git::Revision, ChiknError> {
    svc::save_revision(project_path, &write_locks, &tokens, message)
}

#[tauri::command]
pub fn list_revisions(project_path: String) -> Result<Vec<git::Revision>, ChiknError> {
    svc::list_revisions(project_path)
}

#[tauri::command]
pub fn restore_revision(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    commit_id: String,
) -> Result<git::Revision, ChiknError> {
    svc::restore_revision(project_path, &write_locks, &tokens, commit_id)
}

#[tauri::command]
pub fn create_draft(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    name: String,
) -> Result<(), ChiknError> {
    svc::create_draft(project_path, &write_locks, &tokens, name)
}

#[tauri::command]
pub fn list_drafts(project_path: String) -> Result<Vec<git::DraftVersion>, ChiknError> {
    svc::list_drafts(project_path)
}

#[tauri::command]
pub fn switch_draft(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    name: String,
) -> Result<(), ChiknError> {
    svc::switch_draft(project_path, &write_locks, &tokens, name)
}

#[tauri::command]
pub fn merge_draft(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    name: String,
) -> Result<git::MergeResult, ChiknError> {
    svc::merge_draft(project_path, &write_locks, &tokens, name)
}

#[tauri::command]
pub fn push_backup(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    backup_dir: String,
) -> Result<(), ChiknError> {
    svc::push_backup(project_path, &write_locks, &tokens, backup_dir)
}

#[tauri::command]
pub fn manual_backup(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    backup_dir: String,
) -> Result<Option<git::Revision>, ChiknError> {
    svc::manual_backup(project_path, &write_locks, &tokens, backup_dir)
}

#[tauri::command]
pub fn sync_push(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
) -> Result<(), ChiknError> {
    svc::sync_push(project_path, &write_locks, &tokens)
}

#[tauri::command]
pub fn sync_fetch(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
) -> Result<(), ChiknError> {
    svc::sync_fetch(project_path, &write_locks, &tokens)
}

#[tauri::command]
pub fn sync_status(project_path: String) -> Result<git::SyncStatus, ChiknError> {
    svc::sync_status(project_path)
}

#[tauri::command]
pub fn document_history(
    project_path: String,
    doc_path: String,
) -> Result<Vec<git::Revision>, ChiknError> {
    svc::document_history(project_path, doc_path)
}

#[tauri::command]
pub fn restore_document(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    doc_path: String,
    commit_id: String,
) -> Result<git::Revision, ChiknError> {
    svc::restore_document(project_path, &write_locks, &tokens, doc_path, commit_id)
}

#[tauri::command]
pub fn sync_pull(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
) -> Result<git::PullResult, ChiknError> {
    svc::sync_pull(project_path, &write_locks, &tokens)
}

#[tauri::command]
pub fn sync_abort_pull(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
) -> Result<(), ChiknError> {
    svc::sync_abort_pull(project_path, &write_locks, &tokens)
}

#[tauri::command]
pub fn merge_state(project_path: String) -> Result<git::MergeState, ChiknError> {
    svc::merge_state(project_path)
}

#[tauri::command]
pub fn complete_merge(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    message: String,
) -> Result<git::Revision, ChiknError> {
    svc::complete_merge(project_path, &write_locks, &tokens, message)
}

#[tauri::command]
pub fn force_resolve_merge(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    attestation: String,
) -> Result<(), ChiknError> {
    svc::force_resolve_merge(project_path, &write_locks, &tokens, attestation)
}

#[tauri::command]
pub fn sync_pull_force(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
) -> Result<(), ChiknError> {
    svc::sync_pull_force(project_path, &write_locks, &tokens)
}

#[tauri::command]
pub fn revision_diff(
    project_path: String,
    commit_id: String,
) -> Result<Vec<git::FileDiff>, ChiknError> {
    svc::revision_diff(project_path, commit_id)
}

#[tauri::command]
pub fn word_diff(
    project_path: String,
    commit_id: String,
    doc_path: String,
) -> Result<Vec<(String, String)>, ChiknError> {
    svc::word_diff(project_path, commit_id, doc_path)
}

#[tauri::command]
pub fn compare_drafts(
    project_path: String,
    draft_a: String,
    draft_b: String,
) -> Result<Vec<git::FileDiff>, ChiknError> {
    svc::compare_drafts(project_path, draft_a, draft_b)
}

#[tauri::command]
pub fn word_diff_drafts(
    project_path: String,
    draft_a: String,
    draft_b: String,
    doc_path: String,
) -> Result<Vec<(String, String)>, ChiknError> {
    svc::word_diff_drafts(project_path, draft_a, draft_b, doc_path)
}

#[tauri::command]
pub fn list_draft_documents(
    project_path: String,
    draft: String,
) -> Result<Vec<git::DraftDocument>, ChiknError> {
    svc::list_draft_documents(project_path, draft)
}

#[tauri::command]
pub fn read_draft_document(
    project_path: String,
    draft: String,
    doc_path: String,
) -> Result<String, ChiknError> {
    svc::read_draft_document(project_path, draft, doc_path)
}

#[tauri::command]
pub fn has_changes(project_path: String) -> Result<bool, ChiknError> {
    svc::has_changes(project_path)
}

#[tauri::command]
pub fn backup_on_close(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
) -> Result<(), ChiknError> {
    svc::backup_on_close(project_path, &write_locks, &tokens)
}
