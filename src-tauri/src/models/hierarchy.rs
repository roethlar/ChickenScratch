//! Document hierarchy model
//!
//! Tree structure for organizing documents and folders

use serde::{Deserialize, Serialize};

/// Tree node representing either a folder or a document
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TreeNode {
    Folder {
        id: String,
        name: String,
        children: Vec<TreeNode>,
    },
    Document {
        id: String,
        name: String,
        path: String,
    },
}

impl TreeNode {
    pub fn id(&self) -> &str {
        match self {
            TreeNode::Folder { id, .. } => id,
            TreeNode::Document { id, .. } => id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            TreeNode::Folder { name, .. } => name,
            TreeNode::Document { name, .. } => name,
        }
    }
}
