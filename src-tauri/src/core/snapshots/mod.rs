//! # Snapshot System
//!
//! Automatic backup and restore functionality for .chikn projects.
//!
//! ## Responsibilities
//! - Create compressed snapshots (tarballs) of project state
//! - Restore from snapshots
//! - Manage snapshot history (pruning, manifest)
//! - Provide safety net independent of git
//!
//! ## Structure
//! ```
//! MyProject.chikn/
//! └── revs/
//!     ├── snapshot-20251004-143022.tar.gz
//!     ├── snapshot-20251004-120000.tar.gz
//!     └── manifest.json
//! ```
//!
//! ## Snapshot Contents
//! Archives include:
//! - project.yaml
//! - manuscript/ folder
//! - research/ folder
//! - templates/ folder
//! - settings/ folder
//!
//! Excludes:
//! - .git/ folder
//! - revs/ folder (no recursive snapshots)
//! - Temporary files

mod create;
mod manifest;
mod prune;
mod restore;

pub use create::create_snapshot;
pub use manifest::{SnapshotManifest, SnapshotType};
pub use prune::prune_old_snapshots;
pub use restore::restore_snapshot;

// Re-exported for potential future use
#[allow(unused_imports)]
pub use manifest::SnapshotEntry;
#[allow(unused_imports)]
pub use prune::get_snapshots_size;

/// Snapshot folder name
pub const REVS_FOLDER: &str = "revs";

/// Default number of snapshots to keep
pub const DEFAULT_SNAPSHOT_COUNT: usize = 10;

/// Snapshot filename pattern
pub const SNAPSHOT_PREFIX: &str = "snapshot-";
pub const SNAPSHOT_EXTENSION: &str = "tar.gz";
