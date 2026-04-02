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

use chrono::Utc;
use std::fs;
use std::path::Path;

use super::format::{
    get_document_meta_path, get_manuscript_path, get_project_file_path, get_research_path,
    get_settings_path, get_templates_path,
};
use super::reader::{DocumentMetadata, ProjectMetadata};
use crate::models::Project;
use crate::utils::error::ChiknError;

/// Writes a Project to disk as a .chikn project.
///
/// # Arguments
/// * `project` - Mutable reference to project (modified timestamp will be updated)
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
/// write_project(&mut project)?;
/// ```
pub fn write_project(project: &mut Project) -> Result<(), ChiknError> {
    let project_path = Path::new(&project.path);

    // Update modified timestamp
    project.modified = Utc::now().to_rfc3339();

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
        modified: project.modified.clone(), // Use already-updated timestamp
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
fn write_document(
    project_path: &Path,
    document: &crate::models::Document,
) -> Result<(), ChiknError> {
    // Use document.path to determine actual location
    let doc_path = Path::new(&document.path);

    // Validate path stays within project (security)
    if doc_path.is_absolute() || document.path.contains("..") {
        return Err(ChiknError::InvalidFormat(format!(
            "Document path must be relative and within project: {}",
            document.path
        )));
    }

    // Resolve full path from project root
    let full_content_path = project_path.join(&document.path);

    // Create parent directories if needed
    if let Some(parent) = full_content_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write content (.md file)
    fs::write(&full_content_path, &document.content)?;

    // Write metadata (.meta file) in same directory
    let doc_name = full_content_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            ChiknError::InvalidFormat(format!("Invalid document path: {}", document.path))
        })?;

    let folder_path = full_content_path.parent().ok_or_else(|| {
        ChiknError::InvalidFormat(format!("Document has no parent: {}", document.path))
    })?;

    let meta_path = get_document_meta_path(folder_path, doc_name);
    let metadata = DocumentMetadata {
        id: document.id.clone(),
        name: Some(document.name.clone()), // Save display name
        created: document.created.clone(),
        modified: Utc::now().to_rfc3339(),
        parent_id: document.parent_id.clone(),
        label: None,
        status: None,
        keywords: None,
        synopsis: None,
        section_type: None,
        include_in_compile: None,
        scrivener_uuid: None,
    };

    let meta_content = serde_yaml::to_string(&metadata)?;
    fs::write(&meta_path, meta_content)?;

    Ok(())
}

/// Deletes a document from disk using its stored path
///
/// # Arguments
/// * `project_path` - Root path of project
/// * `document_path` - Document's relative path (from Document.path)
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(ChiknError)` if file doesn't exist or can't be deleted
pub fn delete_document(project_path: &Path, document_path: &str) -> Result<(), ChiknError> {
    // Validate path (security)
    if Path::new(document_path).is_absolute() || document_path.contains("..") {
        return Err(ChiknError::InvalidFormat(format!(
            "Document path must be relative: {}",
            document_path
        )));
    }

    // Resolve full paths
    let full_content_path = project_path.join(document_path);

    // Delete .md file
    if full_content_path.exists() {
        fs::remove_file(&full_content_path)?;
    }

    // Delete .meta file
    let doc_name = full_content_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            ChiknError::InvalidFormat(format!("Invalid document path: {}", document_path))
        })?;

    let folder_path = full_content_path.parent().ok_or_else(|| {
        ChiknError::InvalidFormat(format!("Document has no parent: {}", document_path))
    })?;

    let meta_path = get_document_meta_path(folder_path, doc_name);
    if meta_path.exists() {
        fs::remove_file(&meta_path)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::project::reader::read_project;
    use crate::models::{Document, TreeNode};
    use tempfile::TempDir;

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
        let result = write_project(&mut project);
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

        original_project
            .documents
            .insert(doc.id.clone(), doc.clone());
        original_project.hierarchy.push(TreeNode::Document {
            id: doc.id.clone(),
            name: doc.name.clone(),
            path: doc.path.clone(),
        });

        write_project(&mut original_project).unwrap();

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
        write_project(&mut project).unwrap();

        // Verify file exists
        let content_path = get_manuscript_path(&project_path).join("to-delete.md");
        assert!(content_path.exists());

        // Delete document using its path
        delete_document(&project_path, "manuscript/to-delete.md").unwrap();

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

    #[test]
    fn test_write_nested_document() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("NestedWrite.chikn");

        let mut project = create_project(&project_path, "Nested Write Test").unwrap();

        // Create document with nested path
        let doc = Document {
            id: "nested1".to_string(),
            name: "Chapter 1".to_string(),
            path: "manuscript/part-one/chapter-01.md".to_string(),
            content: "# Nested Chapter\n\nContent".to_string(),
            parent_id: None,
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
        };

        project.documents.insert(doc.id.clone(), doc.clone());
        write_project(&mut project).unwrap();

        // Verify nested file exists
        let nested_path = project_path
            .join("manuscript")
            .join("part-one")
            .join("chapter-01.md");
        assert!(nested_path.exists());

        // Verify content
        let content = fs::read_to_string(&nested_path).unwrap();
        assert_eq!(content, "# Nested Chapter\n\nContent");

        // Verify metadata file
        let meta_path = project_path
            .join("manuscript")
            .join("part-one")
            .join("chapter-01.meta");
        assert!(meta_path.exists());
    }

    #[test]
    fn test_write_research_document() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("ResearchWrite.chikn");

        let mut project = create_project(&project_path, "Research Test").unwrap();

        // Create document in research folder
        let doc = Document {
            id: "research1".to_string(),
            name: "Character Notes".to_string(),
            path: "research/characters.md".to_string(),
            content: "# Characters\n\nJohn Doe".to_string(),
            parent_id: None,
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
        };

        project.documents.insert(doc.id.clone(), doc);
        write_project(&mut project).unwrap();

        // Verify file in research folder
        let research_path = project_path.join("research").join("characters.md");
        assert!(research_path.exists());
    }

    #[test]
    fn test_write_document_rejects_absolute_path() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("SecurityTest.chikn");

        let mut project = create_project(&project_path, "Security Test").unwrap();

        // Try to create document with absolute path
        let doc = Document {
            id: "bad1".to_string(),
            name: "Bad Document".to_string(),
            path: "/etc/passwd".to_string(), // Absolute path - security issue!
            content: "Evil content".to_string(),
            parent_id: None,
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
        };

        project.documents.insert(doc.id.clone(), doc);
        let result = write_project(&mut project);

        // Should fail with InvalidFormat error
        assert!(result.is_err());
    }

    #[test]
    fn test_write_document_rejects_parent_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("TraversalTest.chikn");

        let mut project = create_project(&project_path, "Traversal Test").unwrap();

        // Try to escape project directory
        let doc = Document {
            id: "bad2".to_string(),
            name: "Bad Document".to_string(),
            path: "../../../etc/passwd".to_string(), // Directory traversal!
            content: "Evil content".to_string(),
            parent_id: None,
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
        };

        project.documents.insert(doc.id.clone(), doc);
        let result = write_project(&mut project);

        // Should fail with InvalidFormat error
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_nested_document() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("DeleteNested.chikn");

        let mut project = create_project(&project_path, "Delete Nested Test").unwrap();

        // Create nested document
        let doc = Document {
            id: "nested1".to_string(),
            name: "Nested Chapter".to_string(),
            path: "manuscript/part-one/chapter.md".to_string(),
            content: "Content".to_string(),
            parent_id: None,
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
        };

        project.documents.insert(doc.id.clone(), doc.clone());
        write_project(&mut project).unwrap();

        // Verify it exists
        let nested_path = project_path
            .join("manuscript")
            .join("part-one")
            .join("chapter.md");
        assert!(nested_path.exists());

        // Delete it
        delete_document(&project_path, "manuscript/part-one/chapter.md").unwrap();

        // Verify it's gone
        assert!(!nested_path.exists());
    }

    #[test]
    fn test_modified_timestamp_updates() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("TimestampTest.chikn");

        let mut project = create_project(&project_path, "Timestamp Test").unwrap();
        let original_modified = project.modified.clone();

        // Wait a bit to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Write project
        write_project(&mut project).unwrap();

        // Verify modified timestamp was updated
        assert_ne!(project.modified, original_modified);
    }
}
