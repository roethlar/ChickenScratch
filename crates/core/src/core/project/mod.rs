//! Project management module
//!
//! Handles .chikn project operations including reading, writing,
//! and hierarchy management.

pub mod deletion;
pub mod fidelity;
pub mod format;
pub mod hierarchy;
pub mod reader;
pub(crate) mod safe_path;
pub mod writer;
