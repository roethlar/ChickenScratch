//! Git integration for .chikn projects
//!
//! All revision control uses embedded git2 — no system git required.
//! Writers see "Save Revision" / "Revision History" — never "git".

use crate::utils::error::ChiknError;
use git2::{
    BranchType, Cred, DiffOptions, IndexAddOption, Oid, Repository, Signature, StatusOptions,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

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
    revwalk
        .push(head.target().unwrap())
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
    repo.checkout_tree(tree.as_object(), Some(git2::build::CheckoutBuilder::new().force()))
        .map_err(|e| ChiknError::Unknown(format!("Failed to restore: {}", e)))?;

    // Create a new commit on HEAD pointing to the restored state
    let msg = format!("Restored to: {}", commit.message().unwrap_or("(no message)"));
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

    let head_ref = repo.head().ok().and_then(|h| h.shorthand().map(String::from));

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
        .or_else(|_| Signature::now("Chicken Scratch", "writer@chickenscratch.app"))
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
