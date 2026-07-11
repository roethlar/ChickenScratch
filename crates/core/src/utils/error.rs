//! Error types for ChickenScratch
//!
//! Centralized error handling using thiserror for ergonomic error propagation.

use serde::{Serialize, Serializer};
use thiserror::Error;

/// Actionable git failure classes for UI and CLI callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitErrorKind {
    Auth,
    Conflict,
    NotFastForward,
    NoUpstream,
    RemoteUnavailable,
    NoCommits,
    NotARepo,
    InvalidRevision,
    DetachedHead,
    Other,
}

/// Git-specific error payload that preserves the user-facing message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitError {
    pub kind: GitErrorKind,
    pub message: String,
}

impl GitError {
    pub fn new(kind: GitErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for GitError {}

/// Main error type for ChickenScratch operations
#[derive(Debug, Error)]
pub enum ChiknError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_yaml::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid project: {0}")]
    InvalidProject(String),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("{0}")]
    Git(GitError),

    /// The project may only be opened read-only: it probed Degraded, or a
    /// write token was stale or bound to a different project. The payload
    /// is a plain-English explanation safe to surface directly.
    #[error("This project is read-only: {0}")]
    ReadOnly(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

// Implement Serialize for Tauri error handling
impl Serialize for ChiknError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
