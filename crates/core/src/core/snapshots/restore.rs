//! # Snapshot Restoration
//!
//! Restores projects from snapshot archives.

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::{self, File};
use std::path::Path;
use tar::{Archive, Builder};

use super::REVS_FOLDER;
use crate::utils::error::ChiknError;

/// Restores a project from a snapshot.
///
/// # Arguments
/// * `project_path` - Root path of .chikn project
/// * `snapshot_filename` - Snapshot file to restore from
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(ChiknError)` on failure
///
/// # Warning
/// This overwrites the current project state!
///
/// # Safety
/// Creates backup before restore. Automatically rolls back on failure.
///
/// # Example
/// ```rust
/// restore_snapshot(
///     Path::new("MyNovel.chikn"),
///     "snapshot-20251004-120000.tar.gz"
/// )?;
/// ```
pub fn restore_snapshot(project_path: &Path, snapshot_filename: &str) -> Result<(), ChiknError> {
    let snapshot_path = project_path.join(REVS_FOLDER).join(snapshot_filename);

    if !snapshot_path.exists() {
        return Err(ChiknError::NotFound(format!(
            "Snapshot not found: {}",
            snapshot_filename
        )));
    }

    // Create backup of current state before restore (includes .git and revs)
    let backup_path = project_path.join(REVS_FOLDER).join("restore-backup.tar.gz");
    create_full_backup(project_path, &backup_path)?;

    // Clear working tree (except revs/ and .git/)
    if let Err(e) = clear_working_tree(project_path) {
        // Restore from backup on clear failure
        let _ = restore_from_backup(project_path, &backup_path);
        let _ = fs::remove_file(&backup_path);
        return Err(e);
    }

    // Extract tarball
    if let Err(e) = extract_tarball(&snapshot_path, project_path) {
        // CRITICAL: Restore from backup on extraction failure
        eprintln!("Extraction failed, rolling back to backup...");
        if let Err(rollback_err) = restore_from_backup(project_path, &backup_path) {
            eprintln!("CRITICAL: Rollback failed: {:?}", rollback_err);
            // Backup file still exists for manual recovery
            return Err(ChiknError::Unknown(format!(
                "Restore failed and rollback failed. Backup saved at: {}. Error: {}",
                backup_path.display(),
                e
            )));
        }
        let _ = fs::remove_file(&backup_path);
        return Err(e);
    }

    // Success - clean up backup
    let _ = fs::remove_file(&backup_path);

    Ok(())
}

/// Creates a full backup including .git and revs (for restore safety)
fn create_full_backup(project_path: &Path, backup_path: &Path) -> Result<(), ChiknError> {
    let tar_gz = File::create(backup_path)?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = Builder::new(enc);

    // Archive everything (including .git and revs for full recovery)
    for entry in fs::read_dir(project_path)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();

        // Skip the backup file itself if it exists
        if name == "restore-backup.tar.gz" {
            continue;
        }

        let relative = path
            .strip_prefix(project_path)
            .map_err(|_| ChiknError::InvalidFormat("Path error".to_string()))?;

        if path.is_file() {
            let mut file = File::open(&path)?;
            tar.append_file(relative, &mut file)?;
        } else if path.is_dir() {
            tar.append_dir_all(relative, &path)?;
        }
    }

    tar.finish()?;
    Ok(())
}

/// Restores from the backup tarball
fn restore_from_backup(project_path: &Path, backup_path: &Path) -> Result<(), ChiknError> {
    // Clear everything first
    for entry in fs::read_dir(project_path)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();

        // Don't delete the backup we're about to restore from
        if name == "restore-backup.tar.gz" || name == REVS_FOLDER {
            continue;
        }

        if path.is_file() {
            fs::remove_file(&path)?;
        } else if path.is_dir() {
            fs::remove_dir_all(&path)?;
        }
    }

    // Extract backup
    extract_tarball(backup_path, project_path)?;

    Ok(())
}

/// Clears the working tree, preserving revs/ and .git/
fn clear_working_tree(project_path: &Path) -> Result<(), ChiknError> {
    for entry in fs::read_dir(project_path)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();

        // Preserve revs/ and .git/
        if name == REVS_FOLDER || name == ".git" {
            continue;
        }

        // Remove everything else
        if path.is_file() {
            fs::remove_file(&path)?;
        } else if path.is_dir() {
            fs::remove_dir_all(&path)?;
        }
    }

    Ok(())
}

/// Extracts a tarball to the project directory
fn extract_tarball(tar_path: &Path, extract_to: &Path) -> Result<(), ChiknError> {
    let tar_gz = File::open(tar_path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);

    // Extract all files
    archive
        .unpack(extract_to)
        .map_err(|e| ChiknError::Unknown(format!("Failed to extract snapshot: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::project::writer::create_project;
    use crate::core::snapshots::{create_snapshot, SnapshotType};
    use crate::models::Document;
    use tempfile::TempDir;

    #[test]
    fn test_restore_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("Test.chikn");

        // Create project and add a document
        let mut project = create_project(&project_path, "Test").unwrap();

        let doc = Document {
            id: "doc1".to_string(),
            name: "Original".to_string(),
            path: "manuscript/original.md".to_string(),
            content: "Original content".to_string(),
            parent_id: None,
            created: chrono::Utc::now().to_rfc3339(),
            modified: chrono::Utc::now().to_rfc3339(),
            ..Default::default()
        };

        project.documents.insert(doc.id.clone(), doc);
        crate::core::project::writer::write_project(&mut project).unwrap();

        // Create snapshot
        let filename = create_snapshot(&project_path, SnapshotType::Manual, None).unwrap();

        // Modify project
        let doc2 = Document {
            id: "doc2".to_string(),
            name: "Modified".to_string(),
            path: "manuscript/modified.md".to_string(),
            content: "Modified content".to_string(),
            parent_id: None,
            created: chrono::Utc::now().to_rfc3339(),
            modified: chrono::Utc::now().to_rfc3339(),
            ..Default::default()
        };

        project.documents.insert(doc2.id.clone(), doc2);
        crate::core::project::writer::write_project(&mut project).unwrap();

        // Verify modified state
        assert!(project_path.join("manuscript/modified.md").exists());

        // Restore snapshot
        let result = restore_snapshot(&project_path, &filename);
        assert!(result.is_ok());

        // Verify original state restored
        assert!(project_path.join("manuscript/original.md").exists());
        assert!(!project_path.join("manuscript/modified.md").exists());
    }

    #[test]
    fn test_restore_nonexistent_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("Test.chikn");
        create_project(&project_path, "Test").unwrap();

        let result = restore_snapshot(&project_path, "nonexistent.tar.gz");
        assert!(result.is_err());
    }

    #[test]
    fn test_restore_rollback_on_failure() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("Test.chikn");

        // Create project with content
        let mut project = create_project(&project_path, "Test").unwrap();

        let doc = Document {
            id: "doc1".to_string(),
            name: "Important".to_string(),
            path: "manuscript/important.md".to_string(),
            content: "Critical content".to_string(),
            parent_id: None,
            created: chrono::Utc::now().to_rfc3339(),
            modified: chrono::Utc::now().to_rfc3339(),
            ..Default::default()
        };

        project.documents.insert(doc.id.clone(), doc);
        crate::core::project::writer::write_project(&mut project).unwrap();

        // Create a valid snapshot
        create_snapshot(&project_path, SnapshotType::Manual, None).unwrap();

        // Try to restore from a corrupted/invalid snapshot
        // (Create a fake snapshot file)
        let revs_path = project_path.join(REVS_FOLDER);
        let bad_snapshot = revs_path.join("bad-snapshot.tar.gz");
        fs::write(&bad_snapshot, "not a valid tarball").unwrap();

        // Attempt restore - should fail but preserve original data
        let result = restore_snapshot(&project_path, "bad-snapshot.tar.gz");
        assert!(result.is_err());

        // Verify original data is still intact (rollback succeeded)
        assert!(project_path.join("manuscript/important.md").exists());

        let content = fs::read_to_string(project_path.join("manuscript/important.md")).unwrap();
        assert_eq!(content, "Critical content");
    }
}
