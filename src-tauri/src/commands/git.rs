use chickenscratch_core::core::git;
use chickenscratch_core::core::project::fidelity;
use chickenscratch_core::ChiknError;
use std::path::Path;
use tauri::State;

use super::{ProjectTokens, ProjectWriteLocks};

#[tauri::command]
pub fn save_revision(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    message: String,
) -> Result<git::Revision, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let path = Path::new(&project_path);
        tokens.with_write_permit(path, |permit| {
            let rev = git::save_revision(path, &message, permit)?;

            // After named revision: push to backup remote and remote-sync if configured.
            // Both are fire-and-forget — a failed push must not fail the revision.
            let settings = super::settings::get_app_settings_hydrated();
            if let Some(ref backup_dir) = settings.backup.backup_directory {
                let _ = git::push_backup(path, Path::new(backup_dir), permit);
            }
            if settings.remote.auto_push_on_revision {
                if let Some(ref url) = settings.remote.url {
                    let auth = git::RemoteAuth {
                        username: settings.remote.username.clone(),
                        token: settings.remote.token.clone(),
                    };
                    let _ = git::push_remote(path, url, &auth, permit);
                }
            }

            Ok(rev)
        })
    })
}

#[tauri::command]
pub fn list_revisions(project_path: String) -> Result<Vec<git::Revision>, ChiknError> {
    git::list_revisions(Path::new(&project_path))
}

#[tauri::command]
pub fn restore_revision(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    commit_id: String,
) -> Result<git::Revision, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let result = tokens.with_write_permit(&project_path, |permit| {
            git::restore_revision(Path::new(&project_path), &commit_id, permit)
        });
        // Tree replaced on success: re-probe and reissue (or drop) the token.
        if result.is_ok() {
            tokens.refresh(Path::new(&project_path));
        }
        result
    })
}

#[tauri::command]
pub fn create_draft(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    name: String,
) -> Result<(), ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        tokens.with_write_permit(&project_path, |permit| {
            git::create_draft(Path::new(&project_path), &name, permit)
        })
    })
}

#[tauri::command]
pub fn list_drafts(project_path: String) -> Result<Vec<git::DraftVersion>, ChiknError> {
    git::list_drafts(Path::new(&project_path))
}

#[tauri::command]
pub fn switch_draft(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    name: String,
) -> Result<(), ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let result = tokens.with_write_permit(&project_path, |permit| {
            git::switch_draft(Path::new(&project_path), &name, permit)
        });
        if result.is_ok() {
            tokens.refresh(Path::new(&project_path));
        }
        result
    })
}

#[tauri::command]
pub fn merge_draft(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    name: String,
) -> Result<git::MergeResult, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let result = tokens.with_write_permit(&project_path, |permit| {
            git::merge_draft(Path::new(&project_path), &name, permit)
        });
        if result.is_ok() {
            tokens.refresh(Path::new(&project_path));
        }
        result
    })
}

#[tauri::command]
pub fn push_backup(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    backup_dir: String,
) -> Result<(), ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        tokens.with_write_permit(&project_path, |permit| {
            git::push_backup(Path::new(&project_path), Path::new(&backup_dir), permit)
        })
    })
}

#[tauri::command]
pub fn manual_backup(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    backup_dir: String,
) -> Result<Option<git::Revision>, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        tokens.with_write_permit(&project_path, |permit| {
            git::backup_current_work(
                Path::new(&project_path),
                Path::new(&backup_dir),
                "Manual backup",
                permit,
            )
        })
    })
}

/// Push the current branch to the remote URL configured in settings.
#[tauri::command]
pub fn sync_push(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
) -> Result<(), ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        tokens.with_write_permit(&project_path, |permit| {
            let (url, auth) = remote_from_settings()?;
            git::push_remote(Path::new(&project_path), &url, &auth, permit)
        })
    })
}

/// Fetch the current branch from the configured remote. Does not merge.
#[tauri::command]
pub fn sync_fetch(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
) -> Result<(), ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        tokens.with_write_permit(&project_path, |permit| {
            let (url, auth) = remote_from_settings()?;
            git::fetch_remote(Path::new(&project_path), &url, &auth, permit)
        })
    })
}

/// Ahead/behind counts relative to the last-fetched remote tracking ref.
#[tauri::command]
pub fn sync_status(project_path: String) -> Result<git::SyncStatus, ChiknError> {
    git::sync_status(Path::new(&project_path))
}

#[tauri::command]
pub fn document_history(
    project_path: String,
    doc_path: String,
) -> Result<Vec<git::Revision>, ChiknError> {
    git::document_history(Path::new(&project_path), &doc_path)
}

#[tauri::command]
pub fn restore_document(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    doc_path: String,
    commit_id: String,
) -> Result<git::Revision, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let result = tokens.with_write_permit(&project_path, |permit| {
            git::restore_document(Path::new(&project_path), &doc_path, &commit_id, permit)
        });
        if result.is_ok() {
            tokens.refresh(Path::new(&project_path));
        }
        result
    })
}

/// Pull (fetch + merge). Returns one of: up_to_date, fast_forward, merged,
/// conflicts (with file list). Conflicts leave the working tree with merge
/// markers; call `sync_abort_pull` to revert or `sync_pull_force` to discard
/// local changes.
#[tauri::command]
pub fn sync_pull(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
) -> Result<git::PullResult, ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let result = tokens.with_write_permit(&project_path, |permit| {
            let (url, auth) = remote_from_settings()?;
            git::sync_pull(Path::new(&project_path), &url, &auth, permit)
        });
        if result.is_ok() {
            tokens.refresh(Path::new(&project_path));
        }
        result
    })
}

/// Inner body shared with the command-boundary tests: the recovery
/// commands must be provably reachable from a FRESH process (fresh
/// `ProjectTokens`, simulating restart) while format-file conflicts make
/// the ordinary permit unobtainable — the exact live bug this slice fixes.
pub(crate) fn sync_abort_pull_inner(
    project_path: &str,
    write_locks: &ProjectWriteLocks,
    tokens: &ProjectTokens,
) -> Result<(), ChiknError> {
    write_locks.with_project_lock(project_path, || {
        // Recovery authority, not an ordinary permit: mid-merge the
        // fidelity probe fails by design when conflicts touch
        // project.yaml or a .meta — abort must stay reachable exactly
        // then (plan slice 4; this was a live bug).
        let path = Path::new(project_path);
        let recovery = fidelity::acquire_recovery_permit(path)?;
        let result = git::sync_abort_pull(path, &recovery);
        if result.is_ok() {
            tokens.refresh(path);
        }
        result
    })
}

#[tauri::command]
pub fn sync_abort_pull(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
) -> Result<(), ChiknError> {
    sync_abort_pull_inner(&project_path, &write_locks, &tokens)
}

/// Merge-state snapshot for the persistent merge-in-progress UI. Needs no
/// permit and no probe: it must answer even when format-file conflicts
/// make the project unopenable through the ordinary path.
#[tauri::command]
pub fn merge_state(project_path: String) -> Result<git::MergeState, ChiknError> {
    git::merge_state(Path::new(&project_path))
}

pub(crate) fn complete_merge_inner(
    project_path: &str,
    write_locks: &ProjectWriteLocks,
    tokens: &ProjectTokens,
    message: &str,
) -> Result<git::Revision, ChiknError> {
    write_locks.with_project_lock(project_path, || {
        let path = Path::new(project_path);
        let recovery = fidelity::acquire_recovery_permit(path)?;
        let result = git::complete_merge(path, message, &recovery);
        if result.is_ok() {
            tokens.refresh(path);
        }
        result
    })
}

/// Complete an in-progress merge after manual conflict resolution. The
/// writer's explicit action is the resolution signal — no automatic
/// caller can reach this (plan slice 4).
#[tauri::command]
pub fn complete_merge(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    message: String,
) -> Result<git::Revision, ChiknError> {
    complete_merge_inner(&project_path, &write_locks, &tokens, &message)
}

pub(crate) fn force_resolve_merge_inner(
    project_path: &str,
    write_locks: &ProjectWriteLocks,
    tokens: &ProjectTokens,
    attestation: &str,
) -> Result<(), ChiknError> {
    write_locks.with_project_lock(project_path, || {
        let path = Path::new(project_path);
        let recovery = fidelity::acquire_recovery_permit(path)?;
        let result = git::force_resolve_merge(path, &recovery, Some(attestation));
        if result.is_ok() {
            tokens.refresh(path);
        }
        result
    })
}

/// Resolve an in-progress merge by taking THEIRS (`MERGE_HEAD`) wholesale.
/// Serves both conflict origins — pull (MERGE_HEAD = fetched remote
/// commit) and draft merge (MERGE_HEAD = draft tip) — replacing the old
/// remote-only force path that could never run against a real conflict.
/// `attestation` is the merge state (from `merge_state`) the UI showed the
/// writer when they confirmed the discard; a live merge that no longer
/// matches refuses (finding s4-1).
#[tauri::command]
pub fn force_resolve_merge(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
    attestation: String,
) -> Result<(), ChiknError> {
    force_resolve_merge_inner(&project_path, &write_locks, &tokens, &attestation)
}

#[tauri::command]
pub fn sync_pull_force(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
) -> Result<(), ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let result = tokens.with_write_permit(&project_path, |permit| {
            let (url, auth) = remote_from_settings()?;
            git::sync_pull_force(Path::new(&project_path), &url, &auth, permit)
        });
        if result.is_ok() {
            tokens.refresh(Path::new(&project_path));
        }
        result
    })
}

fn remote_from_settings() -> Result<(String, git::RemoteAuth), ChiknError> {
    let settings = super::settings::get_app_settings_hydrated();
    let url = settings.remote.url.clone().ok_or_else(|| {
        ChiknError::Unknown("No remote URL configured. Set one in Settings > Remote.".to_string())
    })?;
    let auth = git::RemoteAuth {
        username: settings.remote.username.clone(),
        token: settings.remote.token.clone(),
    };
    Ok((url, auth))
}

#[tauri::command]
pub fn revision_diff(
    project_path: String,
    commit_id: String,
) -> Result<Vec<git::FileDiff>, ChiknError> {
    git::revision_diff(Path::new(&project_path), &commit_id)
}

#[tauri::command]
pub fn word_diff(
    project_path: String,
    commit_id: String,
    doc_path: String,
) -> Result<Vec<(String, String)>, ChiknError> {
    git::word_diff(Path::new(&project_path), &commit_id, &doc_path)
}

#[tauri::command]
pub fn compare_drafts(
    project_path: String,
    draft_a: String,
    draft_b: String,
) -> Result<Vec<git::FileDiff>, ChiknError> {
    git::compare_drafts(Path::new(&project_path), &draft_a, &draft_b)
}

#[tauri::command]
pub fn word_diff_drafts(
    project_path: String,
    draft_a: String,
    draft_b: String,
    doc_path: String,
) -> Result<Vec<(String, String)>, ChiknError> {
    git::word_diff_drafts(Path::new(&project_path), &draft_a, &draft_b, &doc_path)
}

#[tauri::command]
pub fn has_changes(project_path: String) -> Result<bool, ChiknError> {
    git::has_changes(Path::new(&project_path))
}

/// Auto-backup: save any unsaved changes and push to backup remote if configured.
/// Called on app close. Non-fatal — failures are logged but don't block exit.
#[tauri::command]
pub fn backup_on_close(
    project_path: String,
    write_locks: State<'_, ProjectWriteLocks>,
    tokens: State<'_, ProjectTokens>,
) -> Result<(), ChiknError> {
    write_locks.with_project_lock(&project_path, || {
        let path = Path::new(&project_path);
        let settings = super::settings::get_app_settings();

        // A Degraded project yields no permit: the close-time auto-save is
        // SKIPPED entirely, never surfaced as an error. This is the exact
        // path that once committed "Auto-save on close" over a legacy
        // project it had loaded as empty.
        tokens.with_write_permit(path, |permit| {
            // Auto-commit any uncommitted changes. An automatic writer
            // never commits mid-merge: save_revision refuses (plan
            // slice 4), and that refusal is skipped deliberately —
            // the merge-in-progress UI owns telling the writer — while
            // any OTHER save failure now propagates instead of
            // disappearing into a `let _` (review round 9).
            if git::has_changes(path).unwrap_or(false) {
                match git::save_revision(path, "Auto-save on close", permit) {
                    Ok(_) => {}
                    Err(ChiknError::ReadOnly(_)) => {}
                    Err(e) => return Err(e),
                }
            }

            // Push to backup remote if configured. The push is benign
            // mid-merge (branch ref only); failures don't block close.
            if settings.backup.auto_backup_on_close {
                if let Some(ref backup_dir) = settings.backup.backup_directory {
                    let _ = git::push_backup(path, Path::new(backup_dir), permit);
                }
            }

            Ok(())
        })
    })
}

#[cfg(test)]
mod tests {
    //! Command-boundary regressions (plan slice 4, rounds 9–12): every
    //! test starts from FRESH `ProjectTokens` — the restart simulation —
    //! with a merge conflict inside `project.yaml`, where the fidelity
    //! probe errors and NO ordinary permit can be issued. Before this
    //! slice that combination stranded the writer: the project would not
    //! even open, and abort/force were permit-gated (abort) or blocked by
    //! their own dirty checks (force). Each exit — abort, complete,
    //! force — must be reachable through the real command bodies.

    use super::super::project::load_project_inner;
    use super::super::{ProjectTokens, ProjectWriteLocks};
    use super::{complete_merge_inner, force_resolve_merge_inner, sync_abort_pull_inner};
    use chickenscratch_core::core::git;
    use chickenscratch_core::core::project::fidelity::acquire_write_token;
    use std::fs;
    use std::path::{Path, PathBuf};

    fn write_manifest(root: &Path, name: &str) {
        fs::write(
            root.join("project.yaml"),
            format!(
                "format_version: '1.2'\nid: prj\nname: {name}\ncreated: '2025-01-01T00:00:00Z'\nmodified: '2025-01-01T00:00:00Z'\nhierarchy:\n- type: Document\n  id: doc-one\n  name: One\n  path: manuscript/one.md\n"
            ),
        )
        .unwrap();
    }

    fn commit(root: &Path, message: &str) {
        acquire_write_token(root)
            .expect("write token")
            .with_write_permit(root, |permit| git::save_revision(root, message, permit))
            .expect("revision");
    }

    /// Project whose `project.yaml` carries live merge-conflict markers:
    /// the fidelity probe errors, `acquire_write_token` refuses, and only
    /// the recovery authority can act.
    fn conflicted_yaml_project(tmp: &tempfile::TempDir) -> PathBuf {
        let root = tmp.path().join("Novel.chikn");
        fs::create_dir_all(root.join("manuscript")).unwrap();
        git::init_repo(&root).unwrap();
        write_manifest(&root, "Test");
        fs::write(
            root.join("manuscript/one.meta"),
            "id: doc-one\ncreated: '2025-01-01T00:00:00Z'\nmodified: '2025-01-01T00:00:00Z'\n",
        )
        .unwrap();
        fs::write(root.join("manuscript/one.md"), "# One\n\nBase.\n").unwrap();
        commit(&root, "Initial");

        let original = git2::Repository::open(&root)
            .unwrap()
            .head()
            .unwrap()
            .shorthand()
            .unwrap()
            .to_string();
        acquire_write_token(&root)
            .unwrap()
            .with_write_permit(&root, |permit| git::create_draft(&root, "alt", permit))
            .unwrap();
        write_manifest(&root, "Draft Name");
        commit(&root, "Draft edit");
        acquire_write_token(&root)
            .unwrap()
            .with_write_permit(&root, |permit| git::switch_draft(&root, &original, permit))
            .unwrap();
        write_manifest(&root, "Master Name");
        commit(&root, "Master edit");
        let merged = acquire_write_token(&root)
            .unwrap()
            .with_write_permit(&root, |permit| git::merge_draft(&root, "alt", permit))
            .unwrap();
        assert!(
            matches!(merged, git::MergeResult::Conflicts { .. }),
            "fixture must conflict"
        );
        assert!(
            chickenscratch_core::core::project::fidelity::probe_project_fidelity(&root).is_err(),
            "fixture must make the probe error"
        );
        root
    }

    /// The restart boundary: a brand-new token registry, as after a crash
    /// or quit mid-merge.
    fn fresh_boundary() -> (ProjectWriteLocks, ProjectTokens) {
        (ProjectWriteLocks::default(), ProjectTokens::default())
    }

    #[test]
    fn conflicted_yaml_project_opens_read_only_and_aborts_from_fresh_boundary() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = conflicted_yaml_project(&tmp);
        let path = root.to_string_lossy().to_string();

        let (locks, tokens) = fresh_boundary();
        let loaded = load_project_inner(&path, &locks, &tokens)
            .expect("the open itself must succeed after restart");
        assert!(loaded.read_only);
        assert!(
            loaded
                .read_only_reasons
                .iter()
                .any(|r| r.contains("merge is in progress")),
            "the reason must tell the writer what to do: {:?}",
            loaded.read_only_reasons
        );
        assert_eq!(loaded.project.name, "Master Name", "metadata from HEAD");

        // The ordinary mutating path stays refused — with a conflicted
        // project.yaml the fidelity probe itself errors (parse failure),
        // so no permit is ever issued and the operation cannot start.
        let refused = tokens.with_write_permit(&root, |permit| {
            git::save_revision(&root, "must refuse", permit)
        });
        assert!(
            refused.is_err(),
            "ordinary mutating path must refuse mid-merge: {refused:?}"
        );

        // …while the recovery command reaches the abort.
        sync_abort_pull_inner(&path, &locks, &tokens).expect("abort via command boundary");

        let (locks, tokens) = fresh_boundary();
        let reloaded = load_project_inner(&path, &locks, &tokens).expect("reopen");
        assert!(!reloaded.read_only, "aborting restores a writable project");
        assert_eq!(reloaded.project.name, "Master Name");
    }

    #[test]
    fn conflicted_yaml_project_completes_from_fresh_boundary_after_resolution() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = conflicted_yaml_project(&tmp);
        let path = root.to_string_lossy().to_string();

        // The writer resolves project.yaml externally, restarts the app…
        write_manifest(&root, "Resolved Name");
        let (locks, tokens) = fresh_boundary();

        complete_merge_inner(&path, &locks, &tokens, "Merged name change")
            .expect("complete via command boundary");

        let repo = git2::Repository::open(&root).unwrap();
        assert_eq!(
            repo.head()
                .unwrap()
                .peel_to_commit()
                .unwrap()
                .parent_count(),
            2,
            "completion heals history with a true merge commit"
        );
        assert!(repo.find_reference("MERGE_HEAD").is_err());

        let (locks, tokens) = fresh_boundary();
        let reloaded = load_project_inner(&path, &locks, &tokens).expect("reopen");
        assert!(!reloaded.read_only);
        assert_eq!(reloaded.project.name, "Resolved Name");
    }

    #[test]
    fn conflicted_yaml_project_force_resolves_from_fresh_boundary() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = conflicted_yaml_project(&tmp);
        let path = root.to_string_lossy().to_string();

        let (locks, tokens) = fresh_boundary();
        // What the UI would have shown the writer at confirmation time.
        let confirmed = git::merge_state(&root)
            .unwrap()
            .attestation
            .expect("mid-merge fixture must expose an attestation");
        force_resolve_merge_inner(&path, &locks, &tokens, &confirmed)
            .expect("force via command boundary");

        let (locks, tokens) = fresh_boundary();
        let reloaded = load_project_inner(&path, &locks, &tokens).expect("reopen");
        assert!(!reloaded.read_only);
        assert_eq!(
            reloaded.project.name, "Draft Name",
            "force takes the incoming (MERGE_HEAD) version"
        );
    }
}
