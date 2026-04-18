//! Document model
//!
//! Represents a single document in a ChickenScratch project

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

    /// Include in compile/export output (default: true)
    #[serde(default = "default_true")]
    pub include_in_compile: bool,

    /// Word count target for this document (0 = no target)
    #[serde(default)]
    pub word_count_target: u32,

    /// Custom compile order (0 = use binder order, higher = later)
    #[serde(default)]
    pub compile_order: i32,

    /// Comments on spans in this document (keyed by comment id)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<Comment>,
}

/// A comment anchored to a span in the document.
/// The anchor is a `<span class="comment" data-comment-id="...">` element in `content`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    /// Unique comment id (matches the data-comment-id attribute in the content)
    pub id: String,
    /// Comment body (plain text)
    pub body: String,
    /// Whether the comment is resolved
    #[serde(default)]
    pub resolved: bool,
    /// Creation timestamp (RFC3339)
    pub created: String,
    /// Last modified timestamp (RFC3339)
    pub modified: String,
}

fn default_true() -> bool {
    true
}

impl Default for Document {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            path: String::new(),
            content: String::new(),
            parent_id: None,
            created: String::new(),
            modified: String::new(),
            synopsis: None,
            label: None,
            status: None,
            keywords: None,
            links: None,
            include_in_compile: true,
            word_count_target: 0,
            compile_order: 0,
            comments: Vec::new(),
        }
    }
}
