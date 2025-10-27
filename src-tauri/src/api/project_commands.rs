//! Project API commands
//!
//! Tauri commands for project CRUD operations.
//! Integrates with core::project reader/writer modules.

use crate::core::project::{hierarchy, reader, writer};
use crate::models::{Project, TreeNode};
use crate::utils::error::ChiknError;
use std::path::Path;

/// Creates a new .chikn project.
///
/// # Arguments
/// * `name` - Project name
/// * `path` - Parent directory path where .chikn folder will be created
///
/// # Returns
/// * `Ok(String)` - Full path to created project
/// * `Err(ChiknError)` on failure
///
/// # Example (from frontend)
/// ```javascript
/// const projectPath = await invoke('create_project', {
///   name: 'My Novel',
///   path: '/Users/john/Documents'
/// });
/// // Returns: '/Users/john/Documents/My Novel.chikn'
/// ```
#[tauri::command]
pub async fn create_project(name: String, path: String) -> Result<String, ChiknError> {
    // Construct full project path
    let project_path = Path::new(&path).join(format!("{}.chikn", name));

    // Use writer module to create project
    let project = writer::create_project(&project_path, &name)?;

    Ok(project.path)
}

/// Loads an existing .chikn project from disk.
///
/// # Arguments
/// * `path` - Full path to .chikn project directory
///
/// # Returns
/// * `Ok(Project)` - Complete project with all documents loaded
/// * `Err(ChiknError)` if project doesn't exist or is invalid
///
/// # Example (from frontend)
/// ```javascript
/// const project = await invoke('load_project', {
///   path: '/Users/john/Documents/My Novel.chikn'
/// });
/// console.log(project.name); // "My Novel"
/// console.log(project.documents); // HashMap of all documents
/// ```
#[tauri::command]
pub async fn load_project(path: String) -> Result<Project, ChiknError> {
    let project_path = Path::new(&path);

    // Use reader module to load project
    reader::read_project(project_path)
}

/// Saves project metadata and hierarchy to disk.
///
/// Note: This saves project.yaml but does NOT save document content.
/// Use save_document() to save individual document changes.
///
/// # Arguments
/// * `project` - Project with updated metadata/hierarchy
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(ChiknError)` on failure
///
/// # Example (from frontend)
/// ```javascript
/// project.hierarchy.push(newDocument);
/// await invoke('save_project', { project });
/// ```
#[tauri::command]
pub async fn save_project(mut project: Project) -> Result<Project, ChiknError> {
    // Use writer module to save project (updates modified timestamp)
    writer::write_project(&mut project)?;
    Ok(project)
}

/// Adds a document to the project hierarchy at root level.
///
/// # Arguments
/// * `project` - Current project state
/// * `node` - TreeNode to add (Document or Folder)
///
/// # Returns
/// * `Ok(Project)` - Updated project with new node
/// * `Err(ChiknError)` on failure
///
/// # Example (from frontend)
/// ```javascript
/// const newDoc = {
///   type: 'Document',
///   id: 'doc123',
///   name: 'Chapter 1',
///   path: 'manuscript/chapter-01.md'
/// };
/// const updatedProject = await invoke('add_to_hierarchy', {
///   project,
///   node: newDoc
/// });
/// ```
#[tauri::command]
pub async fn add_to_hierarchy(mut project: Project, node: TreeNode) -> Result<Project, ChiknError> {
    // Use hierarchy module to add node
    hierarchy::add_document_to_hierarchy(&mut project.hierarchy, node);

    // Save updated project (updates modified timestamp)
    writer::write_project(&mut project)?;

    Ok(project)
}

/// Adds a document as a child of a specific folder.
///
/// # Arguments
/// * `project` - Current project state
/// * `parent_id` - ID of parent folder
/// * `node` - TreeNode to add as child
///
/// # Returns
/// * `Ok(Project)` - Updated project
/// * `Err(ChiknError)` if parent not found or is a document
///
/// # Example (from frontend)
/// ```javascript
/// const updatedProject = await invoke('add_to_folder', {
///   project,
///   parentId: 'folder123',
///   node: newDocument
/// });
/// ```
#[tauri::command]
pub async fn add_to_folder(
    mut project: Project,
    parent_id: String,
    node: TreeNode,
) -> Result<Project, ChiknError> {
    // Use hierarchy module to add child
    hierarchy::add_child_to_folder(&mut project.hierarchy, &parent_id, node)?;

    // Save updated project (updates modified timestamp)
    writer::write_project(&mut project)?;

    Ok(project)
}

/// Removes a node from the hierarchy.
///
/// # Arguments
/// * `project` - Current project state
/// * `node_id` - ID of node to remove
///
/// # Returns
/// * `Ok(Project)` - Updated project
/// * `Err(ChiknError)` if node not found
///
/// # Example (from frontend)
/// ```javascript
/// const updatedProject = await invoke('remove_from_hierarchy', {
///   project,
///   nodeId: 'doc123'
/// });
/// ```
#[tauri::command]
pub async fn remove_from_hierarchy(
    mut project: Project,
    node_id: String,
) -> Result<Project, ChiknError> {
    // Use hierarchy module to remove node
    hierarchy::remove_node(&mut project.hierarchy, &node_id)?;

    // Save updated project (updates modified timestamp)
    writer::write_project(&mut project)?;

    Ok(project)
}

/// Moves a node to a new parent location.
///
/// # Arguments
/// * `project` - Current project state
/// * `node_id` - ID of node to move
/// * `new_parent_id` - ID of new parent (None for root level)
///
/// # Returns
/// * `Ok(Project)` - Updated project
/// * `Err(ChiknError)` if node or parent not found
///
/// # Example (from frontend)
/// ```javascript
/// // Move to root level
/// await invoke('move_node', { project, nodeId: 'doc123', newParentId: null });
///
/// // Move to folder
/// await invoke('move_node', { project, nodeId: 'doc123', newParentId: 'folder456' });
/// ```
#[tauri::command]
pub async fn move_node(
    mut project: Project,
    node_id: String,
    new_parent_id: Option<String>,
) -> Result<Project, ChiknError> {
    // Use hierarchy module to move node
    hierarchy::move_node(&mut project.hierarchy, &node_id, new_parent_id.as_deref())?;

    // Save updated project (updates modified timestamp)
    writer::write_project(&mut project)?;

    Ok(project)
}

/// Reorders a node within its current parent.
///
/// # Arguments
/// * `project` - Current project state
/// * `node_id` - ID of node to reorder
/// * `new_index` - New position (0-based index)
///
/// # Returns
/// * `Ok(Project)` - Updated project
/// * `Err(ChiknError)` if node not found or index out of bounds
///
/// # Example (from frontend)
/// ```javascript
/// // Move first document to second position
/// const updatedProject = await invoke('reorder_node', {
///   project,
///   nodeId: 'doc123',
///   newIndex: 1
/// });
/// ```
#[tauri::command]
pub async fn reorder_node(
    mut project: Project,
    node_id: String,
    new_index: usize,
) -> Result<Project, ChiknError> {
    // Use hierarchy module to reorder
    hierarchy::reorder_node(&mut project.hierarchy, &node_id, new_index)?;

    // Save updated project (updates modified timestamp)
    writer::write_project(&mut project)?;

    Ok(project)
}
