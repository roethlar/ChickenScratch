//! Document API commands
//!
//! Tauri commands for document CRUD operations.
//! Integrates with core::project writer module.

use std::path::Path;
use crate::core::project::writer;
use crate::models::{Document, Project};
use crate::utils::error::ChiknError;
use uuid::Uuid;
use chrono::Utc;

/// Creates a new document in the project.
///
/// # Arguments
/// * `project` - Current project state
/// * `name` - Document name (will be slugified for filename)
/// * `parent_id` - Optional parent folder ID (None for root/manuscript)
///
/// # Returns
/// * `Ok((Project, Document))` - Updated project and new document
/// * `Err(ChiknError)` on failure
///
/// # Example (from frontend)
/// ```javascript
/// const [updatedProject, newDoc] = await invoke('create_document', {
///   project,
///   name: 'Chapter 1',
///   parentId: null // or 'folder123'
/// });
/// ```
#[tauri::command]
pub async fn create_document(
    mut project: Project,
    name: String,
    parent_id: Option<String>,
) -> Result<(Project, Document), ChiknError> {
    // Generate new document
    let doc_id = Uuid::new_v4().to_string();
    let mut slug = slugify(&name);
    let now = Utc::now().to_rfc3339();

    // Ensure unique filename by checking existing paths
    let mut counter = 1;
    let original_slug = slug.clone();
    while project.documents.values().any(|d| d.path == format!("manuscript/{}.md", slug)) {
        slug = format!("{}-{}", original_slug, counter);
        counter += 1;
    }

    let document = Document {
        id: doc_id.clone(),
        name: name.clone(), // Preserve original display name
        path: format!("manuscript/{}.md", slug), // Use unique slug for filename
        content: format!("# {}\n\n", name), // Initialize with original title
        parent_id,
        created: now.clone(),
        modified: now,
    };

    // Add to project documents
    project.documents.insert(doc_id.clone(), document.clone());

    // Write document to disk (updates modified timestamp)
    writer::write_project(&mut project)?;

    Ok((project, document))
}

/// Updates an existing document's content.
///
/// # Arguments
/// * `project` - Current project state
/// * `document_id` - ID of document to update
/// * `content` - New content (Markdown)
///
/// # Returns
/// * `Ok(Project)` - Updated project
/// * `Err(ChiknError)` if document not found
///
/// # Example (from frontend)
/// ```javascript
/// const updatedProject = await invoke('update_document', {
///   project,
///   documentId: 'doc123',
///   content: '# Chapter 1\n\nOnce upon a time...'
/// });
/// ```
#[tauri::command]
pub async fn update_document(
    mut project: Project,
    document_id: String,
    content: String,
) -> Result<Project, ChiknError> {
    // Find document and update content
    let document = project
        .documents
        .get_mut(&document_id)
        .ok_or_else(|| ChiknError::NotFound(format!("Document not found: {}", document_id)))?;

    document.content = content;
    document.modified = Utc::now().to_rfc3339();

    // Write to disk (updates project modified timestamp)
    writer::write_project(&mut project)?;

    Ok(project)
}

/// Deletes a document from the project.
///
/// # Arguments
/// * `project` - Current project state
/// * `document_id` - ID of document to delete
///
/// # Returns
/// * `Ok(Project)` - Updated project
/// * `Err(ChiknError)` if document not found
///
/// # Example (from frontend)
/// ```javascript
/// const updatedProject = await invoke('delete_document', {
///   project,
///   documentId: 'doc123'
/// });
/// ```
#[tauri::command]
pub async fn delete_document(
    mut project: Project,
    document_id: String,
) -> Result<Project, ChiknError> {
    // Get document to find its path
    let document = project
        .documents
        .get(&document_id)
        .ok_or_else(|| ChiknError::NotFound(format!("Document not found: {}", document_id)))?;

    let doc_path = document.path.clone();

    // Remove from documents HashMap
    project.documents.remove(&document_id);

    // Delete from disk using document's path
    let project_path = Path::new(&project.path);
    writer::delete_document(project_path, &doc_path)?;

    // Save updated project (updates modified timestamp)
    writer::write_project(&mut project)?;

    Ok(project)
}

/// Gets a specific document by ID.
///
/// # Arguments
/// * `project` - Current project state
/// * `document_id` - ID of document to retrieve
///
/// # Returns
/// * `Ok(Document)` - The requested document
/// * `Err(ChiknError)` if not found
///
/// # Example (from frontend)
/// ```javascript
/// const doc = await invoke('get_document', {
///   project,
///   documentId: 'doc123'
/// });
/// console.log(doc.content);
/// ```
#[tauri::command]
pub async fn get_document(
    project: Project,
    document_id: String,
) -> Result<Document, ChiknError> {
    project
        .documents
        .get(&document_id)
        .cloned()
        .ok_or_else(|| ChiknError::NotFound(format!("Document not found: {}", document_id)))
}

/// Helper function to slugify a string for use as a filename.
///
/// Converts "My Chapter Name" to "my-chapter-name"
fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("My Chapter Name"), "my-chapter-name");
        assert_eq!(slugify("Chapter 1: The Beginning"), "chapter-1-the-beginning");
        assert_eq!(slugify("Hello World!!!"), "hello-world");
        assert_eq!(slugify("a--b--c"), "a-b-c");
    }


    #[tokio::test]
    async fn test_create_document_preserves_display_name() {
        use crate::core::project::writer::create_project;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("DisplayNameTest.chikn");
        let project = create_project(&project_path, "Test").unwrap();

        // Create document with special characters in name
        let (_updated_project, doc) = create_document(
            project,
            "Chapter 1: The Beginning!".to_string(),
            None
        ).await.unwrap();

        // Verify display name is preserved
        assert_eq!(doc.name, "Chapter 1: The Beginning!");
        // Verify filename is slugified
        assert_eq!(doc.path, "manuscript/chapter-1-the-beginning.md");
    }

    #[tokio::test]
    async fn test_create_document_handles_duplicate_slugs() {
        use crate::core::project::writer::create_project;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("DuplicateTest.chikn");
        let project = create_project(&project_path, "Test").unwrap();

        // Create first document
        let (project, doc1) = create_document(
            project,
            "Chapter 1".to_string(),
            None
        ).await.unwrap();

        assert_eq!(doc1.name, "Chapter 1");
        assert_eq!(doc1.path, "manuscript/chapter-1.md");

        // Create second document with name that slugifies the same
        let (project, doc2) = create_document(
            project,
            "Chapter 1!".to_string(),
            None
        ).await.unwrap();

        // Verify second doc gets unique filename
        assert_eq!(doc2.name, "Chapter 1!");
        assert_eq!(doc2.path, "manuscript/chapter-1-1.md"); // Appended -1

        // Create third duplicate
        let (_project, doc3) = create_document(
            project,
            "Chapter 1?".to_string(),
            None
        ).await.unwrap();

        assert_eq!(doc3.name, "Chapter 1?");
        assert_eq!(doc3.path, "manuscript/chapter-1-2.md"); // Appended -2
    }

    #[tokio::test]
    async fn test_display_name_survives_round_trip() {
        use crate::core::project::writer::create_project;
        use crate::core::project::reader::read_project;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("RoundTripName.chikn");
        let project = create_project(&project_path, "Test").unwrap();

        // Create document with special name
        let (mut project, doc) = create_document(
            project,
            "Chapter 1: The Beginning!".to_string(),
            None
        ).await.unwrap();

        let doc_id = doc.id.clone();

        // Save project
        crate::core::project::writer::write_project(&mut project).unwrap();

        // Reload project from disk
        let loaded_project = read_project(&project_path).unwrap();

        // Verify display name is preserved
        let loaded_doc = loaded_project.documents.get(&doc_id).unwrap();
        assert_eq!(loaded_doc.name, "Chapter 1: The Beginning!");
        assert_ne!(loaded_doc.name, "chapter-1-the-beginning"); // Not slugified!
    }
}
