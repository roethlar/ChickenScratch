//! # Project Writer
//!
//! Writes Project structs to disk as .chikn format.
//!
//! ## Responsibilities
//! - Serialize Project to project.yaml
//! - Write all document content to .md files
//! - Write document metadata to .meta files
//! - Atomic writes (temp file + rename)
//! - Create required directory structure
//!
//! ## Example
//! ```rust
//! use crate::core::project::writer::write_project;
//!
//! let project = /* ... */;
//! write_project(&project)?;
//! println!("Project saved successfully");
//! ```

use std::fs;
use std::path::Path;
use chrono::Utc;

use crate::models::Project;
use crate::utils::error::ChiknError;
use super::format::{
    get_project_file_path,
    get_manuscript_path,
    get_research_path,
    get_templates_path,
    get_settings_path,
    get_document_meta_path,
};
use super::reader::{ProjectMetadata, DocumentMetadata};

/// Writes a Project to disk as a .chikn project.
///
/// # Arguments
/// * `project` - Project to write
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(ChiknError)` on failure
///
/// # Errors
/// - `Io`: File system errors during writing
/// - `Serialization`: YAML serialization errors
///
/// # Example
/// ```rust
/// write_project(&project)?;
/// ```
pub fn write_project(project: &Project) -> Result<(), ChiknError> {
    let project_path = Path::new(&project.path);

    // Create directory structure if it doesn't exist
    create_project_structure(project_path)?;

    // Write project.yaml
    write_project_metadata(project)?;

    // Write all documents
    write_all_documents(project)?;

    Ok(())
}

/// Creates a new .chikn project on disk with required folder structure.
///
/// # Arguments
/// * `path` - Path where project should be created
/// * `name` - Project name
///
/// # Returns
/// * `Ok(Project)` - Newly created project struct
/// * `Err(ChiknError)` on failure
///
/// # Errors
/// - `Io`: File system errors
/// - `InvalidFormat`: Path already exists
///
/// # Example
/// ```rust
/// let project = create_project(Path::new("MyNovel.chikn"), "My Novel")?;
/// ```
pub fn create_project(path: &Path, name: &str) -> Result<Project, ChiknError> {
    // Check if path already exists
    if path.exists() {
        return Err(ChiknError::InvalidFormat(format!(
            "Project already exists: {}",
            path.display()
        )));
    }

    // Create directory structure
    create_project_structure(path)?;

    // Create initial project
    let now = Utc::now().to_rfc3339();
    let project = Project {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        path: path.to_string_lossy().to_string(),
        hierarchy: Vec::new(),
        documents: std::collections::HashMap::new(),
        created: now.clone(),
        modified: now,
    };

    // Write initial project.yaml
    write_project_metadata(&project)?;

    Ok(project)
}

/// Creates the required folder structure for a .chikn project
fn create_project_structure(path: &Path) -> Result<(), ChiknError> {
    // Create root directory
    fs::create_dir_all(path)?;

    // Create required folders
    fs::create_dir_all(get_manuscript_path(path))?;
    fs::create_dir_all(get_research_path(path))?;
    fs::create_dir_all(get_templates_path(path))?;
    fs::create_dir_all(get_settings_path(path))?;

    Ok(())
}

/// Writes project.yaml metadata
fn write_project_metadata(project: &Project) -> Result<(), ChiknError> {
    let project_path = Path::new(&project.path);
    let project_file = get_project_file_path(project_path);

    let metadata = ProjectMetadata {
        id: project.id.clone(),
        name: project.name.clone(),
        hierarchy: project.hierarchy.clone(),
        created: project.created.clone(),
        modified: Utc::now().to_rfc3339(), // Update modified time
    };

    let yaml_content = serde_yaml::to_string(&metadata)?;

    // Atomic write: write to temp file, then rename
    let temp_file = project_file.with_extension("yaml.tmp");
    fs::write(&temp_file, yaml_content)?;
    fs::rename(&temp_file, &project_file)?;

    Ok(())
}

/// Writes all documents to their respective folders
fn write_all_documents(project: &Project) -> Result<(), ChiknError> {
    let project_path = Path::new(&project.path);

    for (_, document) in &project.documents {
        write_document(project_path, document)?;
    }

    Ok(())
}

/// Writes a single document (content + metadata)
fn write_document(project_path: &Path, document: &crate::models::Document) -> Result<(), ChiknError> {
    // Determine which folder based on document path
    // For now, default to manuscript (Phase 2 will handle research/templates)
    let folder_path = get_manuscript_path(project_path);

    // Write content (.md file)
    let content_path = folder_path.join(format!("{}.md", document.name));
    fs::write(&content_path, &document.content)?;

    // Write metadata (.meta file)
    let meta_path = get_document_meta_path(&folder_path, &document.name);
    let metadata = DocumentMetadata {
        id: document.id.clone(),
        created: document.created.clone(),
        modified: Utc::now().to_rfc3339(), // Update modified time
        parent_id: document.parent_id.clone(),
        label: None,
        status: None,
        keywords: None,
        synopsis: None,
    };

    let meta_content = serde_yaml::to_string(&metadata)?;
    fs::write(&meta_path, meta_content)?;

    Ok(())
}

/// Deletes a document from disk
///
/// # Arguments
/// * `project_path` - Root path of project
/// * `document_name` - Name of document to delete
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(ChiknError)` if file doesn't exist or can't be deleted
pub fn delete_document(project_path: &Path, document_name: &str) -> Result<(), ChiknError> {
    let folder_path = get_manuscript_path(project_path);

    // Delete .md file
    let content_path = folder_path.join(format!("{}.md", document_name));
    if content_path.exists() {
        fs::remove_file(&content_path)?;
    }

    // Delete .meta file
    let meta_path = get_document_meta_path(&folder_path, document_name);
    if meta_path.exists() {
        fs::remove_file(&meta_path)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::models::{Document, TreeNode};
    use crate::core::project::reader::read_project;

    #[test]
    fn test_create_project() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("NewProject.chikn");

        let result = create_project(&project_path, "New Project");
        assert!(result.is_ok());

        let project = result.unwrap();
        assert_eq!(project.name, "New Project");
        assert!(project_path.exists());
        assert!(get_manuscript_path(&project_path).exists());
        assert!(get_research_path(&project_path).exists());
    }

    #[test]
    fn test_create_project_already_exists() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("ExistingProject.chikn");

        // Create once
        create_project(&project_path, "Test").unwrap();

        // Try to create again - should fail
        let result = create_project(&project_path, "Test");
        assert!(result.is_err());
    }

    #[test]
    fn test_write_project() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("TestProject.chikn");

        let mut project = create_project(&project_path, "Test Project").unwrap();

        // Add a document
        let doc = Document {
            id: "doc1".to_string(),
            name: "chapter-01".to_string(),
            path: "manuscript/chapter-01.md".to_string(),
            content: "# Chapter 1\n\nTest content".to_string(),
            parent_id: None,
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
        };

        project.documents.insert(doc.id.clone(), doc.clone());
        project.hierarchy.push(TreeNode::Document {
            id: doc.id.clone(),
            name: doc.name.clone(),
            path: doc.path.clone(),
        });

        // Write project
        let result = write_project(&project);
        assert!(result.is_ok());

        // Verify files exist
        let content_path = get_manuscript_path(&project_path).join("chapter-01.md");
        assert!(content_path.exists());

        let meta_path = get_manuscript_path(&project_path).join("chapter-01.meta");
        assert!(meta_path.exists());
    }

    #[test]
    fn test_round_trip_write_read() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("RoundTripProject.chikn");

        // Create and write project
        let mut original_project = create_project(&project_path, "Round Trip Test").unwrap();

        let doc = Document {
            id: "doc1".to_string(),
            name: "test-document".to_string(),
            path: "manuscript/test-document.md".to_string(),
            content: "Test content for round trip".to_string(),
            parent_id: None,
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
        };

        original_project.documents.insert(doc.id.clone(), doc.clone());
        original_project.hierarchy.push(TreeNode::Document {
            id: doc.id.clone(),
            name: doc.name.clone(),
            path: doc.path.clone(),
        });

        write_project(&original_project).unwrap();

        // Read project back
        let loaded_project = read_project(&project_path).unwrap();

        // Verify data matches
        assert_eq!(loaded_project.name, original_project.name);
        assert_eq!(loaded_project.id, original_project.id);
        assert_eq!(loaded_project.documents.len(), 1);
        assert!(loaded_project.documents.contains_key("doc1"));

        let loaded_doc = loaded_project.documents.get("doc1").unwrap();
        assert_eq!(loaded_doc.name, "test-document");
        assert_eq!(loaded_doc.content, "Test content for round trip");
    }

    #[test]
    fn test_delete_document() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("DeleteTest.chikn");

        let mut project = create_project(&project_path, "Delete Test").unwrap();

        // Create a document
        let doc = Document {
            id: "doc1".to_string(),
            name: "to-delete".to_string(),
            path: "manuscript/to-delete.md".to_string(),
            content: "This will be deleted".to_string(),
            parent_id: None,
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
        };

        project.documents.insert(doc.id.clone(), doc.clone());
        write_project(&project).unwrap();

        // Verify file exists
        let content_path = get_manuscript_path(&project_path).join("to-delete.md");
        assert!(content_path.exists());

        // Delete document
        delete_document(&project_path, "to-delete").unwrap();

        // Verify files are gone
        assert!(!content_path.exists());
        let meta_path = get_manuscript_path(&project_path).join("to-delete.meta");
        assert!(!meta_path.exists());
    }

    #[test]
    fn test_write_project_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("MetadataTest.chikn");

        let project = create_project(&project_path, "Metadata Test").unwrap();
        let result = write_project_metadata(&project);

        assert!(result.is_ok());

        let project_file = get_project_file_path(&project_path);
        assert!(project_file.exists());

        // Verify YAML is valid
        let content = fs::read_to_string(&project_file).unwrap();
        assert!(content.contains("name:") && content.contains("Metadata Test"));
    }
}
