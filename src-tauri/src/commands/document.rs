use chickenscratch_core::core::project::{reader, writer};
use chickenscratch_core::{ChiknError, Document};
use std::path::Path;

#[tauri::command]
pub fn get_document(project_path: String, doc_id: String) -> Result<Option<Document>, ChiknError> {
    let project = reader::read_project(Path::new(&project_path))?;
    Ok(project.documents.get(&doc_id).cloned())
}

#[tauri::command]
pub fn update_document_content(
    project_path: String,
    doc_id: String,
    content: String,
) -> Result<(), ChiknError> {
    let mut project = reader::read_project(Path::new(&project_path))?;
    if let Some(doc) = project.documents.get_mut(&doc_id) {
        doc.content = content;
        doc.modified = chrono::Utc::now().to_rfc3339();
        writer::write_project(&mut project)?;
        Ok(())
    } else {
        Err(ChiknError::NotFound(format!("Document not found: {}", doc_id)))
    }
}
