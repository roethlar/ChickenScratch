//! # Project Reader
//!
//! Reads .chikn project files from disk into Project structs.
//!
//! ## Responsibilities
//! - Load project.yaml and parse into Project
//! - Read all document content (.md files)
//! - Build document hierarchy from filesystem
//! - Validate project structure
//!
//! ## Example
//! ```rust
//! use std::path::Path;
//! use crate::core::project::reader::read_project;
//!
//! let project = read_project(Path::new("MyNovel.chikn"))?;
//! println!("Loaded project: {}", project.name);
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[cfg(test)]
use std::path::PathBuf;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{Document, Project, TreeNode};
use crate::utils::error::ChiknError;
use super::format::{
    validate_project_structure,
    get_project_file_path,
    get_manuscript_path,
    get_research_path,
    get_document_meta_path,
    DOCUMENT_EXTENSION,
};

/// Project metadata structure as stored in project.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// Project unique ID
    pub id: String,

    /// Project name
    pub name: String,

    /// Document hierarchy (root nodes)
    pub hierarchy: Vec<TreeNode>,

    /// Project creation timestamp
    pub created: String,

    /// Last modified timestamp
    pub modified: String,
}

/// Document metadata structure as stored in .meta files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Document unique ID
    #[serde(default = "generate_id")]
    pub id: String,

    /// Human-readable display name (e.g., "Chapter 1")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Creation timestamp
    #[serde(default = "current_timestamp")]
    pub created: String,

    /// Last modified timestamp
    #[serde(default = "current_timestamp")]
    pub modified: String,

    /// Parent ID (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,

    // Scrivener metadata (Phase 2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub synopsis: Option<String>,
}

/// Helper function to generate a new UUID
fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

/// Helper function to get current timestamp
fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}

/// Reads a .chikn project from disk.
///
/// # Arguments
/// * `path` - Path to .chikn project directory
///
/// # Returns
/// Complete Project struct with all documents loaded
///
/// # Errors
/// - `NotFound`: Project path doesn't exist
/// - `InvalidFormat`: Missing required files/folders
/// - `Io`: File system errors during reading
/// - `Serialization`: YAML parsing errors
///
/// # Example
/// ```rust
/// let project = read_project(Path::new("MyNovel.chikn"))?;
/// ```
pub fn read_project(path: &Path) -> Result<Project, ChiknError> {
    // Validate project structure
    validate_project_structure(path)?;

    // Read project.yaml
    let metadata = read_project_metadata(path)?;

    // Read all documents from manuscript and research folders
    let documents = read_all_documents(path)?;

    Ok(Project {
        id: metadata.id,
        name: metadata.name,
        path: path.to_string_lossy().to_string(),
        hierarchy: metadata.hierarchy,
        documents,
        created: metadata.created,
        modified: metadata.modified,
    })
}

/// Reads project.yaml and parses into ProjectMetadata
fn read_project_metadata(path: &Path) -> Result<ProjectMetadata, ChiknError> {
    let project_file = get_project_file_path(path);

    let content = fs::read_to_string(&project_file)
        .map_err(|e| ChiknError::Io(e))?;

    let metadata: ProjectMetadata = serde_yaml::from_str(&content)
        .map_err(|e| ChiknError::Serialization(e))?;

    Ok(metadata)
}

/// Reads all documents from manuscript and research folders
fn read_all_documents(project_path: &Path) -> Result<HashMap<String, Document>, ChiknError> {
    let mut documents = HashMap::new();

    // Read from manuscript folder (recursively)
    let manuscript_path = get_manuscript_path(project_path);
    if manuscript_path.exists() {
        read_documents_from_folder(&manuscript_path, project_path, &mut documents)?;
    }

    // Read from research folder (recursively)
    let research_path = get_research_path(project_path);
    if research_path.exists() {
        read_documents_from_folder(&research_path, project_path, &mut documents)?;
    }

    Ok(documents)
}

/// Reads all documents from a folder recursively
fn read_documents_from_folder(
    folder_path: &Path,
    project_path: &Path,
    documents: &mut HashMap<String, Document>,
) -> Result<(), ChiknError> {
    // Iterate through all entries in the folder
    for entry in fs::read_dir(folder_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            // Process .md files
            if let Some(extension) = path.extension() {
                if extension == DOCUMENT_EXTENSION {
                    let doc = read_document(&path, project_path)?;
                    documents.insert(doc.id.clone(), doc);
                }
            }
        } else if path.is_dir() {
            // Recursively process subdirectories
            read_documents_from_folder(&path, project_path, documents)?;
        }
    }

    Ok(())
}

/// Reads a single document (content + metadata)
fn read_document(content_path: &Path, project_path: &Path) -> Result<Document, ChiknError> {
    // Read content (.md file)
    let content = fs::read_to_string(content_path)?;

    // Get filename stem (used as fallback if metadata missing)
    let file_stem = content_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| ChiknError::InvalidFormat(
            format!("Invalid document filename: {}", content_path.display())
        ))?;

    // Read metadata (.meta file) if it exists
    let folder_path = content_path
        .parent()
        .ok_or_else(|| ChiknError::InvalidFormat(
            format!("Document has no parent folder: {}", content_path.display())
        ))?;

    let meta_path = get_document_meta_path(folder_path, file_stem);
    let metadata = if meta_path.exists() {
        let meta_content = fs::read_to_string(&meta_path)?;
        serde_yaml::from_str::<DocumentMetadata>(&meta_content)?
    } else {
        // Create default metadata if .meta file doesn't exist
        DocumentMetadata {
            id: generate_id(),
            name: None,
            created: current_timestamp(),
            modified: current_timestamp(),
            parent_id: None,
            label: None,
            status: None,
            keywords: None,
            synopsis: None,
        }
    };

    // Compute relative path from project root
    let relative_path = content_path
        .strip_prefix(project_path)
        .map_err(|_| ChiknError::InvalidFormat(
            format!("Document path not within project: {}", content_path.display())
        ))?
        .to_string_lossy()
        .to_string();

    // Use display name from metadata if available, otherwise use filename
    let display_name = metadata.name.unwrap_or_else(|| file_stem.to_string());

    Ok(Document {
        id: metadata.id,
        name: display_name,
        path: relative_path,
        content,
        parent_id: metadata.parent_id,
        created: metadata.created,
        modified: metadata.modified,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    use crate::core::project::format::{
        PROJECT_FILE,
        MANUSCRIPT_FOLDER,
        RESEARCH_FOLDER,
        TEMPLATES_FOLDER,
        SETTINGS_FOLDER,
    };

    /// Helper to create a test .chikn project
    fn create_test_project() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("TestProject.chikn");

        // Create directory structure
        fs::create_dir(&project_path).unwrap();
        fs::create_dir(project_path.join(MANUSCRIPT_FOLDER)).unwrap();
        fs::create_dir(project_path.join(RESEARCH_FOLDER)).unwrap();
        fs::create_dir(project_path.join(TEMPLATES_FOLDER)).unwrap();
        fs::create_dir(project_path.join(SETTINGS_FOLDER)).unwrap();

        // Create project.yaml
        let project_yaml = format!(
            r#"id: "{}"
name: "Test Project"
created: "2025-01-01T00:00:00Z"
modified: "2025-01-01T00:00:00Z"
hierarchy:
  - type: Document
    id: "doc1"
    name: "Chapter 1"
    path: "manuscript/chapter-01.md"
"#,
            generate_id()
        );
        fs::write(project_path.join(PROJECT_FILE), project_yaml).unwrap();

        // Create a test document
        let doc_path = project_path.join(MANUSCRIPT_FOLDER).join("chapter-01.md");
        fs::write(&doc_path, "# Chapter 1\n\nOnce upon a time...").unwrap();

        // Create metadata file
        let meta_yaml = format!(
            r#"id: "doc1"
created: "2025-01-01T00:00:00Z"
modified: "2025-01-01T00:00:00Z"
"#
        );
        fs::write(
            project_path.join(MANUSCRIPT_FOLDER).join("chapter-01.meta"),
            meta_yaml
        ).unwrap();

        (temp_dir, project_path)
    }

    #[test]
    fn test_read_project_success() {
        let (_temp, project_path) = create_test_project();
        let result = read_project(&project_path);

        assert!(result.is_ok());
        let project = result.unwrap();
        assert_eq!(project.name, "Test Project");
        assert_eq!(project.hierarchy.len(), 1);
        assert_eq!(project.documents.len(), 1);
    }

    #[test]
    fn test_read_project_metadata() {
        let (_temp, project_path) = create_test_project();
        let result = read_project_metadata(&project_path);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.name, "Test Project");
    }

    #[test]
    fn test_read_document() {
        let (_temp, project_path) = create_test_project();
        let doc_path = project_path.join(MANUSCRIPT_FOLDER).join("chapter-01.md");

        let result = read_document(&doc_path, &project_path);
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert_eq!(doc.name, "chapter-01");
        assert!(doc.content.contains("Once upon a time"));
        assert!(doc.path.starts_with("manuscript/"));
    }

    #[test]
    fn test_read_all_documents() {
        let (_temp, project_path) = create_test_project();
        let result = read_all_documents(&project_path);

        assert!(result.is_ok());
        let documents = result.unwrap();
        assert_eq!(documents.len(), 1);
        assert!(documents.contains_key("doc1"));
    }

    #[test]
    fn test_read_nested_documents() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("NestedTest.chikn");

        // Create project structure
        fs::create_dir(&project_path).unwrap();
        fs::create_dir(project_path.join(MANUSCRIPT_FOLDER)).unwrap();
        fs::create_dir(project_path.join(RESEARCH_FOLDER)).unwrap();
        fs::create_dir(project_path.join(TEMPLATES_FOLDER)).unwrap();
        fs::create_dir(project_path.join(SETTINGS_FOLDER)).unwrap();

        // Create nested folder structure
        let nested_folder = project_path.join(MANUSCRIPT_FOLDER).join("part-one");
        fs::create_dir(&nested_folder).unwrap();

        // Create document in nested folder
        fs::write(
            nested_folder.join("chapter-01.md"),
            "# Nested Chapter\n\nContent in subfolder"
        ).unwrap();

        // Create metadata
        let meta_yaml = r#"id: "nested-doc1"
created: "2025-01-01T00:00:00Z"
modified: "2025-01-01T00:00:00Z"
"#;
        fs::write(nested_folder.join("chapter-01.meta"), meta_yaml).unwrap();

        // Create project.yaml
        let project_yaml = format!(
            r#"id: "{}"
name: "Nested Test"
created: "2025-01-01T00:00:00Z"
modified: "2025-01-01T00:00:00Z"
hierarchy: []
"#,
            generate_id()
        );
        fs::write(project_path.join(PROJECT_FILE), project_yaml).unwrap();

        // Read all documents (should find nested)
        let documents = read_all_documents(&project_path).unwrap();

        assert_eq!(documents.len(), 1);
        assert!(documents.contains_key("nested-doc1"));

        let doc = documents.get("nested-doc1").unwrap();
        assert_eq!(doc.name, "chapter-01");
        // Verify relative path
        assert_eq!(doc.path, "manuscript/part-one/chapter-01.md");
    }

    #[test]
    fn test_read_document_relative_path() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("PathTest.chikn");

        // Create structure
        fs::create_dir(&project_path).unwrap();
        let nested = project_path.join("manuscript").join("subfolder");
        fs::create_dir_all(&nested).unwrap();

        // Create document
        let doc_path = nested.join("test.md");
        fs::write(&doc_path, "Test content").unwrap();

        // Create metadata
        fs::write(
            nested.join("test.meta"),
            "id: \"test-id\"\ncreated: \"2025-01-01T00:00:00Z\"\nmodified: \"2025-01-01T00:00:00Z\"\n"
        ).unwrap();

        // Read document
        let doc = read_document(&doc_path, &project_path).unwrap();

        // Verify path is relative, not absolute
        assert!(!doc.path.starts_with("/"));
        assert_eq!(doc.path, "manuscript/subfolder/test.md");
    }
}
