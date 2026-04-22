//! Git integration for .chikn projects
//!
//! All revision control uses embedded git2 — no system git required.
//! Writers see "Save Revision" / "Revision History" — never "git".

use crate::utils::error::ChiknError;
use git2::{
    BranchType, Cred, DiffOptions, FetchOptions, IndexAddOption, Oid, PushOptions,
    RemoteCallbacks, Repository, Signature, StatusOptions,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Name of the git remote used for remote-sync (distinct from `backup` which is
/// the local-directory mirror remote).
const SYNC_REMOTE: &str = "sync";

/// A single revision (commit) in writer-friendly form
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Revision {
    pub id: String,
    pub message: String,
    pub timestamp: String,
    pub short_id: String,
}

/// A named draft version (branch)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftVersion {
    pub name: String,
    pub is_active: bool,
}

/// A file change in a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub status: String, // "added", "modified", "deleted"
}

/// Initialize a new git repo at the given path. No-op if already a repo.
pub fn init_repo(path: &Path) -> Result<Repository, ChiknError> {
    if path.join(".git").exists() {
        Repository::open(path)
            .map_err(|e| ChiknError::Unknown(format!("Failed to open git repo: {}", e)))
    } else {
        let repo = Repository::init(path)
            .map_err(|e| ChiknError::Unknown(format!("Failed to init git repo: {}", e)))?;

        // Write .gitignore
        let gitignore = path.join(".gitignore");
        if !gitignore.exists() {
            std::fs::write(
                &gitignore,
                "revs/\n.DS_Store\nThumbs.db\n*.tmp\n*.swp\n*~\n",
            )
            .ok();
        }

        Ok(repo)
    }
}

/// Stage all changes and commit with the given message.
pub fn save_revision(path: &Path, message: &str) -> Result<Revision, ChiknError> {
    let repo = Repository::open(path)
        .map_err(|e| ChiknError::Unknown(format!("Not a git repo: {}", e)))?;

    // Stage everything
    let mut index = repo
        .index()
        .map_err(|e| ChiknError::Unknown(format!("Failed to get index: {}", e)))?;
    index
        .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
        .map_err(|e| ChiknError::Unknown(format!("Failed to stage files: {}", e)))?;
    index
        .write()
        .map_err(|e| ChiknError::Unknown(format!("Failed to write index: {}", e)))?;

    let tree_id = index
        .write_tree()
        .map_err(|e| ChiknError::Unknown(format!("Failed to write tree: {}", e)))?;
    let tree = repo
        .find_tree(tree_id)
        .map_err(|e| ChiknError::Unknown(format!("Failed to find tree: {}", e)))?;

    let sig = default_signature(&repo)?;

    // Find parent commit (if any)
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.as_ref().map(|p| vec![p]).unwrap_or_default();

    let oid = repo
        .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
        .map_err(|e| ChiknError::Unknown(format!("Failed to commit: {}", e)))?;

    Ok(oid_to_revision(&repo, oid))
}

/// List all revisions (commits) on the current branch, newest first.
pub fn list_revisions(path: &Path) -> Result<Vec<Revision>, ChiknError> {
    let repo = Repository::open(path)
        .map_err(|e| ChiknError::Unknown(format!("Not a git repo: {}", e)))?;

    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return Ok(Vec::new()), // no commits yet
    };

    let mut revwalk = repo
        .revwalk()
        .map_err(|e| ChiknError::Unknown(format!("Failed to walk revisions: {}", e)))?;
    let head_target = match head.target() {
        Some(oid) => oid,
        None => return Ok(Vec::new()), // symbolic ref with no target
    };
    revwalk
        .push(head_target)
        .map_err(|e| ChiknError::Unknown(format!("Failed to push head: {}", e)))?;

    let mut revisions = Vec::new();
    for oid in revwalk {
        let oid =
            oid.map_err(|e| ChiknError::Unknown(format!("Failed to read revision: {}", e)))?;
        revisions.push(oid_to_revision(&repo, oid));
    }

    Ok(revisions)
}

/// Restore a previous revision by creating a new commit with that state.
/// Never rewrites history — always moves forward.
pub fn restore_revision(path: &Path, commit_id: &str) -> Result<Revision, ChiknError> {
    let repo = Repository::open(path)
        .map_err(|e| ChiknError::Unknown(format!("Not a git repo: {}", e)))?;

    let oid = Oid::from_str(commit_id)
        .map_err(|e| ChiknError::Unknown(format!("Invalid revision ID: {}", e)))?;
    let commit = repo
        .find_commit(oid)
        .map_err(|e| ChiknError::Unknown(format!("Revision not found: {}", e)))?;
    let tree = commit
        .tree()
        .map_err(|e| ChiknError::Unknown(format!("Failed to get tree: {}", e)))?;

    // Checkout that tree into the working directory
    repo.checkout_tree(
        tree.as_object(),
        Some(git2::build::CheckoutBuilder::new().force()),
    )
    .map_err(|e| ChiknError::Unknown(format!("Failed to restore: {}", e)))?;

    // Create a new commit on HEAD pointing to the restored state
    let msg = format!(
        "Restored to: {}",
        commit.message().unwrap_or("(no message)")
    );
    save_revision(path, &msg)
}

/// Create a new draft version (branch).
pub fn create_draft(path: &Path, name: &str) -> Result<(), ChiknError> {
    let repo = Repository::open(path)
        .map_err(|e| ChiknError::Unknown(format!("Not a git repo: {}", e)))?;

    let head = repo
        .head()
        .map_err(|e| ChiknError::Unknown(format!("No commits yet: {}", e)))?;
    let commit = head
        .peel_to_commit()
        .map_err(|e| ChiknError::Unknown(format!("Failed to find head commit: {}", e)))?;

    repo.branch(name, &commit, false)
        .map_err(|e| ChiknError::Unknown(format!("Failed to create draft: {}", e)))?;

    // Switch to the new branch
    let refname = format!("refs/heads/{}", name);
    repo.set_head(&refname)
        .map_err(|e| ChiknError::Unknown(format!("Failed to switch draft: {}", e)))?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .map_err(|e| ChiknError::Unknown(format!("Failed to checkout: {}", e)))?;

    Ok(())
}

/// List all draft versions (branches).
pub fn list_drafts(path: &Path) -> Result<Vec<DraftVersion>, ChiknError> {
    let repo = Repository::open(path)
        .map_err(|e| ChiknError::Unknown(format!("Not a git repo: {}", e)))?;

    let head_ref = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(String::from));

    let branches = repo
        .branches(Some(BranchType::Local))
        .map_err(|e| ChiknError::Unknown(format!("Failed to list drafts: {}", e)))?;

    let mut drafts = Vec::new();
    for branch in branches {
        let (branch, _) =
            branch.map_err(|e| ChiknError::Unknown(format!("Failed to read branch: {}", e)))?;
        if let Some(name) = branch.name().ok().flatten() {
            drafts.push(DraftVersion {
                is_active: head_ref.as_deref() == Some(name),
                name: name.to_string(),
            });
        }
    }

    Ok(drafts)
}

/// Switch to a different draft version (branch).
pub fn switch_draft(path: &Path, name: &str) -> Result<(), ChiknError> {
    let repo = Repository::open(path)
        .map_err(|e| ChiknError::Unknown(format!("Not a git repo: {}", e)))?;

    let refname = format!("refs/heads/{}", name);
    repo.set_head(&refname)
        .map_err(|e| ChiknError::Unknown(format!("Draft not found: {}", e)))?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .map_err(|e| ChiknError::Unknown(format!("Failed to switch: {}", e)))?;

    Ok(())
}

/// Merge a draft branch back into the current branch.
pub fn merge_draft(path: &Path, name: &str) -> Result<(), ChiknError> {
    let repo = Repository::open(path)
        .map_err(|e| ChiknError::Unknown(format!("Not a git repo: {}", e)))?;

    let branch = repo
        .find_branch(name, BranchType::Local)
        .map_err(|e| ChiknError::Unknown(format!("Draft not found: {}", e)))?;
    let commit = branch
        .get()
        .peel_to_commit()
        .map_err(|e| ChiknError::Unknown(format!("Failed to find branch commit: {}", e)))?;

    let annotated = repo
        .find_annotated_commit(commit.id())
        .map_err(|e| ChiknError::Unknown(format!("Failed to annotate commit: {}", e)))?;

    repo.merge(&[&annotated], None, None)
        .map_err(|e| ChiknError::Unknown(format!("Merge failed: {}", e)))?;

    // Commit the merge
    save_revision(path, &format!("Merged draft: {}", name))?;

    // Clean up merge state
    repo.cleanup_state()
        .map_err(|e| ChiknError::Unknown(format!("Failed to cleanup: {}", e)))?;

    Ok(())
}

/// Push to a backup remote. Creates the bare repo and remote if needed.
pub fn push_backup(project_path: &Path, backup_dir: &Path) -> Result<(), ChiknError> {
    let repo = Repository::open(project_path)
        .map_err(|e| ChiknError::Unknown(format!("Not a git repo: {}", e)))?;

    // Derive backup repo path from project name
    let project_name = project_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("project");
    let bare_path = backup_dir.join(format!("{}.git", project_name));

    // Create bare repo if it doesn't exist
    if !bare_path.exists() {
        std::fs::create_dir_all(&bare_path).ok();
        Repository::init_bare(&bare_path)
            .map_err(|e| ChiknError::Unknown(format!("Failed to create backup repo: {}", e)))?;
    }

    // Add or update remote
    let remote_url = bare_path.to_string_lossy().to_string();
    let mut remote = match repo.find_remote("backup") {
        Ok(r) => r,
        Err(_) => repo
            .remote("backup", &remote_url)
            .map_err(|e| ChiknError::Unknown(format!("Failed to add remote: {}", e)))?,
    };

    // Push current branch
    let head = repo.head().ok();
    let refname = head
        .as_ref()
        .and_then(|h| h.name())
        .unwrap_or("refs/heads/main");

    let refspec = format!("{}:{}", refname, refname);
    remote
        .push(&[&refspec], None)
        .map_err(|e| ChiknError::Unknown(format!("Backup push failed: {}", e)))?;

    Ok(())
}

// ── Remote sync ──────────────────────────────────────────────────────────────
//
// Push/fetch against an arbitrary git URL (GitHub, Gitea, self-hosted, or a
// `file://` path for local testing). Separate from `push_backup`, which mirrors
// the repo into a local directory. The remote is named `sync` so it coexists
// with `backup` and any user-managed `origin`.

#[derive(Debug, Clone, Default)]
pub struct RemoteAuth {
    pub username: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub ahead: usize,
    pub behind: usize,
    pub branch: String,
    pub has_remote: bool,
}

fn ensure_sync_remote<'a>(repo: &'a Repository, url: &str) -> Result<git2::Remote<'a>, ChiknError> {
    match repo.find_remote(SYNC_REMOTE) {
        Ok(existing) => {
            if existing.url() != Some(url) {
                repo.remote_set_url(SYNC_REMOTE, url).map_err(|e| {
                    ChiknError::Unknown(format!("Failed to update sync remote URL: {}", e))
                })?;
            }
            repo.find_remote(SYNC_REMOTE)
                .map_err(|e| ChiknError::Unknown(format!("Failed to load sync remote: {}", e)))
        }
        Err(_) => repo
            .remote(SYNC_REMOTE, url)
            .map_err(|e| ChiknError::Unknown(format!("Failed to add sync remote: {}", e))),
    }
}

fn build_callbacks(auth: &RemoteAuth) -> RemoteCallbacks<'_> {
    let mut cb = RemoteCallbacks::new();
    cb.credentials(move |url, username_from_url, allowed| {
        // HTTPS personal access token — most common case for GitHub/Gitea.
        if allowed.is_user_pass_plaintext() {
            if let (Some(user), Some(token)) = (auth.username.as_deref(), auth.token.as_deref()) {
                return Cred::userpass_plaintext(user, token);
            }
            if let Some(token) = auth.token.as_deref() {
                // GitHub accepts any non-empty username with a PAT; fall back to "git".
                let user = username_from_url.unwrap_or("git");
                return Cred::userpass_plaintext(user, token);
            }
        }
        // SSH (git@host:...) — try the agent first if auth is allowed.
        if allowed.is_ssh_key() {
            let user = username_from_url.unwrap_or("git");
            return Cred::ssh_key_from_agent(user);
        }
        // Anonymous / file:// — fall through with an error git2 will translate.
        Cred::default().map_err(|_| {
            git2::Error::from_str(&format!(
                "No credentials available for {} (allowed: {:?})",
                url, allowed
            ))
        })
    });
    cb
}

fn current_branch_name(repo: &Repository) -> Result<String, ChiknError> {
    let head = repo
        .head()
        .map_err(|e| ChiknError::Unknown(format!("No HEAD: {}", e)))?;
    head.shorthand()
        .map(str::to_owned)
        .ok_or_else(|| ChiknError::Unknown("HEAD is detached".to_string()))
}

/// Push the current branch to the configured remote.
pub fn push_remote(project_path: &Path, url: &str, auth: &RemoteAuth) -> Result<(), ChiknError> {
    let repo = Repository::open(project_path)
        .map_err(|e| ChiknError::Unknown(format!("Not a git repo: {}", e)))?;
    let mut remote = ensure_sync_remote(&repo, url)?;

    let branch = current_branch_name(&repo)?;
    let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");

    let mut opts = PushOptions::new();
    opts.remote_callbacks(build_callbacks(auth));

    remote
        .push(&[&refspec], Some(&mut opts))
        .map_err(|e| ChiknError::Unknown(format!("Push failed: {}", e)))?;
    Ok(())
}

/// Fetch the current branch from the configured remote. Does not merge.
pub fn fetch_remote(project_path: &Path, url: &str, auth: &RemoteAuth) -> Result<(), ChiknError> {
    let repo = Repository::open(project_path)
        .map_err(|e| ChiknError::Unknown(format!("Not a git repo: {}", e)))?;
    let mut remote = ensure_sync_remote(&repo, url)?;

    let branch = current_branch_name(&repo)?;
    let refspec = format!("+refs/heads/{branch}:refs/remotes/{SYNC_REMOTE}/{branch}");

    let mut opts = FetchOptions::new();
    opts.remote_callbacks(build_callbacks(auth));

    remote
        .fetch(&[&refspec], Some(&mut opts), None)
        .map_err(|e| ChiknError::Unknown(format!("Fetch failed: {}", e)))?;
    Ok(())
}

/// Compare local HEAD against the last fetched remote tracking ref.
/// Call `fetch_remote` first for the numbers to be current.
pub fn sync_status(project_path: &Path) -> Result<SyncStatus, ChiknError> {
    let repo = Repository::open(project_path)
        .map_err(|e| ChiknError::Unknown(format!("Not a git repo: {}", e)))?;
    let branch = current_branch_name(&repo)?;

    let has_remote = repo.find_remote(SYNC_REMOTE).is_ok();
    if !has_remote {
        return Ok(SyncStatus {
            ahead: 0,
            behind: 0,
            branch,
            has_remote: false,
        });
    }

    let local_oid = match repo
        .refname_to_id(&format!("refs/heads/{branch}"))
        .or_else(|_| repo.refname_to_id("HEAD"))
    {
        Ok(id) => id,
        Err(_) => {
            return Ok(SyncStatus {
                ahead: 0,
                behind: 0,
                branch,
                has_remote: true,
            })
        }
    };

    let remote_ref = format!("refs/remotes/{SYNC_REMOTE}/{branch}");
    let (ahead, behind) = match repo.refname_to_id(&remote_ref) {
        Ok(remote_oid) => repo.graph_ahead_behind(local_oid, remote_oid).map_err(|e| {
            ChiknError::Unknown(format!("Failed to compute ahead/behind: {}", e))
        })?,
        // No fetch has ever happened — every commit is "ahead", nothing "behind".
        Err(_) => {
            let mut walk = repo
                .revwalk()
                .map_err(|e| ChiknError::Unknown(format!("revwalk failed: {}", e)))?;
            walk.push(local_oid)
                .map_err(|e| ChiknError::Unknown(format!("revwalk push failed: {}", e)))?;
            (walk.count(), 0)
        }
    };

    Ok(SyncStatus {
        ahead,
        behind,
        branch,
        has_remote: true,
    })
}

/// Get files changed in a specific revision compared to its parent.
pub fn revision_diff(path: &Path, commit_id: &str) -> Result<Vec<FileDiff>, ChiknError> {
    let repo = Repository::open(path)
        .map_err(|e| ChiknError::Unknown(format!("Not a git repo: {}", e)))?;
    let oid = Oid::from_str(commit_id)
        .map_err(|e| ChiknError::Unknown(format!("Invalid commit ID: {}", e)))?;
    let commit = repo
        .find_commit(oid)
        .map_err(|e| ChiknError::Unknown(format!("Commit not found: {}", e)))?;
    let tree = commit
        .tree()
        .map_err(|e| ChiknError::Unknown(format!("Failed to get tree: {}", e)))?;
    let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());

    let diff = repo
        .diff_tree_to_tree(
            parent_tree.as_ref(),
            Some(&tree),
            Some(&mut DiffOptions::new()),
        )
        .map_err(|e| ChiknError::Unknown(format!("Failed to compute diff: {}", e)))?;

    let mut files = Vec::new();
    for delta in diff.deltas() {
        let path_str = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string();
        if path_str == "project.yaml" || path_str.starts_with(".git") || path_str.ends_with(".meta")
        {
            continue;
        }
        let status = match delta.status() {
            git2::Delta::Added => "added",
            git2::Delta::Deleted => "deleted",
            git2::Delta::Modified => "modified",
            git2::Delta::Renamed => "renamed",
            _ => "changed",
        };
        files.push(FileDiff {
            path: path_str,
            status: status.to_string(),
        });
    }
    Ok(files)
}

/// Get word-level diff of a specific document between two revisions.
/// Returns a list of (change_type, text) pairs for rendering tracked changes.
pub fn word_diff(
    path: &Path,
    commit_id: &str,
    doc_path: &str,
) -> Result<Vec<(String, String)>, ChiknError> {
    let repo = Repository::open(path)
        .map_err(|e| ChiknError::Unknown(format!("Not a git repo: {}", e)))?;

    let oid = Oid::from_str(commit_id)
        .map_err(|e| ChiknError::Unknown(format!("Invalid commit ID: {}", e)))?;
    let commit = repo
        .find_commit(oid)
        .map_err(|e| ChiknError::Unknown(format!("Commit not found: {}", e)))?;
    let tree = commit
        .tree()
        .map_err(|e| ChiknError::Unknown(format!("Failed to get tree: {}", e)))?;

    // Get the file content at this commit
    let new_content = tree
        .get_path(std::path::Path::new(doc_path))
        .ok()
        .and_then(|entry| repo.find_blob(entry.id()).ok())
        .map(|blob| String::from_utf8_lossy(blob.content()).to_string())
        .unwrap_or_default();

    // Get the file content at the parent commit
    let old_content = commit
        .parent(0)
        .ok()
        .and_then(|p| p.tree().ok())
        .and_then(|t| t.get_path(std::path::Path::new(doc_path)).ok())
        .and_then(|entry| repo.find_blob(entry.id()).ok())
        .map(|blob| String::from_utf8_lossy(blob.content()).to_string())
        .unwrap_or_default();

    // Strip HTML and compute word-level diff
    let old_words = strip_html_words(&old_content);
    let new_words = strip_html_words(&new_content);

    Ok(simple_word_diff(&old_words, &new_words))
}

fn strip_html_words(html: &str) -> Vec<String> {
    let mut text = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                text.push(' ');
            }
            _ if !in_tag => text.push(ch),
            _ => {}
        }
    }
    text.split_whitespace().map(|s| s.to_string()).collect()
}

/// Simple longest common subsequence word diff.
/// Returns vec of ("equal"|"added"|"deleted", text)
fn simple_word_diff(old: &[String], new: &[String]) -> Vec<(String, String)> {
    // For performance, limit to reasonable sizes
    if old.len() > 5000 || new.len() > 5000 {
        return vec![
            ("deleted".to_string(), old.join(" ")),
            ("added".to_string(), new.join(" ")),
        ];
    }

    // Build LCS table
    let m = old.len();
    let n = new.len();
    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if old[i - 1] == new[j - 1] {
                dp[i - 1][j - 1] + 1
            } else {
                dp[i - 1][j].max(dp[i][j - 1])
            };
        }
    }

    // Backtrack to produce diff
    let mut result: Vec<(String, String)> = Vec::new();
    let mut i = m;
    let mut j = n;
    let mut buf: Vec<(String, String)> = Vec::new();

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old[i - 1] == new[j - 1] {
            buf.push(("equal".to_string(), old[i - 1].clone()));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            buf.push(("added".to_string(), new[j - 1].clone()));
            j -= 1;
        } else {
            buf.push(("deleted".to_string(), old[i - 1].clone()));
            i -= 1;
        }
    }

    buf.reverse();

    // Merge consecutive same-type spans
    for (kind, word) in buf {
        if let Some(last) = result.last_mut() {
            if last.0 == kind {
                last.1.push(' ');
                last.1.push_str(&word);
                continue;
            }
        }
        result.push((kind, word));
    }

    result
}

/// Check if the working tree has uncommitted changes.
pub fn has_changes(path: &Path) -> Result<bool, ChiknError> {
    let repo = Repository::open(path)
        .map_err(|e| ChiknError::Unknown(format!("Not a git repo: {}", e)))?;

    let mut opts = StatusOptions::new();
    opts.include_untracked(true);

    let statuses = repo
        .statuses(Some(&mut opts))
        .map_err(|e| ChiknError::Unknown(format!("Failed to get status: {}", e)))?;

    Ok(!statuses.is_empty())
}

fn default_signature(repo: &Repository) -> Result<Signature<'static>, ChiknError> {
    // Try repo config first, fall back to generic
    repo.signature()
        .or_else(|_| Signature::now("ChickenScratch", "writer@chickenscratch.app"))
        .map_err(|e| ChiknError::Unknown(format!("Failed to create signature: {}", e)))
}

fn oid_to_revision(repo: &Repository, oid: Oid) -> Revision {
    let commit = repo.find_commit(oid).ok();
    let message = commit
        .as_ref()
        .and_then(|c| c.message())
        .unwrap_or("")
        .trim()
        .to_string();
    let timestamp = commit
        .as_ref()
        .map(|c| {
            let time = c.time();
            chrono::DateTime::from_timestamp(time.seconds(), 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        })
        .unwrap_or_default();

    Revision {
        id: oid.to_string(),
        short_id: oid.to_string()[..8].to_string(),
        message,
        timestamp,
    }
}
