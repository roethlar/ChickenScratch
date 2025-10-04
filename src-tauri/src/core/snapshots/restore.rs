//! # Snapshot Restoration
//!
//! Restores projects from snapshot archives.

use std::fs::{self, File};
use std::path::Path;
use flate2::read::GzDecoder;
use tar::Archive;

use crate::utils::error::ChiknError;
use super::REVS_FOLDER;

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
/// # Example
/// ```rust
/// restore_snapshot(
///     Path::new("MyNovel.chikn"),
///     "snapshot-20251004-120000.tar.gz"
/// )?;
/// ```
pub fn restore_snapshot(
    project_path: &Path,
    snapshot_filename: &str,
) -> Result<(), ChiknError> {
    let snapshot_path = project_path.join(REVS_FOLDER).join(snapshot_filename);

    if !snapshot_path.exists() {
        return Err(ChiknError::NotFound(format!(
            "Snapshot not found: {}",
            snapshot_filename
        )));
    }

    // Create backup of current state before restore
    let backup_path = project_path.join(REVS_FOLDER).join("restore-backup.tar.gz");
    if let Err(e) = create_restore_backup(project_path, &backup_path) {
        eprintln!("Warning: Could not create restore backup: {:?}", e);
    }

    // Clear working tree (except revs/ and .git/)
    clear_working_tree(project_path)?;

    // Extract tarball
    extract_tarball(&snapshot_path, project_path)?;

    // Clean up backup
    let _ = fs::remove_file(&backup_path);

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

/// Creates a temporary backup before restore (safety)
fn create_restore_backup(project_path: &Path, backup_path: &Path) -> Result<(), ChiknError> {
    use super::create::create_tarball;
    create_tarball(project_path, backup_path)
}

/// Extracts a tarball to the project directory
fn extract_tarball(tar_path: &Path, extract_to: &Path) -> Result<(), ChiknError> {
    let tar_gz = File::open(tar_path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);

    // Extract all files
    archive.unpack(extract_to)
        .map_err(|e| ChiknError::Unknown(format!("Failed to extract snapshot: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::core::project::writer::create_project;
    use crate::core::snapshots::{create_snapshot, SnapshotType};
    use crate::models::Document;

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
}
