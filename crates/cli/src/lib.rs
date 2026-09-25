//! chikn-converter library
//!
//! Provides Scrivener <-> .chikn conversion functionality.
//! This is a client of chickenscratch-core (ChickenEngine).
//! It does not implement .chikn format logic itself.

pub mod scrivener;

pub use scrivener::converter::{import_scriv, import_scriv_with_report, ImportReport};
pub use scrivener::exporter::export_to_scriv;
