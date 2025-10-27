//! Snapshot API commands
//!
//! Tauri commands for backup and restore operations.

use crate::core::snapshots::{
    create_snapshot, prune_old_snapshots, restore_snapshot, SnapshotManifest, SnapshotType,
    REVS_FOLDER,
};
use crate::utils::error::ChiknError;
use std::path::Path;

/// Creates a snapshot of the current project state.
///
/// # Arguments
/// * `project_path` - Path to .chikn project
/// * `description` - Optional user description
/// * `is_automatic` - True for auto-snapshots, false for manual
///
/// # Returns
/// * `Ok(String)` - Filename of created snapshot
/// * `Err(ChiknError)` on failure
///
/// # Example (from frontend)
/// ```javascript
/// const filename = await invoke('create_snapshot', {
///   projectPath: '/path/to/Novel.chikn',
///   description: 'Before major rewrite',
///   isAutomatic: false
/// });
/// ```
#[tauri::command]
pub async fn create_project_snapshot(
    project_path: String,
    description: Option<String>,
    is_automatic: bool,
) -> Result<String, ChiknError> {
    let path = Path::new(&project_path);

    let snapshot_type = if is_automatic {
        SnapshotType::Automatic
    } else {
        SnapshotType::Manual
    };

    create_snapshot(path, snapshot_type, description)
}

/// Restores project from a snapshot.
///
/// # Arguments
/// * `project_path` - Path to .chikn project
/// * `snapshot_filename` - Snapshot to restore
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(ChiknError)` on failure
///
/// # Warning
/// Overwrites current project state!
///
/// # Example (from frontend)
/// ```javascript
/// await invoke('restore_from_snapshot', {
///   projectPath: '/path/to/Novel.chikn',
///   snapshotFilename: 'snapshot-20251004-143022.tar.gz'
/// });
/// ```
#[tauri::command]
pub async fn restore_from_snapshot(
    project_path: String,
    snapshot_filename: String,
) -> Result<(), ChiknError> {
    let path = Path::new(&project_path);

    restore_snapshot(path, &snapshot_filename)
}

/// Lists all available snapshots for a project.
///
/// # Arguments
/// * `project_path` - Path to .chikn project
///
/// # Returns
/// * `Ok(SnapshotManifest)` - List of snapshots with metadata
/// * `Err(ChiknError)` on failure
///
/// # Example (from frontend)
/// ```javascript
/// const manifest = await invoke('list_snapshots', {
///   projectPath: '/path/to/Novel.chikn'
/// });
/// console.log(`Found ${manifest.snapshots.length} snapshots`);
/// ```
#[tauri::command]
pub async fn list_snapshots(project_path: String) -> Result<SnapshotManifest, ChiknError> {
    let path = Path::new(&project_path);
    let revs_path = path.join(REVS_FOLDER);

    SnapshotManifest::load(&revs_path)
}

/// Prunes old snapshots, keeping only the most recent N.
///
/// # Arguments
/// * `project_path` - Path to .chikn project
/// * `keep_count` - Number of snapshots to keep
///
/// # Returns
/// * `Ok(usize)` - Number of snapshots deleted
/// * `Err(ChiknError)` on failure
///
/// # Example (from frontend)
/// ```javascript
/// const deleted = await invoke('prune_snapshots', {
///   projectPath: '/path/to/Novel.chikn',
///   keepCount: 10
/// });
/// console.log(`Deleted ${deleted} old snapshots`);
/// ```
#[tauri::command]
pub async fn prune_snapshots(project_path: String, keep_count: usize) -> Result<usize, ChiknError> {
    let path = Path::new(&project_path);

    prune_old_snapshots(path, Some(keep_count))
}
