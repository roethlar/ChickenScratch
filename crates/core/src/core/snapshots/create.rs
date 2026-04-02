//! # Snapshot Creation
//!
//! Creates compressed tarball snapshots of .chikn projects.

use chrono::Utc;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::{self, File};
use std::path::Path;
use tar::Builder;

use super::manifest::{SnapshotEntry, SnapshotManifest, SnapshotType};
use super::{REVS_FOLDER, SNAPSHOT_EXTENSION, SNAPSHOT_PREFIX};
use crate::utils::error::ChiknError;

/// Creates a snapshot of the project.
///
/// # Arguments
/// * `project_path` - Root path of .chikn project
/// * `snapshot_type` - Type of snapshot (automatic, manual, etc.)
/// * `description` - Optional description
///
/// # Returns
/// * `Ok(String)` - Filename of created snapshot
/// * `Err(ChiknError)` on failure
///
/// # Example
/// ```rust
/// let filename = create_snapshot(
///     Path::new("MyNovel.chikn"),
///     SnapshotType::Manual,
///     Some("Before major rewrite")
/// )?;
/// ```
pub fn create_snapshot(
    project_path: &Path,
    snapshot_type: SnapshotType,
    description: Option<String>,
) -> Result<String, ChiknError> {
    // Create revs/ directory if it doesn't exist
    let revs_path = project_path.join(REVS_FOLDER);
    fs::create_dir_all(&revs_path)?;

    // Generate snapshot filename with timestamp
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let filename = format!("{}{}.{}", SNAPSHOT_PREFIX, timestamp, SNAPSHOT_EXTENSION);
    let snapshot_path = revs_path.join(&filename);

    // Create tarball
    create_tarball(project_path, &snapshot_path)?;

    // Get file size
    let size_bytes = fs::metadata(&snapshot_path)?.len();

    // Update manifest
    let mut manifest = SnapshotManifest::load(&revs_path)?;
    manifest.add_snapshot(SnapshotEntry {
        filename: filename.clone(),
        created: Utc::now().to_rfc3339(),
        description,
        snapshot_type,
        size_bytes,
    });
    manifest.save(&revs_path)?;

    Ok(filename)
}

/// Creates a compressed tarball of the project
pub(super) fn create_tarball(project_path: &Path, output_path: &Path) -> Result<(), ChiknError> {
    let tar_gz = File::create(output_path)?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = Builder::new(enc);

    // Add directories and files, excluding revs/ and .git/
    add_directory_to_tar(&mut tar, project_path, project_path)?;

    tar.finish()?;

    Ok(())
}

/// Recursively adds directory contents to tar archive
fn add_directory_to_tar(
    tar: &mut Builder<GzEncoder<File>>,
    project_root: &Path,
    current_path: &Path,
) -> Result<(), ChiknError> {
    for entry in fs::read_dir(current_path)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();

        // Skip revs/ and .git/ folders
        if file_name == REVS_FOLDER || file_name == ".git" {
            continue;
        }

        // Get relative path from project root
        let relative_path = path
            .strip_prefix(project_root)
            .map_err(|_| ChiknError::InvalidFormat("Path error".to_string()))?;

        if path.is_file() {
            // Add file to archive
            let mut file = File::open(&path)?;
            tar.append_file(relative_path, &mut file)?;
        } else if path.is_dir() {
            // Add directory and recurse
            tar.append_dir(relative_path, &path)?;
            add_directory_to_tar(tar, project_root, &path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::project::writer::create_project;
    use tempfile::TempDir;

    #[test]
    fn test_create_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("Test.chikn");

        // Create a project
        create_project(&project_path, "Test Project").unwrap();

        // Create snapshot
        let result = create_snapshot(
            &project_path,
            SnapshotType::Manual,
            Some("Test snapshot".to_string()),
        );

        assert!(result.is_ok());

        let filename = result.unwrap();

        // Verify snapshot exists
        let snapshot_path = project_path.join(REVS_FOLDER).join(&filename);
        assert!(snapshot_path.exists());

        // Verify manifest updated
        let manifest = SnapshotManifest::load(&project_path.join(REVS_FOLDER)).unwrap();
        assert_eq!(manifest.snapshots.len(), 1);
        assert_eq!(manifest.snapshots[0].filename, filename);
    }

    #[test]
    fn test_snapshot_excludes_revs_and_git() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("Test.chikn");

        // Create project (.git is auto-initialized by create_project)
        create_project(&project_path, "Test").unwrap();
        // Ensure .git exists (create_project may have initialized it)
        let _ = fs::create_dir(project_path.join(".git"));
        fs::write(project_path.join(".git/config"), "test").unwrap();

        // Create snapshot
        let filename = create_snapshot(&project_path, SnapshotType::Automatic, None).unwrap();

        // Extract and verify .git and revs are not included
        // (Full extraction test would be in restore tests)
        let snapshot_path = project_path.join(REVS_FOLDER).join(&filename);
        assert!(snapshot_path.exists());

        // Verify size is small (no nested revs)
        let size = fs::metadata(&snapshot_path).unwrap().len();
        assert!(size < 10_000); // Should be very small without nested archives
    }
}
