//! Error types for Chicken Scratch
//!
//! Centralized error handling using thiserror for ergonomic error propagation.

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

    #[error("Unknown error: {0}")]
    Unknown(String),
}

// Convert to Tauri-compatible error
impl From<ChiknError> for tauri::Error {
    fn from(err: ChiknError) -> Self {
        tauri::Error::Anyhow(anyhow::Error::msg(err.to_string()))
    }
}
