//! # .chikn Format Constants and Validation
//!
//! Defines the .chikn file format structure and validation rules.
//!
//! ## Responsibilities
//! - Define file/folder naming constants
//! - Validate .chikn project structure
//! - Ensure format compliance
//!
//! ## Format Structure
//! ```text
//! MyProject.chikn/
//! ├── project.yaml              # Project metadata
//! ├── manuscript/               # Main writing folder
//! │   ├── document.md           # Document content (Pandoc Markdown)
//! │   └── document.meta         # Document metadata (YAML)
//! ├── research/                 # Research folder
//! ├── templates/                # Character/setting templates
//! └── settings/                 # Compile settings, themes
//! ```

use crate::utils::error::ChiknError;
use std::path::{Path, PathBuf};

/// File extension for ChickenScratch projects
pub const PROJECT_EXTENSION: &str = "chikn";

/// Project metadata file name
pub const PROJECT_FILE: &str = "project.yaml";

/// Manuscript folder (main writing)
pub const MANUSCRIPT_FOLDER: &str = "manuscript";

/// Research folder
pub const RESEARCH_FOLDER: &str = "research";

/// Templates folder
pub const TEMPLATES_FOLDER: &str = "templates";

/// Settings folder
pub const SETTINGS_FOLDER: &str = "settings";

/// Document content file extension
pub const DOCUMENT_EXTENSION: &str = "md";

/// Metadata file extension
pub const METADATA_EXTENSION: &str = "meta";

/// Required folders in a .chikn project
pub const REQUIRED_FOLDERS: &[&str] = &[
    MANUSCRIPT_FOLDER,
    RESEARCH_FOLDER,
    TEMPLATES_FOLDER,
    SETTINGS_FOLDER,
];

/// Validates that a path is a valid .chikn project directory.
///
/// # Arguments
/// * `path` - Path to potential .chikn project folder
///
/// # Returns
/// * `Ok(())` if valid project structure
/// * `Err(ChiknError)` if invalid or missing required components
///
/// # Errors
/// - `NotFound`: Path doesn't exist
/// - `InvalidFormat`: Missing required files/folders
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use chickenscratch_core::core::project::format::validate_project_structure;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let project_path = Path::new("MyNovel.chikn");
/// validate_project_structure(project_path)?;
/// # Ok(()) }
/// ```
pub fn validate_project_structure(path: &Path) -> Result<(), ChiknError> {
    // Check if path exists
    if !path.exists() {
        return Err(ChiknError::NotFound(format!(
            "Project path does not exist: {}",
            path.display()
        )));
    }

    // Check if it's a directory
    if !path.is_dir() {
        return Err(ChiknError::InvalidFormat(format!(
            "Project path is not a directory: {}",
            path.display()
        )));
    }

    // Validate project.yaml exists
    let project_file = path.join(PROJECT_FILE);
    if !project_file.exists() {
        return Err(ChiknError::InvalidFormat(format!(
            "Missing required file: {}",
            PROJECT_FILE
        )));
    }

    // Validate required folders exist
    for folder in REQUIRED_FOLDERS {
        let folder_path = path.join(folder);
        if !folder_path.exists() {
            return Err(ChiknError::InvalidFormat(format!(
                "Missing required folder: {}",
                folder
            )));
        }
        if !folder_path.is_dir() {
            return Err(ChiknError::InvalidFormat(format!(
                "Expected folder but found file: {}",
                folder
            )));
        }
    }

    Ok(())
}

/// Gets the path to project.yaml file
///
/// # Arguments
/// * `project_path` - Root path of .chikn project
///
/// # Returns
/// PathBuf to project.yaml
pub fn get_project_file_path(project_path: &Path) -> PathBuf {
    project_path.join(PROJECT_FILE)
}

/// Gets the path to manuscript folder
///
/// # Arguments
/// * `project_path` - Root path of .chikn project
///
/// # Returns
/// PathBuf to manuscript/ folder
pub fn get_manuscript_path(project_path: &Path) -> PathBuf {
    project_path.join(MANUSCRIPT_FOLDER)
}

/// Gets the path to research folder
///
/// # Arguments
/// * `project_path` - Root path of .chikn project
///
/// # Returns
/// PathBuf to research/ folder
pub fn get_research_path(project_path: &Path) -> PathBuf {
    project_path.join(RESEARCH_FOLDER)
}

/// Gets the path to templates folder
///
/// # Arguments
/// * `project_path` - Root path of .chikn project
///
/// # Returns
/// PathBuf to templates/ folder
pub fn get_templates_path(project_path: &Path) -> PathBuf {
    project_path.join(TEMPLATES_FOLDER)
}

/// Gets the path to settings folder
///
/// # Arguments
/// * `project_path` - Root path of .chikn project
///
/// # Returns
/// PathBuf to settings/ folder
pub fn get_settings_path(project_path: &Path) -> PathBuf {
    project_path.join(SETTINGS_FOLDER)
}

/// Gets the document content file path (.md)
///
/// # Arguments
/// * `folder_path` - Folder containing document
/// * `doc_name` - Document name (without extension)
///
/// # Returns
/// PathBuf to document.md file
pub fn get_document_content_path(folder_path: &Path, doc_name: &str) -> PathBuf {
    folder_path.join(format!("{}.{}", doc_name, DOCUMENT_EXTENSION))
}

/// Gets the document metadata file path (.meta)
///
/// # Arguments
/// * `folder_path` - Folder containing document
/// * `doc_name` - Document name (without extension)
///
/// # Returns
/// PathBuf to document.meta file
pub fn get_document_meta_path(folder_path: &Path, doc_name: &str) -> PathBuf {
    folder_path.join(format!("{}.{}", doc_name, METADATA_EXTENSION))
}

/// Checks if a path has the .chikn extension
///
/// # Arguments
/// * `path` - Path to check
///
/// # Returns
/// true if path ends with .chikn extension
pub fn is_chikn_project(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext == PROJECT_EXTENSION)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper to create a valid .chikn project structure for testing
    fn create_test_project() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("TestProject.chikn");

        // Create directory
        fs::create_dir(&project_path).unwrap();

        // Create project.yaml
        fs::write(project_path.join(PROJECT_FILE), "name: Test Project\n").unwrap();

        // Create required folders
        for folder in REQUIRED_FOLDERS {
            fs::create_dir(project_path.join(folder)).unwrap();
        }

        (temp_dir, project_path)
    }

    #[test]
    fn test_validate_project_structure_valid() {
        let (_temp, project_path) = create_test_project();
        let result = validate_project_structure(&project_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_project_structure_missing_project_file() {
        let (_temp, project_path) = create_test_project();
        fs::remove_file(project_path.join(PROJECT_FILE)).unwrap();

        let result = validate_project_structure(&project_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_project_structure_missing_folder() {
        let (_temp, project_path) = create_test_project();
        fs::remove_dir(project_path.join(MANUSCRIPT_FOLDER)).unwrap();

        let result = validate_project_structure(&project_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_chikn_project() {
        let path = Path::new("MyProject.chikn");
        assert!(is_chikn_project(path));

        let not_chikn = Path::new("MyProject.txt");
        assert!(!is_chikn_project(not_chikn));
    }

    #[test]
    fn test_get_project_file_path() {
        let project_path = Path::new("MyProject.chikn");
        let result = get_project_file_path(project_path);
        assert_eq!(result, project_path.join("project.yaml"));
    }

    #[test]
    fn test_get_document_paths() {
        let folder = Path::new("manuscript");
        let doc_name = "chapter-01";

        let content_path = get_document_content_path(folder, doc_name);
        assert_eq!(content_path, folder.join("chapter-01.md"));

        let meta_path = get_document_meta_path(folder, doc_name);
        assert_eq!(meta_path, folder.join("chapter-01.meta"));
    }
}
