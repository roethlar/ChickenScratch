use std::path::{Path, PathBuf};

use chickenscratch_services::{AiStreamRegistry, ProjectTokens, ProjectWriteLocks};

use crate::gate::Gate;

/// Everything the handlers share. One instance per server process.
pub struct AppState {
    /// Canonical projects root: the only folder project paths may resolve into.
    pub root: PathBuf,
    /// The bearer token every `/api/*` request must carry.
    pub token: String,
    /// Folder holding the built UI (`index.html` and `assets/`).
    pub ui_dir: PathBuf,
    pub gate: Gate,
    pub locks: ProjectWriteLocks,
    pub tokens: ProjectTokens,
    pub ai_registry: AiStreamRegistry,
}

impl AppState {
    /// `root` must exist; it is canonicalized so symlinked roots still work.
    pub fn new(root: &Path, token: String, ui_dir: PathBuf) -> std::io::Result<Self> {
        let root = root.canonicalize()?;
        if !root.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotADirectory,
                format!("projects root is not a directory: {}", root.display()),
            ));
        }
        Ok(Self {
            gate: Gate::new(vec![root.clone()]),
            root,
            token,
            ui_dir,
            locks: ProjectWriteLocks::default(),
            tokens: ProjectTokens::default(),
            ai_registry: AiStreamRegistry::default(),
        })
    }
}
