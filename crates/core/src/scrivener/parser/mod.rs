//! # Scrivener Parser
//!
//! Parses Scrivener .scriv project files.
//!
//! ## Responsibilities
//! - Parse .scrivx XML structure
//! - Extract document hierarchy
//! - Read RTF content files
//! - Extract metadata (labels, status, keywords, synopsis)
//!
//! ## Scrivener Format
//! ```
//! MyProject.scriv/
//! ├── MyProject.scrivx           # XML: project structure
//! ├── Files/
//! │   ├── Data/{UUID}/content.rtf
//! │   └── binder.backup
//! └── Settings/*.plist
//! ```

pub mod rtf;
pub mod scrivx;

pub use rtf::{markdown_string_to_rtf, markdown_to_rtf, rtf_to_markdown};
pub use scrivx::{get_rtf_path, parse_scrivx, BinderItem, BinderMetadata, ScrivenerProject};
