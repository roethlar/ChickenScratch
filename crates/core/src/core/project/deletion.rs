//! Permanent project node deletion helpers.
//!
//! UI frontends may implement "move to Trash" separately. The helpers here
//! are for the actual delete operation: remove the hierarchy node, delete all
//! descendant document sidecars, and drop the corresponding document-map
//! entries so later project writes cannot recreate deleted files.

use crate::core::project::fidelity::WritePermit;
use crate::core::project::{hierarchy, writer};
use crate::models::{Project, TreeNode};
use crate::ChiknError;
use std::path::Path;

/// Permanently delete a hierarchy node and all descendant document files.
///
/// Returns the document IDs removed from `project.documents`. Files are
/// deleted before the in-memory hierarchy/map are pruned, so a filesystem
/// error leaves the loaded project structurally intact and no smaller manifest
/// is written by callers that propagate the error.
pub fn delete_node(
    project: &mut Project,
    node_id: &str,
    permit: &WritePermit<'_>,
) -> Result<Vec<String>, ChiknError> {
    let project_path = Path::new(&project.path);
    permit.ensure_valid_for(project_path)?;

    let removed = hierarchy::find_node(&project.hierarchy, node_id)
        .cloned()
        .ok_or_else(|| ChiknError::NotFound(format!("Node not found: {}", node_id)))?;

    let deletions = collect_document_deletions(&removed, project);
    for (_, path) in &deletions {
        writer::delete_document(project_path, path, permit)?;
    }

    hierarchy::remove_node(&mut project.hierarchy, node_id)?;
    for (id, _) in &deletions {
        project.documents.remove(id);
    }

    Ok(deletions.into_iter().map(|(id, _)| id).collect())
}

fn collect_document_deletions(node: &TreeNode, project: &Project) -> Vec<(String, String)> {
    match node {
        TreeNode::Document { id, path, .. } => {
            let path = project
                .documents
                .get(id)
                .map(|doc| doc.path.clone())
                .unwrap_or_else(|| path.clone());
            vec![(id.clone(), path)]
        }
        TreeNode::Folder { children, .. } => children
            .iter()
            .flat_map(|child| collect_document_deletions(child, project))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::project::{reader, writer};
    use crate::models::Document;
    use chrono::Utc;

    #[test]
    fn operation_permit_deletes_multiple_documents_and_prevents_resurrection() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let project_path = temp_dir.path().join("DeleteFolder.chikn");
        let mut project = writer::create_project(&project_path, "Delete Folder").unwrap();
        let token =
            crate::core::project::fidelity::acquire_write_token(&project_path).expect("token");

        let doc = Document {
            id: "nested-doc".to_string(),
            name: "Nested".to_string(),
            path: "manuscript/folder/nested.md".to_string(),
            content: "Nested content.".to_string(),
            parent_id: Some("folder".to_string()),
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };
        project.documents.insert(doc.id.clone(), doc);
        let second_doc = Document {
            id: "nested-doc-2".to_string(),
            name: "Nested Two".to_string(),
            path: "manuscript/folder/nested-two.md".to_string(),
            content: "Second nested content.".to_string(),
            parent_id: Some("folder".to_string()),
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };
        project.documents.insert(second_doc.id.clone(), second_doc);
        project.hierarchy.push(TreeNode::Folder {
            id: "folder".to_string(),
            name: "Folder".to_string(),
            children: vec![
                TreeNode::Document {
                    id: "nested-doc".to_string(),
                    name: "Nested".to_string(),
                    path: "manuscript/folder/nested.md".to_string(),
                },
                TreeNode::Document {
                    id: "nested-doc-2".to_string(),
                    name: "Nested Two".to_string(),
                    path: "manuscript/folder/nested-two.md".to_string(),
                },
            ],
        });
        token
            .with_write_permit(&project_path, |permit| {
                writer::write_project(&mut project, permit)
            })
            .unwrap();

        let doc_path = project_path.join("manuscript/folder/nested.md");
        let meta_path = project_path.join("manuscript/folder/nested.meta");
        let second_doc_path = project_path.join("manuscript/folder/nested-two.md");
        let second_meta_path = project_path.join("manuscript/folder/nested-two.meta");
        assert!(doc_path.exists());
        assert!(meta_path.exists());
        assert!(second_doc_path.exists());
        assert!(second_meta_path.exists());

        let deleted_ids = token
            .with_write_permit(&project_path, |permit| {
                let deleted = delete_node(&mut project, "folder", permit)?;
                writer::write_project(&mut project, permit)?;
                Ok(deleted)
            })
            .unwrap();
        assert_eq!(
            deleted_ids,
            vec!["nested-doc".to_string(), "nested-doc-2".to_string()]
        );
        assert!(!doc_path.exists());
        assert!(!meta_path.exists());
        assert!(!second_doc_path.exists());
        assert!(!second_meta_path.exists());
        assert!(!project.documents.contains_key("nested-doc"));
        assert!(!project.documents.contains_key("nested-doc-2"));
        assert!(hierarchy::find_node(&project.hierarchy, "folder").is_none());

        let reread = reader::read_project(&project_path).unwrap();
        assert!(!reread.documents.contains_key("nested-doc"));
        assert!(!reread.documents.contains_key("nested-doc-2"));
        assert!(hierarchy::find_node(&reread.hierarchy, "nested-doc").is_none());
        assert_eq!(
            crate::core::project::fidelity::probe_project_fidelity(&project_path).unwrap(),
            crate::core::project::fidelity::Fidelity::Full
        );
    }
}
