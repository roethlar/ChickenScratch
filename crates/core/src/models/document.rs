//! Document model
//!
//! Represents a single document in a Chicken Scratch project

use serde::{Deserialize, Serialize};

/// Document model
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Document {
    /// Unique document ID
    pub id: String,

    /// Document name
    pub name: String,

    /// File path relative to .chikn/ directory
    pub path: String,

    /// HTML content
    pub content: String,

    /// Parent ID (folder or root)
    pub parent_id: Option<String>,

    /// Creation timestamp
    pub created: String,

    /// Last modified timestamp
    pub modified: String,

    /// Short summary
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synopsis: Option<String>,

    /// Label (e.g., "Scene", "Chapter", POV character)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    /// Status (e.g., "Draft", "Revised", "Final")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Tags for searching and grouping
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,

    /// IDs of related documents (connections)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<String>>,
}
