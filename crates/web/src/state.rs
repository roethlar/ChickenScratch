use std::path::{Path, PathBuf};

use tokio::sync::broadcast;

use chickenscratch_services::{AiStreamRegistry, ProjectTokens, ProjectWriteLocks};

use crate::events::{self, SseEvent};
use crate::export::CompileReports;
use crate::gate::Gate;
use crate::lease::Leases;
use crate::static_ui::UiSource;

/// Everything the handlers share. One instance per server process.
pub struct AppState {
    /// Canonical projects root: the only folder project paths may resolve into.
    pub root: PathBuf,
    /// Optional second folder the browser may browse (backup setting). Not a
    /// project root: commands still resolve project paths against `gate`.
    pub backup: Option<PathBuf>,
    /// Folders the folder picker may list: the root, plus `backup` if set.
    pub browse: Gate,
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
    /// Recent exports' compile warnings, fetched by the UI after a download
    /// (`GET /api/compile-report`).
    pub reports: CompileReports,
}

impl AppState {
    /// `root` must exist; it is canonicalized so symlinked roots still work.
    /// `backup`, when given, is created if missing and canonicalized too.
    pub fn new(root: &Path, backup: Option<&Path>, ui: UiSource) -> std::io::Result<Self> {
        let root = root.canonicalize()?;
        if !root.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotADirectory,
                format!("projects root is not a directory: {}", root.display()),
            ));
        }
        let backup = match backup {
            Some(dir) => {
                std::fs::create_dir_all(dir)?;
                Some(dir.canonicalize()?)
            }
            None => None,
        };
        let mut browse_roots = vec![root.clone()];
        browse_roots.extend(backup.iter().cloned());
        Ok(Self {
            gate: Gate::new(vec![root.clone()]),
            browse: Gate::new(browse_roots),
            root,
            backup,
            ui,
            locks: ProjectWriteLocks::default(),
            tokens: ProjectTokens::default(),
            ai_registry: AiStreamRegistry::default(),
            events: events::channel(),
            leases: Leases::default(),
            reports: CompileReports::default(),
        })
    }
}
