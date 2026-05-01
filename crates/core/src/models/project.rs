//! Project model
//!
//! Core data structure representing a .chikn project

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{Document, TreeNode};

/// A plot thread — novelist UI convention persisted at the project root in
/// `threads.yaml`. The format itself stays genre-agnostic; this lives in the
/// project model so any frontend that wants threads has a typed view of them
/// while frontends that don't simply ignore the field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    /// Stable id (slug-style); referenced from a Document's `fields.threads`.
    pub id: String,
    /// Display name.
    pub name: String,
    /// CSS-style colour (e.g. `#3b82f6`); used for binder dots and chips.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Optional free-form prose description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Project-level metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_type: Option<String>, // "Novel", "Short Story", "Screenplay", etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

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

    /// Project-level metadata (title, author, type, etc.)
    #[serde(default)]
    pub metadata: ProjectMeta,

    /// Plot threads (novelist convention; persisted in `threads.yaml`).
    /// Empty for projects that don't use them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub threads: Vec<Thread>,
}
