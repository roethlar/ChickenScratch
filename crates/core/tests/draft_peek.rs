//! Read-only draft peek integration tests (PLAN_DRAFT_PEEK.md, slice 1).
//!
//! GUARD: `list_draft_documents` and `read_draft_document` must be pure
//! object reads. The fingerprint assertions fail if any implementation of
//! the peek ever moves HEAD or touches the working tree (e.g. by checking
//! the draft out to read it).

use chickenscratch_core::core::git;
use chickenscratch_core::core::project::fidelity::{acquire_write_token, WriteToken};
use chickenscratch_core::test_support;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const MASTER_DOC: &str = "# One\n\nMaster version.\n";
const ALT_DOC: &str = "# One\n\nAlternate version.\n";

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

/// Healthy single-document project with two diverged drafts. Returns
/// `(root, original_branch_name)`; the project is left on the original
/// branch with `MASTER_DOC` on disk, while draft `alt` holds `ALT_DOC`.
fn init_two_draft_project(tmp: &TempDir) -> (PathBuf, String) {
    let root = tmp.path().join("Novel.chikn");
    fs::create_dir_all(root.join("manuscript")).unwrap();
    test_support::init_repo(&root).expect("init repo");
    fs::write(
        root.join("project.yaml"),
        "format_version: '1.2'\nid: prj\nname: Test\ncreated: '2025-01-01T00:00:00Z'\nmodified: '2025-01-01T00:00:00Z'\nhierarchy:\n- type: Document\n  id: doc-one\n  name: One\n  path: manuscript/one.md\n",
    )
    .unwrap();
    fs::write(
        root.join("manuscript/one.meta"),
        "id: doc-one\nname: One\ncreated: '2025-01-01T00:00:00Z'\nmodified: '2025-01-01T00:00:00Z'\n",
    )
    .unwrap();
    fs::write(root.join("manuscript/one.md"), "# One\n\nBase.\n").unwrap();
    with_permit!(&root, |permit| git::save_revision(&root, "Initial", permit))
        .expect("initial revision");

    let original = current_branch(&root);

    // `create_draft` switches to the new draft; put the alternate text there.
    with_permit!(&root, |permit| git::create_draft(&root, "alt", permit)).expect("create draft");
    fs::write(root.join("manuscript/one.md"), ALT_DOC).unwrap();
    with_permit!(&root, |permit| git::save_revision(
        &root, "Alt edit", permit
    ))
    .expect("alt revision");

    // Back to the original branch, then diverge it too.
    with_permit!(&root, |permit| git::switch_draft(&root, &original, permit)).expect("switch back");
    fs::write(root.join("manuscript/one.md"), MASTER_DOC).unwrap();
    with_permit!(&root, |permit| git::save_revision(
        &root,
        "Master edit",
        permit
    ))
    .expect("master revision");

    (root, original)
}

fn current_branch(root: &Path) -> String {
    git2::Repository::open(root)
        .unwrap()
        .head()
        .unwrap()
        .shorthand()
        .unwrap()
        .to_string()
}

/// Byte-level fingerprint of everything the peek must not touch: HEAD's
/// symbolic name + commit id, and every working-tree file outside `.git`.
fn fingerprint(root: &Path) -> (String, String, BTreeMap<PathBuf, Vec<u8>>) {
    let repo = git2::Repository::open(root).unwrap();
    let head = repo.head().unwrap();
    let head_name = head.shorthand().unwrap().to_string();
    let head_oid = head.peel_to_commit().unwrap().id().to_string();

    let mut files = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.file_name().is_some_and(|n| n == ".git") {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else {
                files.insert(path.clone(), fs::read(&path).unwrap());
            }
        }
    }
    (head_name, head_oid, files)
}

#[test]
fn peek_reads_other_draft_without_touching_worktree_or_head() {
    let tmp = TempDir::new().unwrap();
    let (root, original) = init_two_draft_project(&tmp);
    let before = fingerprint(&root);

    // Listing sees the other draft's documents, named from the sidecar.
    let docs = git::list_draft_documents(&root, "alt").expect("list alt");
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].path, "manuscript/one.md");
    assert_eq!(docs[0].name, "One");

    // Reading returns the *other* draft's text while the working tree keeps
    // the current draft's text.
    let text = git::read_draft_document(&root, "alt", "manuscript/one.md").expect("read alt");
    assert_eq!(text, ALT_DOC);
    assert_eq!(
        fs::read_to_string(root.join("manuscript/one.md")).unwrap(),
        MASTER_DOC
    );

    // GUARD: byte-identical worktree, same HEAD name and commit.
    assert_eq!(current_branch(&root), original);
    assert_eq!(before, fingerprint(&root), "peek mutated project state");
}

#[test]
fn peek_lists_only_documents() {
    let tmp = TempDir::new().unwrap();
    let (root, _) = init_two_draft_project(&tmp);

    let docs = git::list_draft_documents(&root, "alt").expect("list alt");
    let paths: Vec<&str> = docs.iter().map(|d| d.path.as_str()).collect();
    assert!(!paths.iter().any(|p| p.ends_with(".meta")));
    assert!(!paths.contains(&"project.yaml"));
}

#[test]
fn peek_errors_are_plain_results_not_panics() {
    let tmp = TempDir::new().unwrap();
    let (root, _) = init_two_draft_project(&tmp);

    assert!(git::list_draft_documents(&root, "no-such-draft").is_err());
    assert!(git::read_draft_document(&root, "alt", "manuscript/missing.md").is_err());
}

#[test]
fn peek_falls_back_to_file_stem_without_sidecar_name() {
    let tmp = TempDir::new().unwrap();
    let (root, original) = init_two_draft_project(&tmp);

    // Add a document with no `name:` in its sidecar on the current draft.
    fs::write(
        root.join("project.yaml"),
        "format_version: '1.2'\nid: prj\nname: Test\ncreated: '2025-01-01T00:00:00Z'\nmodified: '2025-01-01T00:00:00Z'\nhierarchy:\n- type: Document\n  id: doc-one\n  name: One\n  path: manuscript/one.md\n- type: Document\n  id: doc-two\n  name: Two\n  path: manuscript/two.md\n",
    )
    .unwrap();
    fs::write(
        root.join("manuscript/two.meta"),
        "id: doc-two\ncreated: '2025-01-01T00:00:00Z'\nmodified: '2025-01-01T00:00:00Z'\n",
    )
    .unwrap();
    fs::write(root.join("manuscript/two.md"), "# Two\n").unwrap();
    with_permit!(&root, |permit| git::save_revision(&root, "Add two", permit)).expect("save two");

    let docs = git::list_draft_documents(&root, &original).expect("list current");
    let two = docs
        .iter()
        .find(|d| d.path == "manuscript/two.md")
        .expect("two listed");
    assert_eq!(two.name, "two");
}
