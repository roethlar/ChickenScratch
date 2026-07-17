pub mod core;
pub mod models;
pub mod utils;

pub use models::{Document, Project, SessionTarget, Thread, TreeNode};
pub use utils::error::{ChiknError, GitError, GitErrorKind};

/// Test-fixture helpers, exported only under the `test-support` feature.
///
/// Never enable this feature from application code: everything in here
/// bypasses the `WritePermit` boundary and exists solely so integration
/// tests can build fixtures without re-owning format details.
#[cfg(feature = "test-support")]
pub mod test_support {
    use std::path::Path;

    /// Create or open a git repo exactly as project creation does
    /// (verify-or-create, standard `.gitignore`). Wraps the crate-private
    /// `core::git::init_repo`, which is unguarded by design and therefore
    /// not part of the public API.
    pub fn init_repo(path: &Path) -> Result<git2::Repository, crate::ChiknError> {
        crate::core::git::init_repo(path)
    }
}
