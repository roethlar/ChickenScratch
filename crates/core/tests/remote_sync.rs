//! Integration test: remote sync round-trip against a local bare repo.

use chickenscratch_core::core::git;
use chickenscratch_core::{ChiknError, GitErrorKind};
use std::fs;
use std::path::Path;
use std::process::Command;

fn init_test_repo(path: &Path) {
    git::init_repo(path).expect("init repo");
    fs::write(path.join("project.yaml"), "id: test\nname: Test\n").unwrap();
    fs::create_dir_all(path.join("manuscript")).unwrap();
    fs::write(path.join("manuscript/one.md"), "# Chapter 1\n\nHello.\n").unwrap();
    git::save_revision(path, "Initial").expect("initial revision");
}

fn file_url(path: &Path) -> String {
    format!("file://{}", path.display())
}

fn assert_git_kind<T>(result: Result<T, ChiknError>, expected: GitErrorKind) {
    match result {
        Err(ChiknError::Git(err)) => assert_eq!(err.kind, expected, "{}", err.message),
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

    git::push_remote(&project, &url, &auth).expect("push should succeed");
    git::fetch_remote(&project, &url, &auth).expect("fetch should succeed");

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

    let remote = tmp.path().join("remote.git");
    if !init_bare_repo(&remote) {
        return;
    }

    let url = file_url(&remote);
    assert_git_kind(
        git::push_remote(&project, &url, &git::RemoteAuth::default()),
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
        git::sync_pull(&project, &url, &git::RemoteAuth::default()),
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
        git::fetch_remote(&project, &url, &git::RemoteAuth::default()),
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
    git::push_remote(&project_a, &url, &auth).expect("initial push should succeed");

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
    git::save_revision(&project_b, "B rewrite").unwrap();
    git::push_remote(&project_b, &url, &auth).expect("B push should succeed");

    fs::write(
        project_a.join("manuscript/one.md"),
        "# Chapter 1\n\nChanged from A.\n",
    )
    .unwrap();
    git::save_revision(&project_a, "A rewrite").unwrap();

    assert_git_kind(
        git::push_remote(&project_a, &url, &auth),
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

    git::push_remote(&project, &url, &auth).unwrap();
    git::fetch_remote(&project, &url, &auth).unwrap();

    fs::write(
        project.join("manuscript/one.md"),
        "# Chapter 1\n\nRewritten.\n",
    )
    .unwrap();
    git::save_revision(&project, "Rewrite").unwrap();

    let s = git::sync_status(&project).expect("status");
    assert!(s.has_remote);
    assert_eq!(s.ahead, 1);
    assert_eq!(s.behind, 0);

    git::push_remote(&project, &url, &auth).unwrap();
    git::fetch_remote(&project, &url, &auth).unwrap();

    let s2 = git::sync_status(&project).expect("status");
    assert_eq!(s2.ahead, 0);
    assert_eq!(s2.behind, 0);
}
