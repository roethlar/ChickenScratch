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

pub mod scrivx;
pub mod rtf;

pub use scrivx::{ScrivenerProject, BinderItem, BinderMetadata, parse_scrivx, get_rtf_path};
pub use rtf::{rtf_to_markdown, markdown_to_rtf, markdown_string_to_rtf};
