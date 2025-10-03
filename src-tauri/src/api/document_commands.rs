//! Document API commands
//!
//! Tauri commands for document CRUD operations

use crate::models::Document;
use crate::utils::{error::ChiknError, fs::slugify};

/// Create a new document
#[tauri::command]
pub async fn create_document(
    project_path: String,
    parent_id: String,
    name: String,
) -> Result<Document, ChiknError> {
    // Generate document ID
    let id = uuid::Uuid::new_v4().to_string();

    // Determine file path based on parent
    let relative_path = if parent_id == "manuscript" {
        format!("manuscript/{}.md", slugify(&name))
    } else if parent_id == "research" {
        format!("research/{}.md", slugify(&name))
    } else {
        // For nested folders (Phase 1: simplified)
        format!("manuscript/{}.md", slugify(&name))
    };

    let document = Document {
        id: id.clone(),
        name: name.clone(),
        path: relative_path.clone(),
        content: String::new(),
        parent_id: Some(parent_id),
        created: chrono::Utc::now().to_rfc3339(),
        modified: chrono::Utc::now().to_rfc3339(),
    };

    // Write empty .md file
    let full_path = format!("{}/{}", project_path, relative_path);
    std::fs::write(full_path, "")?;

    Ok(document)
}

/// Update document content
#[tauri::command]
pub async fn update_document(
    project_path: String,
    document_path: String,
    content: String,
) -> Result<(), ChiknError> {
    // Write content to .md file
    let full_path = format!("{}/{}", project_path, document_path);
    std::fs::write(full_path, content)?;

    Ok(())
}

/// Delete a document
#[tauri::command]
pub async fn delete_document(
    project_path: String,
    document_path: String,
) -> Result<(), ChiknError> {
    // Delete .md file
    let full_path = format!("{}/{}", project_path, document_path);
    std::fs::remove_file(full_path)?;

    Ok(())
}
