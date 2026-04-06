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
pub fn update_document_metadata(
    project_path: String,
    doc_id: String,
    synopsis: Option<String>,
    label: Option<String>,
    status: Option<String>,
    keywords: Option<Vec<String>>,
    include_in_compile: Option<bool>,
) -> Result<Project, ChiknError> {
    let mut project = reader::read_project(Path::new(&project_path))?;
    if let Some(doc) = project.documents.get_mut(&doc_id) {
        doc.synopsis = synopsis;
        doc.label = label;
        doc.status = status;
        doc.keywords = keywords;
        if let Some(inc) = include_in_compile {
            doc.include_in_compile = inc;
        }
        doc.modified = chrono::Utc::now().to_rfc3339();
        writer::write_project(&mut project)?;
        Ok(project)
    } else {
        Err(ChiknError::NotFound(format!("Document not found: {}", doc_id)))
    }
}

#[tauri::command]
pub fn rename_node(
    project_path: String,
    node_id: String,
    new_name: String,
) -> Result<Project, ChiknError> {
    let mut project = reader::read_project(Path::new(&project_path))?;

    // Rename in hierarchy
    rename_in_hierarchy(&mut project.hierarchy, &node_id, &new_name);

    // Rename document if it exists
    if let Some(doc) = project.documents.get_mut(&node_id) {
        doc.name = new_name;
        doc.modified = chrono::Utc::now().to_rfc3339();
    }

    writer::write_project(&mut project)?;
    Ok(project)
}

fn rename_in_hierarchy(nodes: &mut Vec<TreeNode>, node_id: &str, new_name: &str) {
    for node in nodes {
        match node {
            TreeNode::Document { id, name, .. } if id == node_id => {
                *name = new_name.to_string();
                return;
            }
            TreeNode::Folder { id, name, children } => {
                if id == node_id {
                    *name = new_name.to_string();
                    return;
                }
                rename_in_hierarchy(children, node_id, new_name);
            }
            _ => {}
        }
    }
}

#[tauri::command]
pub fn link_documents(
    project_path: String,
    doc_id_a: String,
    doc_id_b: String,
) -> Result<Project, ChiknError> {
    let mut project = reader::read_project(Path::new(&project_path))?;

    // Add bidirectional link
    for (from, to) in [(&doc_id_a, &doc_id_b), (&doc_id_b, &doc_id_a)] {
        if let Some(doc) = project.documents.get_mut(from) {
            let links = doc.links.get_or_insert_with(Vec::new);
            if !links.contains(to) {
                links.push(to.clone());
            }
        }
    }

    writer::write_project(&mut project)?;
    Ok(project)
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
        ..Default::default()
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
