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
    let doc_name = slugify(&name);
    let now = Utc::now().to_rfc3339();

    let document = Document {
        id: doc_id.clone(),
        name: doc_name.clone(),
        path: format!("manuscript/{}.md", doc_name),
        content: format!("# {}\n\n", name), // Initialize with title
        parent_id,
        created: now.clone(),
        modified: now,
    };

    // Add to project documents
    project.documents.insert(doc_id.clone(), document.clone());

    // Write document to disk
    writer::write_project(&project)?;

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

    // Write to disk
    writer::write_project(&project)?;

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
    // Get document to find its name
    let document = project
        .documents
        .get(&document_id)
        .ok_or_else(|| ChiknError::NotFound(format!("Document not found: {}", document_id)))?;

    let doc_name = document.name.clone();

    // Remove from documents HashMap
    project.documents.remove(&document_id);

    // Delete from disk
    let project_path = Path::new(&project.path);
    writer::delete_document(project_path, &doc_name)?;

    // Save updated project
    writer::write_project(&project)?;

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
}
