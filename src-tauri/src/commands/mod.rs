pub mod ai;
pub mod document;
pub mod git;
pub mod io;
pub mod project;
pub mod search;
pub mod settings;
pub mod templates;
pub mod threads;

use chickenscratch_core::core::project::fidelity::{self, WriteToken};
use chickenscratch_core::ChiknError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct ProjectWriteLocks {
    locks: Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>,
}

/// Per-project write tokens held in app state (PLAN_TRUST_FOUNDATIONS
/// Slice 1). A token enters the map when a project opens Full (or on
/// demand via [`ProjectTokens::checkout`]); a Degraded project never
/// yields one, so every mutating command cleanly refuses. Tokens are
/// shared via `Arc` — `WriteToken` itself stays non-`Clone`, and a stale
/// token (after a tree-replacing operation bumped the project's write
/// epoch) is dropped and re-acquired through a fresh fidelity probe.
#[derive(Default)]
pub struct ProjectTokens {
    tokens: Mutex<HashMap<PathBuf, Arc<WriteToken>>>,
}

impl ProjectTokens {
    fn lock(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, HashMap<PathBuf, Arc<WriteToken>>>, ChiknError> {
        self.tokens
            .lock()
            .map_err(|_| ChiknError::Unknown("Project token registry is poisoned".to_string()))
    }

    /// Fetch the cached token for a project, re-probing when it is
    /// missing or stale. Errors with `ReadOnly` for Degraded projects —
    /// the single refusal path for every mutating command.
    pub fn checkout(&self, project_path: impl AsRef<Path>) -> Result<Arc<WriteToken>, ChiknError> {
        let path = project_path.as_ref();
        let key = project_lock_key(path)?;
        {
            let tokens = self.lock()?;
            if let Some(token) = tokens.get(&key) {
                if !token.is_stale() {
                    return Ok(Arc::clone(token));
                }
            }
        }
        match fidelity::acquire_write_token(path) {
            Ok(token) => {
                let token = Arc::new(token);
                self.lock()?.insert(key, Arc::clone(&token));
                Ok(token)
            }
            Err(e) => {
                self.lock()?.remove(&key);
                Err(e)
            }
        }
    }

    /// Record a freshly issued token for an opened project.
    pub fn store(&self, project_path: &Path, token: WriteToken) -> Result<(), ChiknError> {
        let key = project_lock_key(project_path)?;
        self.lock()?.insert(key, Arc::new(token));
        Ok(())
    }

    /// Drop the cached token (Degraded open, or close).
    pub fn invalidate(&self, project_path: &Path) {
        if let Ok(key) = project_lock_key(project_path) {
            if let Ok(mut tokens) = self.tokens.lock() {
                tokens.remove(&key);
            }
        }
    }

    /// After a tree-replacing operation bumped the write epoch: drop the
    /// stale token and best-effort re-acquire (re-probes fidelity). If the
    /// replacement content is Degraded, no token returns and subsequent
    /// mutations refuse.
    pub fn refresh(&self, project_path: &Path) {
        self.invalidate(project_path);
        let _ = self.checkout(project_path);
    }
}

impl ProjectWriteLocks {
    pub fn with_project_lock<T>(
        &self,
        project_path: impl AsRef<Path>,
        f: impl FnOnce() -> Result<T, ChiknError>,
    ) -> Result<T, ChiknError> {
        let key = project_lock_key(project_path.as_ref())?;
        let lock = {
            let mut locks = self.locks.lock().map_err(|_| {
                ChiknError::Unknown("Project write lock registry is poisoned".to_string())
            })?;
            Arc::clone(locks.entry(key).or_default())
        };

        let _guard = lock
            .lock()
            .map_err(|_| ChiknError::Unknown("Project write lock is poisoned".to_string()))?;
        f()
    }
}

fn project_lock_key(path: &Path) -> Result<PathBuf, ChiknError> {
    match path.canonicalize() {
        Ok(path) => Ok(path),
        Err(_) if path.is_absolute() => Ok(path.to_path_buf()),
        Err(_) => Ok(std::env::current_dir()?.join(path)),
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectWriteLocks;
    use chickenscratch_core::ChiknError;
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn same_project_lock_serializes_operations() {
        let locks = Arc::new(ProjectWriteLocks::default());
        let project_path =
            std::env::temp_dir().join(format!("chickenscratch-lock-test-{}", uuid::Uuid::new_v4()));
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();

        let first_locks = Arc::clone(&locks);
        let first_path = project_path.clone();
        let first = std::thread::spawn(move || {
            first_locks
                .with_project_lock(&first_path, || {
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    Ok::<_, ChiknError>(())
                })
                .unwrap();
        });

        entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();

        let second_locks = Arc::clone(&locks);
        let second_path = project_path;
        let second = std::thread::spawn(move || {
            second_locks
                .with_project_lock(&second_path, || {
                    done_tx.send(()).unwrap();
                    Ok::<_, ChiknError>(())
                })
                .unwrap();
        });

        assert!(done_rx.recv_timeout(Duration::from_millis(100)).is_err());
        release_tx.send(()).unwrap();
        done_rx.recv_timeout(Duration::from_secs(1)).unwrap();

        first.join().unwrap();
        second.join().unwrap();
    }

    #[test]
    fn different_project_locks_do_not_block_each_other() {
        let locks = Arc::new(ProjectWriteLocks::default());
        let first_path = std::env::temp_dir().join(format!(
            "chickenscratch-lock-test-a-{}",
            uuid::Uuid::new_v4()
        ));
        let second_path = std::env::temp_dir().join(format!(
            "chickenscratch-lock-test-b-{}",
            uuid::Uuid::new_v4()
        ));
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();

        let first_locks = Arc::clone(&locks);
        let first = std::thread::spawn(move || {
            first_locks
                .with_project_lock(&first_path, || {
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    Ok::<_, ChiknError>(())
                })
                .unwrap();
        });

        entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();

        let second_locks = Arc::clone(&locks);
        let second = std::thread::spawn(move || {
            second_locks
                .with_project_lock(&second_path, || {
                    done_tx.send(()).unwrap();
                    Ok::<_, ChiknError>(())
                })
                .unwrap();
        });

        done_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        release_tx.send(()).unwrap();

        first.join().unwrap();
        second.join().unwrap();
    }
}
