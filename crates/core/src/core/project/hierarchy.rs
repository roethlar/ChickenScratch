//! # Hierarchy Operations
//!
//! Tree manipulation operations for document organization.
//!
//! ## Responsibilities
//! - Add documents/folders to hierarchy
//! - Move nodes within tree
//! - Delete nodes from tree
//! - Reorder sibling nodes
//! - Find nodes by ID
//!
//! ## Example
//! ```rust
//! use crate::core::project::hierarchy::add_document_to_hierarchy;
//! use crate::models::TreeNode;
//!
//! let mut hierarchy = Vec::new();
//! let node = TreeNode::Document {
//!     id: "doc1".to_string(),
//!     name: "Chapter 1".to_string(),
//!     path: "manuscript/chapter-01.md".to_string(),
//! };
//! add_document_to_hierarchy(&mut hierarchy, node);
//! ```

use crate::models::TreeNode;
use crate::utils::error::ChiknError;

/// Adds a document or folder to the hierarchy at the root level.
///
/// # Arguments
/// * `hierarchy` - Mutable reference to root hierarchy vector
/// * `node` - TreeNode to add
///
/// # Example
/// ```rust
/// add_document_to_hierarchy(&mut project.hierarchy, new_doc);
/// ```
pub fn add_document_to_hierarchy(hierarchy: &mut Vec<TreeNode>, node: TreeNode) {
    hierarchy.push(node);
}

/// Adds a document or folder as a child of a specific folder.
///
/// # Arguments
/// * `hierarchy` - Root hierarchy vector
/// * `parent_id` - ID of parent folder
/// * `node` - TreeNode to add as child
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(ChiknError::NotFound)` if parent folder not found
/// * `Err(ChiknError::InvalidFormat)` if parent is a document (not a folder)
///
/// # Example
/// ```rust
/// add_child_to_folder(&mut project.hierarchy, "folder1", new_doc)?;
/// ```
pub fn add_child_to_folder(
    hierarchy: &mut Vec<TreeNode>,
    parent_id: &str,
    node: TreeNode,
) -> Result<(), ChiknError> {
    find_and_add_child(hierarchy, parent_id, node)
}

/// Recursive helper to find folder and add child
fn find_and_add_child(
    nodes: &mut Vec<TreeNode>,
    parent_id: &str,
    child: TreeNode,
) -> Result<(), ChiknError> {
    for node in nodes.iter_mut() {
        match node {
            TreeNode::Folder { id, children, .. } => {
                if id == parent_id {
                    children.push(child);
                    return Ok(());
                }
                // Recursively search in children
                if find_and_add_child(children, parent_id, child.clone()).is_ok() {
                    return Ok(());
                }
            }
            TreeNode::Document { id, .. } => {
                if id == parent_id {
                    return Err(ChiknError::InvalidFormat(
                        "Cannot add child to a document (only folders can have children)"
                            .to_string(),
                    ));
                }
            }
        }
    }
    Err(ChiknError::NotFound(format!(
        "Parent folder not found: {}",
        parent_id
    )))
}

/// Removes a node from the hierarchy by ID.
///
/// # Arguments
/// * `hierarchy` - Root hierarchy vector
/// * `node_id` - ID of node to remove
///
/// # Returns
/// * `Ok(TreeNode)` - The removed node
/// * `Err(ChiknError::NotFound)` if node not found
///
/// # Example
/// ```rust
/// let removed = remove_node(&mut project.hierarchy, "doc1")?;
/// ```
pub fn remove_node(hierarchy: &mut Vec<TreeNode>, node_id: &str) -> Result<TreeNode, ChiknError> {
    remove_node_recursive(hierarchy, node_id)
}

/// Recursive helper to find and remove node
fn remove_node_recursive(nodes: &mut Vec<TreeNode>, node_id: &str) -> Result<TreeNode, ChiknError> {
    // Check if node is at this level
    if let Some(pos) = nodes.iter().position(|n| n.id() == node_id) {
        return Ok(nodes.remove(pos));
    }

    // Recursively search in folder children
    for node in nodes.iter_mut() {
        if let TreeNode::Folder { children, .. } = node {
            if let Ok(removed) = remove_node_recursive(children, node_id) {
                return Ok(removed);
            }
        }
    }

    Err(ChiknError::NotFound(format!("Node not found: {}", node_id)))
}

/// Finds a node in the hierarchy by ID.
///
/// # Arguments
/// * `hierarchy` - Root hierarchy vector
/// * `node_id` - ID to search for
///
/// # Returns
/// * `Some(&TreeNode)` if found
/// * `None` if not found
///
/// # Example
/// ```rust
/// if let Some(node) = find_node(&project.hierarchy, "doc1") {
///     println!("Found: {}", node.name());
/// }
/// ```
pub fn find_node<'a>(hierarchy: &'a [TreeNode], node_id: &str) -> Option<&'a TreeNode> {
    find_node_recursive(hierarchy, node_id)
}

/// Recursive helper to find node
fn find_node_recursive<'a>(nodes: &'a [TreeNode], node_id: &str) -> Option<&'a TreeNode> {
    for node in nodes {
        if node.id() == node_id {
            return Some(node);
        }

        if let TreeNode::Folder { children, .. } = node {
            if let Some(found) = find_node_recursive(children, node_id) {
                return Some(found);
            }
        }
    }
    None
}

/// Moves a node to a new parent folder.
///
/// # Arguments
/// * `hierarchy` - Root hierarchy vector
/// * `node_id` - ID of node to move
/// * `new_parent_id` - ID of new parent folder (None for root level)
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(ChiknError)` if node or parent not found
///
/// # Example
/// ```rust
/// // Move document to root level
/// move_node(&mut project.hierarchy, "doc1", None)?;
///
/// // Move document into folder
/// move_node(&mut project.hierarchy, "doc1", Some("folder1"))?;
/// ```
pub fn move_node(
    hierarchy: &mut Vec<TreeNode>,
    node_id: &str,
    new_parent_id: Option<&str>,
) -> Result<(), ChiknError> {
    // Remove node from current location
    let node = remove_node(hierarchy, node_id)?;

    // Add to new location
    match new_parent_id {
        None => {
            // Move to root level
            add_document_to_hierarchy(hierarchy, node);
        }
        Some(parent_id) => {
            // Move to specific folder
            add_child_to_folder(hierarchy, parent_id, node)?;
        }
    }

    Ok(())
}

/// Reorders a node within its current parent.
///
/// # Arguments
/// * `hierarchy` - Root hierarchy vector
/// * `node_id` - ID of node to reorder
/// * `new_index` - New position index (0-based)
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(ChiknError)` if node not found or index out of bounds
///
/// # Example
/// ```rust
/// // Move first item to second position
/// reorder_node(&mut project.hierarchy, "doc1", 1)?;
/// ```
pub fn reorder_node(
    hierarchy: &mut Vec<TreeNode>,
    node_id: &str,
    new_index: usize,
) -> Result<(), ChiknError> {
    reorder_node_recursive(hierarchy, node_id, new_index)
}

/// Recursive helper to reorder node
fn reorder_node_recursive(
    nodes: &mut Vec<TreeNode>,
    node_id: &str,
    new_index: usize,
) -> Result<(), ChiknError> {
    // Check if node is at this level
    if let Some(old_index) = nodes.iter().position(|n| n.id() == node_id) {
        if new_index >= nodes.len() {
            return Err(ChiknError::InvalidFormat(format!(
                "Index out of bounds: {} (max: {})",
                new_index,
                nodes.len() - 1
            )));
        }

        let node = nodes.remove(old_index);
        nodes.insert(new_index, node);
        return Ok(());
    }

    // Recursively search in folder children
    for node in nodes.iter_mut() {
        if let TreeNode::Folder { children, .. } = node {
            if reorder_node_recursive(children, node_id, new_index).is_ok() {
                return Ok(());
            }
        }
    }

    Err(ChiknError::NotFound(format!("Node not found: {}", node_id)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_hierarchy() -> Vec<TreeNode> {
        vec![
            TreeNode::Document {
                id: "doc1".to_string(),
                name: "Chapter 1".to_string(),
                path: "manuscript/ch1.md".to_string(),
            },
            TreeNode::Folder {
                id: "folder1".to_string(),
                name: "Part 1".to_string(),
                children: vec![TreeNode::Document {
                    id: "doc2".to_string(),
                    name: "Chapter 2".to_string(),
                    path: "manuscript/ch2.md".to_string(),
                }],
            },
        ]
    }

    #[test]
    fn test_add_document_to_hierarchy() {
        let mut hierarchy = Vec::new();
        let node = TreeNode::Document {
            id: "doc1".to_string(),
            name: "Test".to_string(),
            path: "test.md".to_string(),
        };

        add_document_to_hierarchy(&mut hierarchy, node);
        assert_eq!(hierarchy.len(), 1);
    }

    #[test]
    fn test_add_child_to_folder() {
        let mut hierarchy = create_test_hierarchy();
        let new_doc = TreeNode::Document {
            id: "doc3".to_string(),
            name: "Chapter 3".to_string(),
            path: "manuscript/ch3.md".to_string(),
        };

        let result = add_child_to_folder(&mut hierarchy, "folder1", new_doc);
        assert!(result.is_ok());

        // Verify it was added
        if let TreeNode::Folder { children, .. } = &hierarchy[1] {
            assert_eq!(children.len(), 2);
        } else {
            panic!("Expected folder");
        }
    }

    #[test]
    fn test_add_child_to_document_fails() {
        let mut hierarchy = create_test_hierarchy();
        let new_doc = TreeNode::Document {
            id: "doc3".to_string(),
            name: "Chapter 3".to_string(),
            path: "manuscript/ch3.md".to_string(),
        };

        let result = add_child_to_folder(&mut hierarchy, "doc1", new_doc);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_node() {
        let mut hierarchy = create_test_hierarchy();
        let result = remove_node(&mut hierarchy, "doc1");

        assert!(result.is_ok());
        assert_eq!(hierarchy.len(), 1);
    }

    #[test]
    fn test_remove_nested_node() {
        let mut hierarchy = create_test_hierarchy();
        let result = remove_node(&mut hierarchy, "doc2");

        assert!(result.is_ok());

        // Verify folder now has no children
        if let TreeNode::Folder { children, .. } = &hierarchy[1] {
            assert_eq!(children.len(), 0);
        }
    }

    #[test]
    fn test_find_node() {
        let hierarchy = create_test_hierarchy();

        let found = find_node(&hierarchy, "doc1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "Chapter 1");

        let not_found = find_node(&hierarchy, "nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_find_nested_node() {
        let hierarchy = create_test_hierarchy();

        let found = find_node(&hierarchy, "doc2");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "Chapter 2");
    }

    #[test]
    fn test_move_node_to_root() {
        let mut hierarchy = create_test_hierarchy();

        // Move doc2 from folder to root
        let result = move_node(&mut hierarchy, "doc2", None);
        assert!(result.is_ok());

        assert_eq!(hierarchy.len(), 3);

        // Verify folder is now empty
        if let TreeNode::Folder { children, .. } = &hierarchy[1] {
            assert_eq!(children.len(), 0);
        }
    }

    #[test]
    fn test_move_node_to_folder() {
        let mut hierarchy = create_test_hierarchy();

        // Move doc1 into folder1
        let result = move_node(&mut hierarchy, "doc1", Some("folder1"));
        assert!(result.is_ok());

        assert_eq!(hierarchy.len(), 1);

        // Verify folder now has 2 children
        if let TreeNode::Folder { children, .. } = &hierarchy[0] {
            assert_eq!(children.len(), 2);
        }
    }

    #[test]
    fn test_reorder_node() {
        let mut hierarchy = create_test_hierarchy();

        // Move doc1 (at index 0) to index 1
        let result = reorder_node(&mut hierarchy, "doc1", 1);
        assert!(result.is_ok());

        assert_eq!(hierarchy[0].id(), "folder1");
        assert_eq!(hierarchy[1].id(), "doc1");
    }

    #[test]
    fn test_reorder_node_out_of_bounds() {
        let mut hierarchy = create_test_hierarchy();

        let result = reorder_node(&mut hierarchy, "doc1", 10);
        assert!(result.is_err());
    }
}
