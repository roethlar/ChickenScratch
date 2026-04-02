//! Document model
//!
//! Represents a single document in a Chicken Scratch project

use serde::{Deserialize, Serialize};

/// Document model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Unique document ID
    pub id: String,

    /// Document name
    pub name: String,

    /// File path relative to .chikn/ directory
    pub path: String,

    /// Markdown content
    pub content: String,

    /// Parent ID (folder or root)
    pub parent_id: Option<String>,

    /// Creation timestamp
    pub created: String,

    /// Last modified timestamp
    pub modified: String,
}
