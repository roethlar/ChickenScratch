use std::path::{Path, PathBuf};

use tokio::sync::broadcast;

use chickenscratch_services::{AiStreamRegistry, ProjectTokens, ProjectWriteLocks};

use crate::events::{self, SseEvent};
use crate::gate::Gate;
use crate::lease::Leases;
use crate::static_ui::UiSource;

/// Everything the handlers share. One instance per server process.
pub struct AppState {
    /// Canonical projects root: the only folder project paths may resolve into.
    pub root: PathBuf,
    /// The bearer token every `/api/*` request must carry.
    pub token: String,
    /// Where the built UI comes from (a folder, or assets embedded in the binary).
    pub ui: UiSource,
    pub gate: Gate,
    pub locks: ProjectWriteLocks,
    pub tokens: ProjectTokens,
    pub ai_registry: AiStreamRegistry,
    /// Fan-out for the browser's server-sent events (AI streaming).
    pub events: broadcast::Sender<SseEvent>,
    /// Single-writer lease per project for browser sessions.
    pub leases: Leases,
}

impl AppState {
    /// `root` must exist; it is canonicalized so symlinked roots still work.
    pub fn new(root: &Path, token: String, ui: UiSource) -> std::io::Result<Self> {
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
            ui,
            locks: ProjectWriteLocks::default(),
            tokens: ProjectTokens::default(),
            ai_registry: AiStreamRegistry::default(),
            events: events::channel(),
            leases: Leases::default(),
        })
    }
}
