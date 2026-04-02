//! Project model
//!
//! Core data structure representing a .chikn project

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{Document, TreeNode};

/// Project model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Project unique ID
    pub id: String,

    /// Project name
    pub name: String,

    /// File system path to .chikn folder
    pub path: String,

    /// Document hierarchy (root nodes)
    pub hierarchy: Vec<TreeNode>,

    /// All documents by ID
    pub documents: HashMap<String, Document>,

    /// Project creation timestamp
    pub created: String,

    /// Last modified timestamp
    pub modified: String,
}
