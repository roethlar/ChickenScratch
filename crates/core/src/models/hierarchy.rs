//! Document hierarchy model
//!
//! Tree structure for organizing documents and folders

use serde::{Deserialize, Serialize};

/// Tree node representing either a folder or a document.
///
/// Wire type is the `type` discriminator. The canonical (Rust-written) form
/// is `Folder` / `Document`, but the macOS Swift writer and the Windows C#
/// writer both emit lowercase `folder` / `document`. The `alias` attributes
/// make the reader accept either case so a project authored on any frontend
/// can be reopened in Tauri without re-saving (cross-frontend drift sibling
/// to F-001/F-002).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TreeNode {
    #[serde(alias = "folder")]
    Folder {
        id: String,
        name: String,
        children: Vec<TreeNode>,
    },
    #[serde(alias = "document")]
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
