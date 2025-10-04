//! # Scrivener Compatibility Module
//!
//! Import and export functionality for Scrivener .scriv projects.
//!
//! ## Responsibilities
//! - Parse .scrivx XML files
//! - Convert RTF ↔ Markdown
//! - Import .scriv → .chikn
//! - Export .chikn → .scriv
//! - Preserve metadata for round-trip fidelity
//!
//! ## Structure
//! - `parser`: .scrivx XML parsing and RTF reading
//! - `converter`: .scriv → .chikn conversion logic
//! - `exporter`: .chikn → .scriv generation

pub mod parser;
pub mod converter;
pub mod exporter;

/// Scrivener project version constants
pub const SCRIVENER_VERSION_3: &str = "3.0";
pub const SCRIVENER_VERSION_1: &str = "1.0";
