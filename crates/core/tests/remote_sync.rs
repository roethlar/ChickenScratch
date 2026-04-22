//! Integration test: remote sync round-trip against a local bare repo.

use chickenscratch_core::core::git;
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

    fs::write(project.join("manuscript/one.md"), "# Chapter 1\n\nRewritten.\n").unwrap();
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
