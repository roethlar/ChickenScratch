//! Integration test: remote sync round-trip against a local bare repo.

use chickenscratch_core::core::git;
use chickenscratch_core::core::project::fidelity::{acquire_write_token, WriteToken};
use chickenscratch_core::{ChiknError, GitErrorKind};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Minimal probe-able manifest whose hierarchy references manuscript/one.md
/// — the shape every fixture in this file uses. Without it the fidelity
/// probe classifies one.md as an orphan (Degraded) and refuses a token.
fn write_manifest(path: &Path) {
    fs::write(
        path.join("project.yaml"),
        "id: test\nname: Test\ncreated: '2025-01-01T00:00:00Z'\nmodified: '2025-01-01T00:00:00Z'\nhierarchy:\n- type: Document\n  id: doc-one\n  name: One\n  path: manuscript/one.md\n",
    )
    .unwrap();
}

/// Manifest for fixtures with no documents at all.
fn write_empty_manifest(path: &Path) {
    fs::write(
        path.join("project.yaml"),
        "id: test\nname: Test\ncreated: '2025-01-01T00:00:00Z'\nmodified: '2025-01-01T00:00:00Z'\nhierarchy: []\n",
    )
    .unwrap();
}

/// Fresh write token for the current epoch. Acquired inline per call: the
/// tree-replacing git operations bump the epoch, so a token held across
/// them would be (correctly) refused as stale.
fn tk(path: &Path) -> WriteToken {
    acquire_write_token(path).expect("write token")
}

macro_rules! with_permit {
    ($path:expr, |$permit:ident| $operation:expr) => {{
        let operation_path: &Path = $path;
        let token = tk(operation_path);
        token.with_write_permit(operation_path, |$permit| $operation)
    }};
}

fn init_test_repo(path: &Path) {
    git::init_repo(path).expect("init repo");
    write_manifest(path);
    fs::create_dir_all(path.join("manuscript")).unwrap();
    fs::write(path.join("manuscript/one.md"), "# Chapter 1\n\nHello.\n").unwrap();
    with_permit!(path, |permit| git::save_revision(path, "Initial", permit))
        .expect("initial revision");
}

fn file_url(path: &Path) -> String {
    format!("file://{}", path.display())
}

fn current_branch(path: &Path) -> String {
    git2::Repository::open(path)
        .unwrap()
        .head()
        .unwrap()
        .shorthand()
        .unwrap()
        .to_string()
}

fn head_id(path: &Path) -> git2::Oid {
    git2::Repository::open(path)
        .unwrap()
        .head()
        .unwrap()
        .target()
        .unwrap()
}

fn assert_git_kind<T>(result: Result<T, ChiknError>, expected: GitErrorKind) {
    match result {
        Err(ChiknError::Git(err)) => assert_eq!(err.kind, expected, "{}", err.message),
        Err(err) => panic!("expected git error {expected:?}, got {err:?}"),
        Ok(_) => panic!("expected git error {expected:?}, got ok"),
    }
}

fn assert_git_kind_with_message<T>(
    result: Result<T, ChiknError>,
    expected: GitErrorKind,
    message_fragment: &str,
) {
    match result {
        Err(ChiknError::Git(err)) => {
            assert_eq!(err.kind, expected, "{}", err.message);
            assert!(
                err.message.contains(message_fragment),
                "expected message to contain {message_fragment:?}, got {:?}",
                err.message
            );
        }
        Err(err) => panic!("expected git error {expected:?}, got {err:?}"),
        Ok(_) => panic!("expected git error {expected:?}, got ok"),
    }
}

fn init_bare_repo(path: &Path) -> bool {
    let status = Command::new("git")
        .args(["init", "--bare"])
        .arg(path)
        .status()
        .expect("need system git");
    status.success()
}

#[test]
fn restore_document_rejects_dirty_worktree_without_clobbering_file() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("Novel.chikn");
    fs::create_dir_all(&project).unwrap();
    git::init_repo(&project).expect("init repo");

    let manuscript = project.join("manuscript/one.md");
    fs::create_dir_all(manuscript.parent().unwrap()).unwrap();
    fs::write(&manuscript, "# Chapter 1\n\nOriginal.\n").unwrap();
    write_manifest(&project);
    let original = with_permit!(&project, |permit| {
        git::save_revision(&project, "Original", permit)
    })
    .expect("original revision");

    fs::write(&manuscript, "# Chapter 1\n\nCommitted rewrite.\n").unwrap();
    with_permit!(&project, |permit| {
        git::save_revision(&project, "Rewrite", permit)
    })
    .expect("rewrite revision");

    let dirty_content = "# Chapter 1\n\nUnsaved typing.\n";
    fs::write(&manuscript, dirty_content).unwrap();

    assert_git_kind_with_message(
        with_permit!(&project, |permit| {
            git::restore_document(&project, "manuscript/one.md", &original.id, permit)
        }),
        GitErrorKind::Conflict,
        "unsaved changes",
    );
    assert_eq!(fs::read_to_string(&manuscript).unwrap(), dirty_content);
}

#[cfg(unix)]
#[test]
fn restore_document_rejects_symlink_document_without_touching_outside_file() {
    use std::os::unix::fs as unix_fs;

    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("Novel.chikn");
    fs::create_dir_all(&project).unwrap();
    git::init_repo(&project).expect("init repo");

    let manuscript = project.join("manuscript/one.md");
    fs::create_dir_all(manuscript.parent().unwrap()).unwrap();
    fs::write(&manuscript, "# Chapter 1\n\nOriginal.\n").unwrap();
    write_manifest(&project);
    // Acquired while the project is still healthy: after the symlink lands
    // the probe would classify it Degraded and refuse a fresh token, and
    // this test exercises the writer's own symlink rejection instead.
    let token = tk(&project);
    let original = token
        .with_write_permit(&project, |permit| {
            git::save_revision(&project, "Original", permit)
        })
        .expect("original revision");

    let outside = tmp.path().join("outside.md");
    fs::write(&outside, "outside original").unwrap();
    let result = token.with_write_permit(&project, |permit| {
        fs::remove_file(&manuscript)?;
        unix_fs::symlink(&outside, &manuscript)?;
        git::save_revision(&project, "Commit symlink target", permit)?;
        assert!(!git::has_changes(&project).expect("clean worktree"));
        git::restore_document(&project, "manuscript/one.md", &original.id, permit)
    });

    assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
    assert_eq!(fs::read_to_string(&outside).unwrap(), "outside original");
}

#[cfg(unix)]
#[test]
fn restore_document_rejects_symlink_meta_without_touching_outside_file() {
    use std::os::unix::fs as unix_fs;

    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("Novel.chikn");
    fs::create_dir_all(&project).unwrap();
    git::init_repo(&project).expect("init repo");

    let manuscript = project.join("manuscript/one.md");
    let meta = project.join("manuscript/one.meta");
    fs::create_dir_all(manuscript.parent().unwrap()).unwrap();
    fs::write(&manuscript, "# Chapter 1\n\nOriginal.\n").unwrap();
    fs::write(&meta, "id: doc-one\nname: One\nstatus: Original\n").unwrap();
    write_manifest(&project);
    // Acquired while the project is still healthy — see the sibling
    // symlink test for why.
    let token = tk(&project);
    let original = token
        .with_write_permit(&project, |permit| {
            git::save_revision(&project, "Original", permit)
        })
        .expect("original revision");

    fs::write(&manuscript, "# Chapter 1\n\nCurrent.\n").unwrap();
    let outside = tmp.path().join("outside.meta");
    fs::write(&outside, "outside original").unwrap();
    let result = token.with_write_permit(&project, |permit| {
        fs::remove_file(&meta)?;
        unix_fs::symlink(&outside, &meta)?;
        git::save_revision(&project, "Commit meta symlink", permit)?;
        assert!(!git::has_changes(&project).expect("clean worktree"));
        git::restore_document(&project, "manuscript/one.md", &original.id, permit)
    });

    assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
    assert_eq!(fs::read_to_string(&outside).unwrap(), "outside original");
    assert_eq!(
        fs::read_to_string(&manuscript).unwrap(),
        "# Chapter 1\n\nCurrent.\n"
    );
}

#[test]
fn restore_revision_rejects_dirty_worktree_without_clobbering_file() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("Novel.chikn");
    fs::create_dir_all(&project).unwrap();
    git::init_repo(&project).expect("init repo");

    let manuscript = project.join("manuscript/one.md");
    fs::create_dir_all(manuscript.parent().unwrap()).unwrap();
    fs::write(&manuscript, "# Chapter 1\n\nOriginal.\n").unwrap();
    write_manifest(&project);
    let original = with_permit!(&project, |permit| {
        git::save_revision(&project, "Original", permit)
    })
    .expect("original revision");

    fs::write(&manuscript, "# Chapter 1\n\nCommitted rewrite.\n").unwrap();
    with_permit!(&project, |permit| {
        git::save_revision(&project, "Rewrite", permit)
    })
    .expect("rewrite revision");

    let dirty_content = "# Chapter 1\n\nUnsaved typing.\n";
    fs::write(&manuscript, dirty_content).unwrap();

    assert_git_kind_with_message(
        with_permit!(&project, |permit| {
            git::restore_revision(&project, &original.id, permit)
        }),
        GitErrorKind::Conflict,
        "unsaved changes",
    );
    assert_eq!(fs::read_to_string(&manuscript).unwrap(), dirty_content);
}

#[test]
fn restore_revision_clean_worktree_restores_and_commits_forward() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("Novel.chikn");
    fs::create_dir_all(&project).unwrap();
    git::init_repo(&project).expect("init repo");

    let manuscript = project.join("manuscript/one.md");
    fs::create_dir_all(manuscript.parent().unwrap()).unwrap();
    fs::write(&manuscript, "# Chapter 1\n\nOriginal.\n").unwrap();
    write_manifest(&project);
    let original = with_permit!(&project, |permit| {
        git::save_revision(&project, "Original", permit)
    })
    .expect("original revision");

    fs::write(&manuscript, "# Chapter 1\n\nCommitted rewrite.\n").unwrap();
    with_permit!(&project, |permit| {
        git::save_revision(&project, "Rewrite", permit)
    })
    .expect("rewrite revision");

    let restored = with_permit!(&project, |permit| {
        git::restore_revision(&project, &original.id, permit)
    })
    .expect("restore revision");

    assert_eq!(
        fs::read_to_string(&manuscript).unwrap(),
        "# Chapter 1\n\nOriginal.\n"
    );
    assert_ne!(
        restored.id, original.id,
        "restore should create a new commit"
    );
    assert!(!git::has_changes(&project).expect("clean worktree"));
}

#[test]
fn create_draft_rejects_dirty_worktree_and_does_not_create_branch() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("Novel.chikn");
    fs::create_dir_all(&project).unwrap();
    init_test_repo(&project);

    let dirty_content = "# Chapter 1\n\nUnsaved typing.\n";
    fs::write(project.join("manuscript/one.md"), dirty_content).unwrap();

    assert_git_kind_with_message(
        with_permit!(&project, |permit| {
            git::create_draft(&project, "draft-2", permit)
        }),
        GitErrorKind::Conflict,
        "unsaved changes",
    );
    assert_eq!(
        fs::read_to_string(project.join("manuscript/one.md")).unwrap(),
        dirty_content
    );
    assert!(
        !git::list_drafts(&project)
            .unwrap()
            .iter()
            .any(|draft| draft.name == "draft-2"),
        "dirty create_draft should not leave a branch behind"
    );
}

#[test]
fn switch_draft_rejects_dirty_worktree_without_switching_or_clobbering() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("Novel.chikn");
    fs::create_dir_all(&project).unwrap();
    init_test_repo(&project);
    let main_branch = current_branch(&project);

    with_permit!(&project, |permit| {
        git::create_draft(&project, "draft-2", permit)
    })
    .expect("create draft");
    fs::write(
        project.join("manuscript/one.md"),
        "# Chapter 1\n\nDraft content.\n",
    )
    .unwrap();
    with_permit!(&project, |permit| {
        git::save_revision(&project, "Draft content", permit)
    })
    .expect("draft revision");
    with_permit!(&project, |permit| {
        git::switch_draft(&project, &main_branch, permit)
    })
    .expect("switch back to main");

    let dirty_content = "# Chapter 1\n\nUnsaved main typing.\n";
    fs::write(project.join("manuscript/one.md"), dirty_content).unwrap();

    assert_git_kind_with_message(
        with_permit!(&project, |permit| {
            git::switch_draft(&project, "draft-2", permit)
        }),
        GitErrorKind::Conflict,
        "unsaved changes",
    );
    assert_eq!(current_branch(&project), main_branch);
    assert_eq!(
        fs::read_to_string(project.join("manuscript/one.md")).unwrap(),
        dirty_content
    );
}

#[test]
fn merge_draft_fast_forward_rejects_dirty_worktree_without_advancing_head() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("Novel.chikn");
    fs::create_dir_all(&project).unwrap();
    init_test_repo(&project);
    let main_branch = current_branch(&project);

    with_permit!(&project, |permit| {
        git::create_draft(&project, "draft-2", permit)
    })
    .expect("create draft");
    fs::write(
        project.join("manuscript/one.md"),
        "# Chapter 1\n\nDraft content.\n",
    )
    .unwrap();
    with_permit!(&project, |permit| {
        git::save_revision(&project, "Draft content", permit)
    })
    .expect("draft revision");
    with_permit!(&project, |permit| {
        git::switch_draft(&project, &main_branch, permit)
    })
    .expect("switch back to main");

    let head_before = head_id(&project);
    let dirty_content = "# Chapter 1\n\nUnsaved main typing.\n";
    fs::write(project.join("manuscript/one.md"), dirty_content).unwrap();

    assert_git_kind_with_message(
        with_permit!(&project, |permit| {
            git::merge_draft(&project, "draft-2", permit)
        }),
        GitErrorKind::Conflict,
        "unsaved changes",
    );
    assert_eq!(head_id(&project), head_before);
    assert_eq!(
        fs::read_to_string(project.join("manuscript/one.md")).unwrap(),
        dirty_content
    );
}

#[test]
fn push_then_status_is_up_to_date() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("Novel.chikn");
    fs::create_dir_all(&project).unwrap();
    init_test_repo(&project);

    let remote = tmp.path().join("remote.git");
    let status = Command::new("git")
        .args(["init", "--bare"])
        .arg(&remote)
        .status()
        .expect("need system git for this test");
    if !status.success() {
        eprintln!("skipping: git init --bare failed");
        return;
    }

    let url = file_url(&remote);
    let auth = git::RemoteAuth::default();

    with_permit!(&project, |permit| {
        git::push_remote(&project, &url, &auth, permit)
    })
    .expect("push should succeed");
    with_permit!(&project, |permit| {
        git::fetch_remote(&project, &url, &auth, permit)
    })
    .expect("fetch should succeed");

    let status = git::sync_status(&project).expect("status");
    assert!(status.has_remote);
    assert_eq!(status.ahead, 0, "nothing ahead after fetch");
    assert_eq!(status.behind, 0, "nothing behind after fetch");
}

#[test]
fn push_without_commits_returns_no_commits_git_error() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("Novel.chikn");
    fs::create_dir_all(&project).unwrap();
    git::init_repo(&project).expect("init repo");
    write_empty_manifest(&project);

    let remote = tmp.path().join("remote.git");
    if !init_bare_repo(&remote) {
        return;
    }

    let url = file_url(&remote);
    assert_git_kind(
        with_permit!(&project, |permit| {
            git::push_remote(&project, &url, &git::RemoteAuth::default(), permit)
        }),
        GitErrorKind::NoCommits,
    );
}

#[test]
fn pull_empty_remote_returns_no_upstream_git_error() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("Novel.chikn");
    fs::create_dir_all(&project).unwrap();
    init_test_repo(&project);

    let remote = tmp.path().join("remote.git");
    if !init_bare_repo(&remote) {
        return;
    }

    let url = file_url(&remote);
    assert_git_kind(
        with_permit!(&project, |permit| {
            git::sync_pull(&project, &url, &git::RemoteAuth::default(), permit)
        }),
        GitErrorKind::NoUpstream,
    );
}

#[test]
fn fetch_missing_remote_returns_remote_unavailable_git_error() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("Novel.chikn");
    fs::create_dir_all(&project).unwrap();
    init_test_repo(&project);

    let missing_remote = tmp.path().join("missing.git");
    let url = file_url(&missing_remote);
    assert_git_kind(
        with_permit!(&project, |permit| {
            git::fetch_remote(&project, &url, &git::RemoteAuth::default(), permit)
        }),
        GitErrorKind::RemoteUnavailable,
    );
}

#[test]
fn push_diverged_remote_returns_not_fast_forward_git_error() {
    let tmp = tempfile::tempdir().unwrap();
    let project_a = tmp.path().join("A.chikn");
    fs::create_dir_all(&project_a).unwrap();
    init_test_repo(&project_a);

    let remote = tmp.path().join("remote.git");
    if !init_bare_repo(&remote) {
        return;
    }

    let url = file_url(&remote);
    let auth = git::RemoteAuth::default();
    with_permit!(&project_a, |permit| {
        git::push_remote(&project_a, &url, &auth, permit)
    })
    .expect("initial push should succeed");

    let project_b = tmp.path().join("B.chikn");
    let branch = git2::Repository::open(&project_a)
        .unwrap()
        .head()
        .unwrap()
        .shorthand()
        .unwrap()
        .to_string();
    git2::build::RepoBuilder::new()
        .branch(&branch)
        .clone(&url, &project_b)
        .expect("clone project");

    fs::write(
        project_b.join("manuscript/one.md"),
        "# Chapter 1\n\nChanged from B.\n",
    )
    .unwrap();
    with_permit!(&project_b, |permit| {
        git::save_revision(&project_b, "B rewrite", permit)
    })
    .unwrap();
    with_permit!(&project_b, |permit| {
        git::push_remote(&project_b, &url, &auth, permit)
    })
    .expect("B push should succeed");

    fs::write(
        project_a.join("manuscript/one.md"),
        "# Chapter 1\n\nChanged from A.\n",
    )
    .unwrap();
    with_permit!(&project_a, |permit| {
        git::save_revision(&project_a, "A rewrite", permit)
    })
    .unwrap();

    assert_git_kind(
        with_permit!(&project_a, |permit| {
            git::push_remote(&project_a, &url, &auth, permit)
        }),
        GitErrorKind::NotFastForward,
    );
}

#[test]
fn ahead_count_increases_after_new_revision() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("Novel.chikn");
    fs::create_dir_all(&project).unwrap();
    init_test_repo(&project);

    let remote = tmp.path().join("remote.git");
    let status = Command::new("git")
        .args(["init", "--bare"])
        .arg(&remote)
        .status()
        .expect("need system git");
    if !status.success() {
        return;
    }

    let url = file_url(&remote);
    let auth = git::RemoteAuth::default();

    with_permit!(&project, |permit| {
        git::push_remote(&project, &url, &auth, permit)
    })
    .unwrap();
    with_permit!(&project, |permit| {
        git::fetch_remote(&project, &url, &auth, permit)
    })
    .unwrap();

    fs::write(
        project.join("manuscript/one.md"),
        "# Chapter 1\n\nRewritten.\n",
    )
    .unwrap();
    with_permit!(&project, |permit| {
        git::save_revision(&project, "Rewrite", permit)
    })
    .unwrap();

    let s = git::sync_status(&project).expect("status");
    assert!(s.has_remote);
    assert_eq!(s.ahead, 1);
    assert_eq!(s.behind, 0);

    with_permit!(&project, |permit| {
        git::push_remote(&project, &url, &auth, permit)
    })
    .unwrap();
    with_permit!(&project, |permit| {
        git::fetch_remote(&project, &url, &auth, permit)
    })
    .unwrap();

    let s2 = git::sync_status(&project).expect("status");
    assert_eq!(s2.ahead, 0);
    assert_eq!(s2.behind, 0);
}

#[test]
fn sync_pull_fast_forward_rejects_dirty_worktree_without_advancing_head() {
    let tmp = tempfile::tempdir().unwrap();
    let project_a = tmp.path().join("A.chikn");
    fs::create_dir_all(&project_a).unwrap();
    init_test_repo(&project_a);

    let remote = tmp.path().join("remote.git");
    if !init_bare_repo(&remote) {
        return;
    }

    let url = file_url(&remote);
    let auth = git::RemoteAuth::default();
    with_permit!(&project_a, |permit| {
        git::push_remote(&project_a, &url, &auth, permit)
    })
    .expect("initial push should succeed");

    let project_b = tmp.path().join("B.chikn");
    let branch = current_branch(&project_a);
    git2::build::RepoBuilder::new()
        .branch(&branch)
        .clone(&url, &project_b)
        .expect("clone project");

    fs::write(
        project_b.join("manuscript/one.md"),
        "# Chapter 1\n\nRemote rewrite.\n",
    )
    .unwrap();
    with_permit!(&project_b, |permit| {
        git::save_revision(&project_b, "Remote rewrite", permit)
    })
    .unwrap();
    with_permit!(&project_b, |permit| {
        git::push_remote(&project_b, &url, &auth, permit)
    })
    .expect("remote push should succeed");

    let head_before = head_id(&project_a);
    let dirty_content = "# Chapter 1\n\nUnsaved local typing.\n";
    fs::write(project_a.join("manuscript/one.md"), dirty_content).unwrap();

    assert_git_kind_with_message(
        with_permit!(&project_a, |permit| {
            git::sync_pull(&project_a, &url, &auth, permit)
        }),
        GitErrorKind::Conflict,
        "unsaved changes",
    );
    assert_eq!(head_id(&project_a), head_before);
    assert_eq!(
        fs::read_to_string(project_a.join("manuscript/one.md")).unwrap(),
        dirty_content
    );
}

#[test]
fn force_pull_rejects_dirty_worktree_without_clobbering_file() {
    let tmp = tempfile::tempdir().unwrap();
    let project_a = tmp.path().join("A.chikn");
    fs::create_dir_all(&project_a).unwrap();
    init_test_repo(&project_a);

    let remote = tmp.path().join("remote.git");
    if !init_bare_repo(&remote) {
        return;
    }

    let url = file_url(&remote);
    let auth = git::RemoteAuth::default();
    with_permit!(&project_a, |permit| {
        git::push_remote(&project_a, &url, &auth, permit)
    })
    .expect("initial push should succeed");

    let project_b = tmp.path().join("B.chikn");
    let branch = git2::Repository::open(&project_a)
        .unwrap()
        .head()
        .unwrap()
        .shorthand()
        .unwrap()
        .to_string();
    git2::build::RepoBuilder::new()
        .branch(&branch)
        .clone(&url, &project_b)
        .expect("clone project");

    fs::write(
        project_b.join("manuscript/one.md"),
        "# Chapter 1\n\nRemote rewrite.\n",
    )
    .unwrap();
    with_permit!(&project_b, |permit| {
        git::save_revision(&project_b, "Remote rewrite", permit)
    })
    .unwrap();
    with_permit!(&project_b, |permit| {
        git::push_remote(&project_b, &url, &auth, permit)
    })
    .expect("remote push should succeed");

    let dirty_content = "# Chapter 1\n\nUnsaved local typing.\n";
    fs::write(project_a.join("manuscript/one.md"), dirty_content).unwrap();

    assert_git_kind_with_message(
        with_permit!(&project_a, |permit| {
            git::sync_pull_force(&project_a, &url, &auth, permit)
        }),
        GitErrorKind::Conflict,
        "unsaved changes",
    );
    assert_eq!(
        fs::read_to_string(project_a.join("manuscript/one.md")).unwrap(),
        dirty_content
    );
}

#[test]
fn force_pull_clean_worktree_overwrites_with_remote() {
    let tmp = tempfile::tempdir().unwrap();
    let project_a = tmp.path().join("A.chikn");
    fs::create_dir_all(&project_a).unwrap();
    init_test_repo(&project_a);

    let remote = tmp.path().join("remote.git");
    if !init_bare_repo(&remote) {
        return;
    }

    let url = file_url(&remote);
    let auth = git::RemoteAuth::default();
    with_permit!(&project_a, |permit| {
        git::push_remote(&project_a, &url, &auth, permit)
    })
    .expect("initial push should succeed");

    let project_b = tmp.path().join("B.chikn");
    let branch = git2::Repository::open(&project_a)
        .unwrap()
        .head()
        .unwrap()
        .shorthand()
        .unwrap()
        .to_string();
    git2::build::RepoBuilder::new()
        .branch(&branch)
        .clone(&url, &project_b)
        .expect("clone project");

    let remote_content = "# Chapter 1\n\nRemote rewrite.\n";
    fs::write(project_b.join("manuscript/one.md"), remote_content).unwrap();
    with_permit!(&project_b, |permit| {
        git::save_revision(&project_b, "Remote rewrite", permit)
    })
    .unwrap();
    with_permit!(&project_b, |permit| {
        git::push_remote(&project_b, &url, &auth, permit)
    })
    .expect("remote push should succeed");

    fs::write(
        project_a.join("manuscript/one.md"),
        "# Chapter 1\n\nCommitted local rewrite.\n",
    )
    .unwrap();
    with_permit!(&project_a, |permit| {
        git::save_revision(&project_a, "Local rewrite", permit)
    })
    .unwrap();

    with_permit!(&project_a, |permit| {
        git::sync_pull_force(&project_a, &url, &auth, permit)
    })
    .expect("force pull should succeed");

    assert_eq!(
        fs::read_to_string(project_a.join("manuscript/one.md")).unwrap(),
        remote_content
    );
    assert!(!git::has_changes(&project_a).expect("clean worktree"));
}
