//! The projects-root gate.
//!
//! The desktop app may touch any path its user picks in a native dialog. A
//! browser has no such dialog and the server is reachable over a network, so
//! every path-bearing command argument (`path`, `projectPath`, `outputPath`,
//! `filePath`, `folderPath`, `scrivPath`) must resolve inside one of the
//! allowed roots before the command runs. Arg structs declare such fields as
//! [`PathArg`], whose inner string is private: a handler cannot forward a path
//! it has not resolved through the gate.
//!
//! Resolution follows symlinks the way the OS does (`canonicalize`), so a link
//! inside the root that points outside is rejected. For paths that will be
//! created (a new project folder, an export file) the parent must resolve
//! inside a root and the final component must be a plain name.
//!
//! Deliberately **not** gated: settings values that are paths (the backup
//! folder, the Pandoc executable) and the `backupDir` argument of the backup
//! commands. They are server-local preferences set by the single
//! authenticated user and are consumed by the services layer exactly as on the
//! desktop.

use std::fmt;
use std::path::{Component, Path, PathBuf};

use axum::http::StatusCode;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Gate {
    roots: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateError {
    /// A relative path: the UI always sends absolute paths the server handed out.
    Relative(String),
    /// Resolved outside every allowed root (or a symlink led there).
    Outside,
    /// The path (or, for creatable paths, its parent folder) does not exist.
    Missing(String),
    /// A creatable path whose final component is not a plain name.
    BadName(String),
}

impl GateError {
    pub fn status(&self) -> StatusCode {
        match self {
            GateError::Relative(_) | GateError::BadName(_) => StatusCode::BAD_REQUEST,
            GateError::Outside => StatusCode::FORBIDDEN,
            GateError::Missing(_) => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
}

impl fmt::Display for GateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GateError::Relative(_) => f.write_str("Path must be absolute"),
            GateError::Outside => f.write_str("Path is outside the projects folder"),
            GateError::Missing(_) => f.write_str("That folder or file does not exist"),
            GateError::BadName(_) => f.write_str("Path ends in a name that cannot be created"),
        }
    }
}

impl std::error::Error for GateError {}

impl Gate {
    /// `roots` must already be canonical.
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Self { roots }
    }

    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    fn inside(&self, canonical: &Path) -> bool {
        self.roots.iter().any(|root| canonical.starts_with(root))
    }

    /// An existing file or folder, resolved through symlinks, inside a root.
    pub fn existing(&self, raw: &str) -> Result<PathBuf, GateError> {
        let path = Path::new(raw);
        if !path.is_absolute() {
            return Err(GateError::Relative(raw.to_string()));
        }
        let canonical = path
            .canonicalize()
            .map_err(|_| GateError::Missing(raw.to_string()))?;
        if self.inside(&canonical) {
            Ok(canonical)
        } else {
            tracing::debug!(path = raw, "gate: outside projects root");
            Err(GateError::Outside)
        }
    }

    /// A path that may not exist yet: its parent must be an existing folder
    /// inside a root and its last component a plain name.
    pub fn creatable(&self, raw: &str) -> Result<PathBuf, GateError> {
        let path = Path::new(raw);
        if !path.is_absolute() {
            return Err(GateError::Relative(raw.to_string()));
        }
        if path.exists() {
            return self.existing(raw);
        }
        let name = match path.components().next_back() {
            Some(Component::Normal(name)) => name.to_owned(),
            _ => return Err(GateError::BadName(raw.to_string())),
        };
        let parent = path
            .parent()
            .ok_or_else(|| GateError::BadName(raw.to_string()))?;
        let parent = parent
            .canonicalize()
            .map_err(|_| GateError::Missing(raw.to_string()))?;
        if !self.inside(&parent) {
            tracing::debug!(path = raw, "gate: parent outside projects root");
            return Err(GateError::Outside);
        }
        Ok(parent.join(name))
    }
}

/// A command argument that names a filesystem path. Deserializes from a plain
/// JSON string; usable only by resolving it through a [`Gate`].
#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct PathArg(String);

impl PathArg {
    pub fn existing(self, gate: &Gate) -> Result<String, GateError> {
        Ok(gate.existing(&self.0)?.to_string_lossy().into_owned())
    }

    pub fn creatable(self, gate: &Gate) -> Result<String, GateError> {
        Ok(gate.creatable(&self.0)?.to_string_lossy().into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn gate(root: &Path) -> Gate {
        Gate::new(vec![root.canonicalize().unwrap()])
    }

    #[test]
    fn existing_inside_root_is_accepted_canonically() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir_all(root.join("Notes.chikn")).unwrap();
        let g = gate(&root);
        let p = g
            .existing(&root.join("Notes.chikn").to_string_lossy())
            .unwrap();
        assert_eq!(p, root.canonicalize().unwrap().join("Notes.chikn"));
    }

    #[test]
    fn relative_and_dotdot_and_outside_are_refused() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().join("root");
        let outside = tmp.path().join("outside");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        let g = gate(&root);
        assert!(matches!(
            g.existing("relative/path"),
            Err(GateError::Relative(_))
        ));
        assert_eq!(
            g.existing(&outside.to_string_lossy()),
            Err(GateError::Outside)
        );
        let dotdot = root.join("..").join("outside");
        assert_eq!(
            g.existing(&dotdot.to_string_lossy()),
            Err(GateError::Outside)
        );
        assert_eq!(
            g.creatable(&outside.join("New.chikn").to_string_lossy()),
            Err(GateError::Outside)
        );
    }

    #[test]
    fn creatable_requires_existing_parent_and_plain_name() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir_all(&root).unwrap();
        let g = gate(&root);
        let ok = g
            .creatable(&root.join("New.chikn").to_string_lossy())
            .unwrap();
        assert_eq!(ok, root.canonicalize().unwrap().join("New.chikn"));
        assert!(matches!(
            g.creatable(&root.join("missing").join("New.chikn").to_string_lossy()),
            Err(GateError::Missing(_))
        ));
        assert!(matches!(
            g.creatable(&root.join("x").join("..").to_string_lossy()),
            Err(GateError::BadName(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_inside_root_pointing_outside_is_refused() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().join("root");
        let outside = tmp.path().join("outside");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("escape")).unwrap();
        let g = gate(&root);
        assert_eq!(
            g.existing(&root.join("escape").to_string_lossy()),
            Err(GateError::Outside)
        );
        assert_eq!(
            g.creatable(&root.join("escape").join("New.chikn").to_string_lossy()),
            Err(GateError::Outside)
        );
    }
}
