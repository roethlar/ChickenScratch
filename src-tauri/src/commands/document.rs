//! Thin Tauri wrappers over `chickenscratch_services::document` — transport only.
use chickenscratch_services::document as svc;
#[allow(unused_imports)]
use chickenscratch_services::document::*;
use chickenscratch_services::ChiknError;
use chickenscratch_services::{Document, Project};
use chickenscratch_services::{ProjectTokens, ProjectWriteLocks};
use tauri::State;

#[tauri::command]
pub fn get_document(project_path: String, doc_id: String) -> Result<Option<Document>, ChiknError> {
    svc::get_document(project_path, doc_id)
}

#[tauri::command]
pub fn update_document_content(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    doc_id: String,
    content: String,
) -> Result<(), ChiknError> {
    svc::update_document_content(project_path, &write_locks, &tokens, doc_id, content)
}

#[tauri::command]
pub fn add_comment(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    doc_id: String,
    comment_id: String,
    body: String,
    new_content: String,
) -> Result<Project, ChiknError> {
    svc::add_comment(
        project_path,
        &write_locks,
        &tokens,
        doc_id,
        comment_id,
        body,
        new_content,
    )
}

#[tauri::command]
pub fn update_comment(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    doc_id: String,
    comment_id: String,
    body: Option<String>,
    resolved: Option<bool>,
) -> Result<Project, ChiknError> {
    svc::update_comment(
        project_path,
        &write_locks,
        &tokens,
        doc_id,
        comment_id,
        body,
        resolved,
    )
}

#[tauri::command]
pub fn delete_comment(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    doc_id: String,
    comment_id: String,
    new_content: String,
) -> Result<Project, ChiknError> {
    svc::delete_comment(
        project_path,
        &write_locks,
        &tokens,
        doc_id,
        comment_id,
        new_content,
    )
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_document_metadata(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    doc_id: String,
    synopsis: Option<String>,
    label: Option<String>,
    status: Option<String>,
    keywords: Option<Vec<String>>,
    include_in_compile: Option<bool>,
    word_count_target: Option<u32>,
    compile_order: Option<i32>,
    fields: Option<FieldUpdates>,
) -> Result<Project, ChiknError> {
    svc::update_document_metadata(
        project_path,
        &write_locks,
        &tokens,
        doc_id,
        synopsis,
        label,
        status,
        keywords,
        include_in_compile,
        word_count_target,
        compile_order,
        fields,
    )
}

#[tauri::command]
pub fn rename_node(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    node_id: String,
    new_name: String,
) -> Result<Project, ChiknError> {
    svc::rename_node(project_path, &write_locks, &tokens, node_id, new_name)
}

#[tauri::command]
pub fn link_documents(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    doc_id_a: String,
    doc_id_b: String,
) -> Result<Project, ChiknError> {
    svc::link_documents(project_path, &write_locks, &tokens, doc_id_a, doc_id_b)
}

#[tauri::command]
pub fn create_document(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    name: String,
    parent_id: Option<String>,
) -> Result<Project, ChiknError> {
    svc::create_document(project_path, &write_locks, &tokens, name, parent_id)
}

#[tauri::command]
pub fn create_entity(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    name: String,
    kind: String,
) -> Result<Project, ChiknError> {
    svc::create_entity(project_path, &write_locks, &tokens, name, kind)
}

#[tauri::command]
pub fn create_folder(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    name: String,
    parent_id: Option<String>,
) -> Result<Project, ChiknError> {
    svc::create_folder(project_path, &write_locks, &tokens, name, parent_id)
}

#[tauri::command]
pub fn delete_node(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    node_id: String,
) -> Result<Project, ChiknError> {
    svc::delete_node(project_path, &write_locks, &tokens, node_id)
}

#[tauri::command]
pub fn move_node(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    node_id: String,
    new_parent_id: Option<String>,
    new_index: Option<usize>,
) -> Result<Project, ChiknError> {
    svc::move_node(
        project_path,
        &write_locks,
        &tokens,
        node_id,
        new_parent_id,
        new_index,
    )
}
