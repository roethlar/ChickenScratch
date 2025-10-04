//! Error types for Chicken Scratch
//!
//! Centralized error handling using thiserror for ergonomic error propagation.

use serde::{Serialize, Serializer};
use thiserror::Error;

/// Main error type for Chicken Scratch operations
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
