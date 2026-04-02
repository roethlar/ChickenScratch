//! # Snapshot Pruning
//!
//! Manages snapshot retention and cleanup.

use std::fs;
use std::path::Path;

use super::manifest::SnapshotManifest;
use super::{DEFAULT_SNAPSHOT_COUNT, REVS_FOLDER};
use crate::utils::error::ChiknError;

/// Prunes old snapshots, keeping only the most recent N.
///
/// # Arguments
/// * `project_path` - Root path of .chikn project
/// * `keep_count` - Number of snapshots to keep (None = use default)
///
/// # Returns
/// * `Ok(usize)` - Number of snapshots deleted
/// * `Err(ChiknError)` on failure
///
/// # Example
/// ```rust
/// // Keep only last 10 snapshots
/// let deleted = prune_old_snapshots(Path::new("MyNovel.chikn"), Some(10))?;
/// println!("Deleted {} old snapshots", deleted);
/// ```
pub fn prune_old_snapshots(
    project_path: &Path,
    keep_count: Option<usize>,
) -> Result<usize, ChiknError> {
    let revs_path = project_path.join(REVS_FOLDER);

    if !revs_path.exists() {
        return Ok(0); // No snapshots to prune
    }

    // Load manifest
    let mut manifest = SnapshotManifest::load(&revs_path)?;

    // Determine how many to keep
    let keep = keep_count.unwrap_or(DEFAULT_SNAPSHOT_COUNT);

    // Get list of snapshots to delete
    let to_delete = manifest.prune(keep);
    let delete_count = to_delete.len();

    // Delete snapshot files
    for filename in &to_delete {
        let snapshot_path = revs_path.join(filename);
        if snapshot_path.exists() {
            fs::remove_file(&snapshot_path)?;
        }
    }

    // Save updated manifest
    manifest.save(&revs_path)?;

    Ok(delete_count)
}

/// Gets the total size of all snapshots
///
/// # Arguments
/// * `project_path` - Root path of .chikn project
///
/// # Returns
/// Total size in bytes
pub fn get_snapshots_size(project_path: &Path) -> Result<u64, ChiknError> {
    let revs_path = project_path.join(REVS_FOLDER);

    if !revs_path.exists() {
        return Ok(0);
    }

    let manifest = SnapshotManifest::load(&revs_path)?;

    let total_size: u64 = manifest.snapshots.iter().map(|s| s.size_bytes).sum();

    Ok(total_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::project::writer::create_project;
    use crate::core::snapshots::{create_snapshot, SnapshotType};
    use tempfile::TempDir;

    #[test]
    fn test_prune_old_snapshots() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("Test.chikn");

        create_project(&project_path, "Test").unwrap();

        // Create 5 snapshots
        for i in 0..5 {
            create_snapshot(
                &project_path,
                SnapshotType::Automatic,
                Some(format!("Snapshot {}", i)),
            )
            .unwrap();
        }

        // Verify 5 snapshots exist
        let manifest = SnapshotManifest::load(&project_path.join(REVS_FOLDER)).unwrap();
        assert_eq!(manifest.snapshots.len(), 5);

        // Prune to keep only 3
        let deleted = prune_old_snapshots(&project_path, Some(3)).unwrap();
        assert_eq!(deleted, 2);

        // Verify only 3 remain
        let manifest = SnapshotManifest::load(&project_path.join(REVS_FOLDER)).unwrap();
        assert_eq!(manifest.snapshots.len(), 3);
    }

    #[test]
    fn test_get_snapshots_size() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("Test.chikn");

        create_project(&project_path, "Test").unwrap();

        // Initially no snapshots
        let size = get_snapshots_size(&project_path).unwrap();
        assert_eq!(size, 0);

        // Create snapshot
        create_snapshot(&project_path, SnapshotType::Manual, None).unwrap();

        // Size should be > 0
        let size = get_snapshots_size(&project_path).unwrap();
        assert!(size > 0);
    }

    #[test]
    fn test_prune_with_no_snapshots() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("Test.chikn");

        create_project(&project_path, "Test").unwrap();

        // Prune when no snapshots exist
        let deleted = prune_old_snapshots(&project_path, Some(10)).unwrap();
        assert_eq!(deleted, 0);
    }
}
