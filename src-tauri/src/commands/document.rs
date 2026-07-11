use chickenscratch_core::core::project::fidelity::{self, WriteToken};
use chickenscratch_core::core::project::{hierarchy, reader, writer};
use chickenscratch_core::models::Comment;
use chickenscratch_core::utils::slug;
use chickenscratch_core::{ChiknError, Document, Project, TreeNode};
use std::path::{Path, PathBuf};
use tauri::State;

use super::ProjectWriteLocks;

#[tauri::command]
pub fn get_document(project_path: String, doc_id: String) -> Result<Option<Document>, ChiknError> {
    let project = reader::read_project(Path::new(&project_path))?;
    Ok(project.documents.get(&doc_id).cloned())
}

#[tauri::command]
pub fn update_document_content(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    doc_id: String,
    content: String,
) -> Result<(), ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let token = fidelity::acquire_write_token(Path::new(&project_path))?;
        let mut project = reader::read_project(Path::new(&project_path))?;
        if let Some(doc) = project.documents.get_mut(&doc_id) {
            doc.content = content;
            doc.modified = chrono::Utc::now().to_rfc3339();
            writer::write_project(&mut project, &token)?;
            Ok(())
        } else {
            Err(ChiknError::NotFound(format!(
                "Document not found: {}",
                doc_id
            )))
        }
    })
}

/// Add a comment anchored to the given span id. Caller wraps the span with
/// `<span class="comment" data-comment-id="{id}">...</span>` in the content first.
#[tauri::command]
pub fn add_comment(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    doc_id: String,
    comment_id: String,
    body: String,
    new_content: String,
) -> Result<Project, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let token = fidelity::acquire_write_token(Path::new(&project_path))?;
        let mut project = reader::read_project(Path::new(&project_path))?;
        if let Some(doc) = project.documents.get_mut(&doc_id) {
            let now = chrono::Utc::now().to_rfc3339();
            doc.content = new_content;
            doc.comments.push(Comment {
                id: comment_id,
                body,
                resolved: false,
                created: now.clone(),
                modified: now.clone(),
            });
            doc.modified = now;
            writer::write_project(&mut project, &token)?;
            Ok(project)
        } else {
            Err(ChiknError::NotFound(format!(
                "Document not found: {}",
                doc_id
            )))
        }
    })
}

#[tauri::command]
pub fn update_comment(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    doc_id: String,
    comment_id: String,
    body: Option<String>,
    resolved: Option<bool>,
) -> Result<Project, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let token = fidelity::acquire_write_token(Path::new(&project_path))?;
        let mut project = reader::read_project(Path::new(&project_path))?;
        if let Some(doc) = project.documents.get_mut(&doc_id) {
            if let Some(c) = doc.comments.iter_mut().find(|c| c.id == comment_id) {
                if let Some(b) = body {
                    c.body = b;
                }
                if let Some(r) = resolved {
                    c.resolved = r;
                }
                c.modified = chrono::Utc::now().to_rfc3339();
                doc.modified = chrono::Utc::now().to_rfc3339();
                writer::write_project(&mut project, &token)?;
                Ok(project)
            } else {
                Err(ChiknError::NotFound(format!(
                    "Comment not found: {}",
                    comment_id
                )))
            }
        } else {
            Err(ChiknError::NotFound(format!(
                "Document not found: {}",
                doc_id
            )))
        }
    })
}

/// Delete a comment and unwrap its span in the content.
#[tauri::command]
pub fn delete_comment(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    doc_id: String,
    comment_id: String,
    new_content: String,
) -> Result<Project, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let token = fidelity::acquire_write_token(Path::new(&project_path))?;
        let mut project = reader::read_project(Path::new(&project_path))?;
        if let Some(doc) = project.documents.get_mut(&doc_id) {
            doc.comments.retain(|c| c.id != comment_id);
            doc.content = new_content;
            doc.modified = chrono::Utc::now().to_rfc3339();
            writer::write_project(&mut project, &token)?;
            Ok(project)
        } else {
            Err(ChiknError::NotFound(format!(
                "Document not found: {}",
                doc_id
            )))
        }
    })
}

/// Per-key update to a document's generic `fields` map.
///
/// The format has no opinion about keys — any novelist convention lives in the
/// UI. The frontend hands us a set of field updates; for each entry, a value of
/// `Some(Value::Null)` or an empty string/array removes the key, so absent UI
/// input produces a clean .meta diff rather than a stored empty.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct FieldUpdates(pub std::collections::HashMap<String, serde_yaml::Value>);

fn apply_field_updates(doc: &mut chickenscratch_core::models::Document, updates: FieldUpdates) {
    use serde_yaml::Value;
    for (key, value) in updates.0 {
        let remove = match &value {
            Value::Null => true,
            Value::String(s) => s.is_empty(),
            Value::Sequence(seq) => seq.is_empty(),
            Value::Mapping(map) => map.is_empty(),
            _ => false,
        };
        if remove {
            doc.fields.remove(&key);
        } else {
            doc.fields.insert(key, value);
        }
    }
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_document_metadata(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
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
    write_locks.with_project_lock(&project_path, || {
        let token = fidelity::acquire_write_token(Path::new(&project_path))?;
        let mut project = reader::read_project(Path::new(&project_path))?;
        if let Some(doc) = project.documents.get_mut(&doc_id) {
            doc.synopsis = synopsis;
            doc.label = label;
            doc.status = status;
            doc.keywords = keywords;
            if let Some(inc) = include_in_compile {
                doc.include_in_compile = inc;
            }
            if let Some(target) = word_count_target {
                doc.word_count_target = target;
            }
            if let Some(order) = compile_order {
                doc.compile_order = order;
            }
            if let Some(updates) = fields {
                apply_field_updates(doc, updates);
            }
            doc.modified = chrono::Utc::now().to_rfc3339();
            writer::write_project(&mut project, &token)?;
            Ok(project)
        } else {
            Err(ChiknError::NotFound(format!(
                "Document not found: {}",
                doc_id
            )))
        }
    })
}

#[tauri::command]
pub fn rename_node(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    node_id: String,
    new_name: String,
) -> Result<Project, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let token = fidelity::acquire_write_token(Path::new(&project_path))?;
        let mut project = reader::read_project(Path::new(&project_path))?;

        // Rename in hierarchy
        rename_in_hierarchy(&mut project.hierarchy, &node_id, &new_name);

        // Rename document if it exists
        if let Some(doc) = project.documents.get_mut(&node_id) {
            doc.name = new_name;
            doc.modified = chrono::Utc::now().to_rfc3339();
        }

        writer::write_project(&mut project, &token)?;
        Ok(project)
    })
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
    write_locks: State<'_, ProjectWriteLocks>,
    doc_id_a: String,
    doc_id_b: String,
) -> Result<Project, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let token = fidelity::acquire_write_token(Path::new(&project_path))?;
        let mut project = reader::read_project(Path::new(&project_path))?;
        let now = chrono::Utc::now().to_rfc3339();

        // Add bidirectional link. Both endpoints are mutated so both must
        // bump `modified` — the writer now preserves the existing timestamp,
        // so without an explicit bump the .meta files would record the link
        // change with stale dates.
        for (from, to) in [(&doc_id_a, &doc_id_b), (&doc_id_b, &doc_id_a)] {
            if let Some(doc) = project.documents.get_mut(from) {
                let links = doc.links.get_or_insert_with(Vec::new);
                if !links.contains(to) {
                    links.push(to.clone());
                    doc.modified = now.clone();
                }
            }
        }

        writer::write_project(&mut project, &token)?;
        Ok(project)
    })
}

#[tauri::command]
pub fn create_document(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    name: String,
    parent_id: Option<String>,
) -> Result<Project, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let token = fidelity::acquire_write_token(Path::new(&project_path))?;
        let mut project = reader::read_project(Path::new(&project_path))?;

        let doc_id = uuid::Uuid::new_v4().to_string();
        let s = slug::unique_slug(&name, "manuscript/", &project.documents);
        let doc_path = format!("manuscript/{}.md", s);
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

        writer::write_project(&mut project, &token)?;
        Ok(project)
    })
}

/// Create a character or location entity. Entities are regular Documents
/// living under `characters/` or `locations/` (novelist convention) — the
/// format itself stays genre-agnostic.
#[tauri::command]
pub fn create_entity(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    name: String,
    kind: String, // "character" or "location"
) -> Result<Project, ChiknError> {
    let project_path = PathBuf::from(project_path);
    write_locks.with_project_lock(&project_path, || {
        create_entity_impl(&project_path, name, kind)
    })
}

fn create_entity_impl(
    project_path: &Path,
    name: String,
    kind: String,
) -> Result<Project, ChiknError> {
    let folder = match kind.as_str() {
        "character" => "characters",
        "location" => "locations",
        other => {
            return Err(ChiknError::InvalidFormat(format!(
                "Unknown entity kind: {}",
                other
            )))
        }
    };

    let token = fidelity::acquire_write_token(project_path)?;
    writer::ensure_project_subdir(&token, Path::new(folder))?;

    let mut project = reader::read_project(project_path)?;

    let doc_id = uuid::Uuid::new_v4().to_string();
    let base_path = format!("{}/", folder);
    let s = slug::unique_slug(&name, &base_path, &project.documents);
    let doc_path = format!("{}/{}.md", folder, s);
    let now = chrono::Utc::now().to_rfc3339();

    // Tag the entity via the generic fields map so any UI can detect it
    let mut fields = std::collections::BTreeMap::new();
    fields.insert("entity_kind".to_string(), serde_yaml::Value::String(kind));

    let document = Document {
        id: doc_id.clone(),
        name: name.clone(),
        path: doc_path,
        content: String::new(),
        parent_id: None,
        created: now.clone(),
        modified: now,
        fields,
        ..Default::default()
    };

    project.documents.insert(doc_id, document);
    writer::write_project(&mut project, &token)?;
    Ok(project)
}

#[tauri::command]
pub fn create_folder(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    name: String,
    parent_id: Option<String>,
) -> Result<Project, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let token = fidelity::acquire_write_token(Path::new(&project_path))?;
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

        writer::write_project(&mut project, &token)?;
        Ok(project)
    })
}

#[tauri::command]
pub fn delete_node(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    node_id: String,
) -> Result<Project, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let token = fidelity::acquire_write_token(Path::new(&project_path))?;
        let mut project = reader::read_project(Path::new(&project_path))?;
        let path = Path::new(&project_path);

        // Remove from hierarchy
        let removed = hierarchy::remove_node(&mut project.hierarchy, &node_id)?;

        // Delete files AND drop entries from `project.documents`. Without the
        // map cleanup, `write_project` below would iterate the still-present
        // documents and recreate the .md / .meta files we just deleted.
        delete_node_files(&removed, &mut project, path, &token)?;

        writer::write_project(&mut project, &token)?;
        Ok(project)
    })
}

fn delete_node_files(
    node: &TreeNode,
    project: &mut Project,
    project_path: &Path,
    token: &WriteToken,
) -> Result<(), ChiknError> {
    match node {
        TreeNode::Document { id, .. } => {
            // Surface filesystem errors instead of silently dropping the
            // doc from the in-memory map. Otherwise a permission denial
            // or disk-full would leave orphan `.md` / `.meta` files on
            // disk while the binder thinks they're gone — and the next
            // reload's repair pass would re-import them as orphans.
            if let Some(doc) = project.documents.get(id) {
                writer::delete_document(project_path, &doc.path, token)?;
            }
            project.documents.remove(id);
        }
        TreeNode::Folder { children, .. } => {
            for child in children {
                delete_node_files(child, project, project_path, token)?;
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn move_node(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    node_id: String,
    new_parent_id: Option<String>,
    new_index: Option<usize>,
) -> Result<Project, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let token = fidelity::acquire_write_token(Path::new(&project_path))?;
        let mut project = reader::read_project(Path::new(&project_path))?;

        // `None` from the UI means "keep current parent" — used for in-place
        // reorder via Move Up / Move Down and drag-drop within the same
        // sibling list. Without this guard the node would get pulled out of
        // its folder onto the root every time the user nudges it. Use the
        // dedicated reorder path in that case; only call the parent-changing
        // move when a parent was specified.
        if let Some(parent_id) = new_parent_id.as_deref() {
            hierarchy::move_node(&mut project.hierarchy, &node_id, Some(parent_id))?;
            if let Some(idx) = new_index {
                // Propagate reorder errors instead of silently leaving the
                // node at the parent's tail with `Ok(())`. An invalid index
                // (e.g. UI passing a stale position from before another
                // user reordered) used to return success here while the
                // actual position was wrong.
                hierarchy::reorder_node(&mut project.hierarchy, &node_id, idx)?;
            }
        } else if let Some(idx) = new_index {
            hierarchy::reorder_node(&mut project.hierarchy, &node_id, idx)?;
        }

        writer::write_project(&mut project, &token)?;
        Ok(project)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn create_entity_impl_writes_safe_entity_folder() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("Entities.chikn");
        writer::create_project(&project_path, "Entities").unwrap();

        let project = create_entity_impl(
            &project_path,
            "Sarah Bennett".to_string(),
            "character".to_string(),
        )
        .unwrap();

        assert!(project_path.join("characters/sarah-bennett.md").exists());
        assert!(project
            .documents
            .values()
            .any(|doc| doc.path == "characters/sarah-bennett.md"));
    }

    #[cfg(unix)]
    #[test]
    fn create_entity_impl_rejects_symlink_entity_folder() {
        use std::os::unix::fs as unix_fs;

        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("Hostile.chikn");
        let outside_path = temp_dir.path().join("outside");
        fs::create_dir(&outside_path).unwrap();

        writer::create_project(&project_path, "Hostile").unwrap();
        unix_fs::symlink(&outside_path, project_path.join("characters")).unwrap();

        let result = create_entity_impl(
            &project_path,
            "Mallory".to_string(),
            "character".to_string(),
        );

        // The fidelity probe now refuses the write token before the
        // safe-path machinery is even reached: a symlinked entity folder
        // classifies the project Degraded (ReadOnly refusal).
        assert!(matches!(result, Err(ChiknError::ReadOnly(_))));
        assert!(fs::read_dir(&outside_path).unwrap().next().is_none());
        assert!(fs::symlink_metadata(project_path.join("characters"))
            .unwrap()
            .file_type()
            .is_symlink());
    }
}
