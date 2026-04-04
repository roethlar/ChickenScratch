use chickenscratch_core::core::project::{hierarchy, reader, writer};
use chickenscratch_core::utils::slug;
use chickenscratch_core::{ChiknError, Document, Project, TreeNode};
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
        Err(ChiknError::NotFound(format!(
            "Document not found: {}",
            doc_id
        )))
    }
}

#[tauri::command]
pub fn create_document(
    project_path: String,
    name: String,
    parent_id: Option<String>,
) -> Result<Project, ChiknError> {
    let mut project = reader::read_project(Path::new(&project_path))?;

    let doc_id = uuid::Uuid::new_v4().to_string();
    let s = slug::unique_slug(&name, "manuscript/", &project.documents);
    let doc_path = format!("manuscript/{}.html", s);
    let now = chrono::Utc::now().to_rfc3339();

    let document = Document {
        id: doc_id.clone(),
        name: name.clone(),
        path: doc_path.clone(),
        content: String::new(),
        parent_id: parent_id.clone(),
        created: now.clone(),
        modified: now,
    };

    project.documents.insert(doc_id.clone(), document);

    let node = TreeNode::Document {
        id: doc_id,
        name,
        path: doc_path,
    };

    match parent_id {
        Some(pid) => hierarchy::add_child_to_folder(&mut project.hierarchy, &pid, node)?,
        None => hierarchy::add_document_to_hierarchy(&mut project.hierarchy, node),
    }

    writer::write_project(&mut project)?;
    Ok(project)
}

#[tauri::command]
pub fn create_folder(
    project_path: String,
    name: String,
    parent_id: Option<String>,
) -> Result<Project, ChiknError> {
    let mut project = reader::read_project(Path::new(&project_path))?;

    let folder_id = uuid::Uuid::new_v4().to_string();
    let node = TreeNode::Folder {
        id: folder_id,
        name,
        children: Vec::new(),
    };

    match parent_id {
        Some(pid) => hierarchy::add_child_to_folder(&mut project.hierarchy, &pid, node)?,
        None => hierarchy::add_document_to_hierarchy(&mut project.hierarchy, node),
    }

    writer::write_project(&mut project)?;
    Ok(project)
}

#[tauri::command]
pub fn delete_node(project_path: String, node_id: String) -> Result<Project, ChiknError> {
    let mut project = reader::read_project(Path::new(&project_path))?;
    let path = Path::new(&project_path);

    // Remove from hierarchy
    let removed = hierarchy::remove_node(&mut project.hierarchy, &node_id)?;

    // Delete files for documents (and recursively for folders)
    delete_node_files(&removed, &project, path)?;

    writer::write_project(&mut project)?;
    Ok(project)
}

fn delete_node_files(node: &TreeNode, project: &Project, project_path: &Path) -> Result<(), ChiknError> {
    match node {
        TreeNode::Document { id, .. } => {
            if let Some(doc) = project.documents.get(id) {
                let _ = writer::delete_document(project_path, &doc.path);
            }
        }
        TreeNode::Folder { children, .. } => {
            for child in children {
                delete_node_files(child, project, project_path)?;
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn move_node(
    project_path: String,
    node_id: String,
    new_parent_id: Option<String>,
    new_index: Option<usize>,
) -> Result<Project, ChiknError> {
    let mut project = reader::read_project(Path::new(&project_path))?;

    hierarchy::move_node(
        &mut project.hierarchy,
        &node_id,
        new_parent_id.as_deref(),
    )?;

    if let Some(idx) = new_index {
        let _ = hierarchy::reorder_node(&mut project.hierarchy, &node_id, idx);
    }

    writer::write_project(&mut project)?;
    Ok(project)
}
