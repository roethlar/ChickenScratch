use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use crate::utils::error::ChiknError;

pub fn ensure_project_subdir_safe(
    project_path: &Path,
    relative_path: &Path,
) -> Result<PathBuf, ChiknError> {
    let (path, _) = ensure_project_subdir_safe_with_status(project_path, relative_path)?;
    Ok(path)
}

pub fn ensure_project_subdir_safe_with_status(
    project_path: &Path,
    relative_path: &Path,
) -> Result<(PathBuf, bool), ChiknError> {
    let project_root = canonical_project_root(project_path)?;
    let mut current = project_path.to_path_buf();
    let mut created = false;
    let mut has_component = false;

    for component in relative_path.components() {
        let Component::Normal(part) = component else {
            return Err(invalid_project_subdir(
                relative_path,
                "only normal relative path components are allowed",
            ));
        };
        has_component = true;
        current.push(part);

        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                ensure_directory_metadata_safe(&current, &metadata, &project_root, relative_path)?;
            }
            Err(e) if e.kind() == ErrorKind::NotFound => {
                fs::create_dir(&current)?;
                let metadata = fs::symlink_metadata(&current)?;
                ensure_directory_metadata_safe(&current, &metadata, &project_root, relative_path)?;
                created = true;
            }
            Err(e) => return Err(e.into()),
        }
    }

    if !has_component {
        return Err(invalid_project_subdir(
            relative_path,
            "path must contain at least one component",
        ));
    }

    Ok((current, created))
}

fn canonical_project_root(project_path: &Path) -> Result<PathBuf, ChiknError> {
    let project_root = project_path.canonicalize()?;
    if !project_root.is_dir() {
        return Err(ChiknError::InvalidFormat(format!(
            "Project path is not a directory: {}",
            project_path.display()
        )));
    }
    Ok(project_root)
}

fn ensure_directory_metadata_safe(
    path: &Path,
    metadata: &fs::Metadata,
    project_root: &Path,
    relative_path: &Path,
) -> Result<(), ChiknError> {
    if metadata.file_type().is_symlink() {
        return Err(ChiknError::InvalidFormat(format!(
            "Project subdirectory is a symlink: {} ({})",
            relative_path.display(),
            path.display()
        )));
    }
    if !metadata.is_dir() {
        return Err(ChiknError::InvalidFormat(format!(
            "Project subdirectory path is not a directory: {} ({})",
            relative_path.display(),
            path.display()
        )));
    }

    let canonical_path = path.canonicalize()?;
    if !canonical_path.starts_with(project_root) {
        return Err(ChiknError::InvalidFormat(format!(
            "Project subdirectory escapes project root: {} ({})",
            relative_path.display(),
            path.display()
        )));
    }

    Ok(())
}

fn invalid_project_subdir(relative_path: &Path, reason: &str) -> ChiknError {
    ChiknError::InvalidFormat(format!(
        "Project subdirectory path must be relative and within project ({}): {}",
        reason,
        relative_path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::ensure_project_subdir_safe;
    use crate::ChiknError;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn creates_missing_safe_subdir() {
        let temp_dir = TempDir::new().unwrap();
        let path = ensure_project_subdir_safe(temp_dir.path(), Path::new("characters")).unwrap();

        assert_eq!(path, temp_dir.path().join("characters"));
        assert!(path.is_dir());
    }

    #[test]
    fn creates_missing_nested_safe_subdir() {
        let temp_dir = TempDir::new().unwrap();
        let path =
            ensure_project_subdir_safe(temp_dir.path(), Path::new("characters/group")).unwrap();

        assert_eq!(path, temp_dir.path().join("characters/group"));
        assert!(path.is_dir());
    }

    #[test]
    fn rejects_parent_dir_component() {
        let temp_dir = TempDir::new().unwrap();
        let result = ensure_project_subdir_safe(temp_dir.path(), Path::new("../escape"));

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
    }

    #[test]
    fn rejects_absolute_path() {
        let temp_dir = TempDir::new().unwrap();
        let result = ensure_project_subdir_safe(temp_dir.path(), Path::new("/tmp/escape"));

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_subdir_without_touching_target() {
        use std::fs;
        use std::os::unix::fs as unix_fs;

        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("Project.chikn");
        let outside_path = temp_dir.path().join("outside");
        fs::create_dir(&project_path).unwrap();
        fs::create_dir(&outside_path).unwrap();
        unix_fs::symlink(&outside_path, project_path.join("characters")).unwrap();

        let result = ensure_project_subdir_safe(&project_path, Path::new("characters"));

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
        assert!(fs::read_dir(&outside_path).unwrap().next().is_none());
    }
}
