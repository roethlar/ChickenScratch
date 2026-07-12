//! Write-guard integration tests (PLAN_TRUST_FOUNDATIONS.md, Slice 1).
//!
//! The engine must never save over a project it cannot fully read. Each
//! Degraded fixture asserts, in order:
//!   1. the fidelity probe classifies it Degraded with the right reason and
//!      is byte-for-byte side-effect-free;
//!   2. the public default (Degraded) open is byte-for-byte
//!      side-effect-free;
//!   3. every mutating engine path refuses, and the folder stays
//!      byte-identical after the attempts — INCLUDING the project-internal
//!      app-file path used by the Statistics panel's writing history.
//!
//! GUARD-PROOF DRILLS (recorded in DEVLOG): bypassing fresh permit probing
//! lets an old Full session enter every representative mutation below after
//! external degradation, and the ReadOnly/byte-identity assertions fail.
//! Rerouting the public read through disk repair likewise fails its pure-read
//! guards. Restoring both boundaries makes the suite pass.

use chickenscratch_core::core::git;
use chickenscratch_core::core::project::fidelity::{
    acquire_write_token, probe_project_fidelity, DegradedReason, Fidelity,
};
use chickenscratch_core::core::project::reader::{
    read_project, read_project_readonly, read_project_with_repair,
};
use chickenscratch_core::core::project::writer::{
    self, create_project, delete_document, write_project,
};
use chickenscratch_core::ChiknError;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Byte-exact snapshot of every file (and directory name) under `root`.
/// Equality of two snapshots proves nothing was created, renamed, or
/// rewritten.
fn tree_snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    snapshot_inner(root, root, &mut out);
    out
}

fn snapshot_inner(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let metadata = fs::symlink_metadata(&path).unwrap();
        if metadata.is_dir() {
            out.insert(format!("{rel}/"), Vec::new());
            snapshot_inner(root, &path, out);
        } else if metadata.is_file() {
            out.insert(rel, fs::read(&path).unwrap());
        }
    }
}

/// Base fixture: a healthy single-document project written raw on disk
/// (all standard folders present, sidecar id matching the hierarchy).
fn base_fixture() -> (TempDir, PathBuf) {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("Fixture.chikn");
    fs::create_dir(&root).unwrap();
    for folder in ["manuscript", "research", "templates", "settings"] {
        fs::create_dir(root.join(folder)).unwrap();
    }
    fs::write(
        root.join("project.yaml"),
        r#"format_version: '1.2'
id: "prj"
name: "Fixture"
created: "2025-01-01T00:00:00Z"
modified: "2025-01-01T00:00:00Z"
hierarchy:
  - type: Document
    id: "doc1"
    name: "Chapter 1"
    path: "manuscript/chapter-01.md"
"#,
    )
    .unwrap();
    fs::write(root.join("manuscript/chapter-01.md"), "# Chapter 1\n").unwrap();
    fs::write(
        root.join("manuscript/chapter-01.meta"),
        "id: \"doc1\"\ncreated: \"2025-01-01T00:00:00Z\"\nmodified: \"2025-01-01T00:00:00Z\"\n",
    )
    .unwrap();
    (temp, root)
}

/// Fixture (a): April-era project whose hierarchy references `.html`
/// documents — real prose the current engine cannot load.
fn legacy_html_fixture() -> (TempDir, PathBuf) {
    let (temp, root) = base_fixture();
    fs::write(
        root.join("project.yaml"),
        r#"id: "prj"
name: "Fixture"
created: "2025-01-01T00:00:00Z"
modified: "2025-01-01T00:00:00Z"
hierarchy:
  - type: Document
    id: "doc1"
    name: "Untitled"
    path: "manuscript/untitled.html"
"#,
    )
    .unwrap();
    fs::remove_file(root.join("manuscript/chapter-01.md")).unwrap();
    fs::remove_file(root.join("manuscript/chapter-01.meta")).unwrap();
    fs::write(
        root.join("manuscript/untitled.html"),
        "<p>one hundred eighteen lines of real work</p>\n",
    )
    .unwrap();
    (temp, root)
}

/// Fixture (b): hierarchy references a file that does not exist.
fn missing_document_fixture() -> (TempDir, PathBuf) {
    let (temp, root) = base_fixture();
    fs::remove_file(root.join("manuscript/chapter-01.md")).unwrap();
    fs::remove_file(root.join("manuscript/chapter-01.meta")).unwrap();
    (temp, root)
}

/// Fixture (c): corrupt document sidecar that cannot be fully interpreted.
fn corrupt_sidecar_fixture() -> (TempDir, PathBuf) {
    let (temp, root) = base_fixture();
    fs::write(root.join("manuscript/chapter-01.meta"), "id: [").unwrap();
    (temp, root)
}

/// Fixture (e): `format_version` newer than this engine writes.
fn newer_version_fixture() -> (TempDir, PathBuf) {
    let (temp, root) = base_fixture();
    let yaml = fs::read_to_string(root.join("project.yaml"))
        .unwrap()
        .replace("format_version: '1.2'", "format_version: '9.9'");
    fs::write(root.join("project.yaml"), yaml).unwrap();
    (temp, root)
}

/// The shared guard assertion: Degraded classification with the expected
/// reason, side-effect-free probe and Degraded open, refused mutations,
/// and byte-identity after every attempt.
fn assert_degraded_and_untouched(
    root: &Path,
    reason_matches: impl Fn(&DegradedReason) -> bool,
    reason_label: &str,
) {
    let before = tree_snapshot(root);

    // 1. Probe: Degraded with the right reason; no side effects.
    match probe_project_fidelity(root).expect("probe must succeed") {
        Fidelity::Degraded { reasons } => {
            assert!(
                reasons.iter().any(&reason_matches),
                "expected a {reason_label} reason, got {reasons:?}"
            );
        }
        Fidelity::Full => panic!("fixture must probe Degraded"),
    }
    assert_eq!(
        before,
        tree_snapshot(root),
        "probe must be byte-for-byte side-effect-free"
    );

    // 2. Public/default Degraded open: no side effects. The explicit alias
    //    remains equally pure for source compatibility.
    let public_project = read_project(root);
    assert_eq!(
        before,
        tree_snapshot(root),
        "public Degraded open must be byte-for-byte side-effect-free"
    );
    let _ = read_project_readonly(root);
    assert_eq!(before, tree_snapshot(root));

    // 3. Mutations refused. With a healthy guard, acquire_write_token
    //    refuses and no write can even be expressed. If the guard is
    //    disabled (guard-proof drill), the attempts below run for real and
    //    the final byte-identity assertion fails.
    match acquire_write_token(root) {
        Err(ChiknError::ReadOnly(_)) => {}
        Err(other) => panic!("expected ReadOnly refusal, got {other:?}"),
        Ok(token) => {
            // Drill mode only: bypassing both token acquisition and fresh
            // permit probing lets these real mutations execute.
            let _ = token.with_write_permit(root, |permit| {
                if let Ok(mut project) = public_project {
                    let _ = write_project(&mut project, permit);
                }
                let _ = delete_document(root, "manuscript/chapter-01.md", permit);
                let _ = writer::write_project_app_file(
                    permit,
                    Path::new("settings/writing-history.json"),
                    b"{\"entries\":[]}",
                );
                let _ = writer::ensure_project_subdir(permit, Path::new("characters"));
                let _ = git::save_revision(root, "Auto-save on close", permit);
                Ok(())
            });
        }
    }
    assert_eq!(
        before,
        tree_snapshot(root),
        "Degraded project must stay byte-identical after mutation attempts"
    );
}

#[test]
fn legacy_html_project_is_degraded_and_untouched() {
    let (_t, root) = legacy_html_fixture();
    assert_degraded_and_untouched(
        &root,
        |r| matches!(r, DegradedReason::LegacyDocumentPath { path } if path == "manuscript/untitled.html"),
        "LegacyDocumentPath",
    );
}

#[test]
fn missing_document_project_is_degraded_and_untouched() {
    let (_t, root) = missing_document_fixture();
    assert_degraded_and_untouched(
        &root,
        |r| matches!(r, DegradedReason::UnresolvedDocument { path, .. } if path == "manuscript/chapter-01.md"),
        "UnresolvedDocument",
    );
}

#[test]
fn corrupt_sidecar_project_is_degraded_and_untouched() {
    let (_t, root) = corrupt_sidecar_fixture();
    assert_degraded_and_untouched(
        &root,
        |r| matches!(r, DegradedReason::CorruptSidecar { path } if path == "manuscript/chapter-01.meta"),
        "CorruptSidecar",
    );
}

#[test]
fn public_read_of_corrupt_sidecar_and_missing_folders_is_browsable_and_pure() {
    let (_temp, root) = corrupt_sidecar_fixture();
    fs::remove_dir(root.join("research")).unwrap();
    fs::remove_dir(root.join("templates")).unwrap();
    fs::remove_dir(root.join("settings")).unwrap();
    let before = tree_snapshot(&root);

    let project = read_project(&root).expect("pure read should keep the project browsable");

    let document = project.documents.get("doc1").expect("hierarchy identity");
    assert_eq!(document.path, "manuscript/chapter-01.md");
    assert_eq!(document.content, "# Chapter 1\n");
    assert_eq!(before, tree_snapshot(&root));
    assert_eq!(
        fs::read(root.join("manuscript/chapter-01.meta")).unwrap(),
        b"id: ["
    );
}

#[test]
fn newer_format_version_project_is_degraded_and_untouched() {
    let (_t, root) = newer_version_fixture();
    assert_degraded_and_untouched(
        &root,
        |r| matches!(r, DegradedReason::NewerFormatVersion { found } if found == "9.9"),
        "NewerFormatVersion",
    );
}

#[test]
fn fresh_fidelity_old_session_cannot_begin_operation() {
    let (_temp, root) = base_fixture();
    let mut project = read_project(&root).unwrap();
    let token = acquire_write_token(&root).unwrap();

    let yaml = fs::read_to_string(root.join("project.yaml"))
        .unwrap()
        .replace("format_version: '1.2'", "format_version: '9.9'");
    fs::write(root.join("project.yaml"), yaml).unwrap();
    let before = tree_snapshot(&root);
    assert!(
        !token.is_stale(),
        "external edits do not bump the in-process epoch"
    );

    let write = token.with_write_permit(&root, |permit| write_project(&mut project, permit));
    assert!(
        matches!(write, Err(ChiknError::ReadOnly(_))),
        "project write must refuse the externally degraded tree: {write:?}"
    );

    let delete = token.with_write_permit(&root, |permit| {
        delete_document(&root, "manuscript/chapter-01.md", permit)
    });
    assert!(
        matches!(delete, Err(ChiknError::ReadOnly(_))),
        "document delete must refuse the externally degraded tree: {delete:?}"
    );

    let app_file = token.with_write_permit(&root, |permit| {
        writer::write_project_app_file(
            permit,
            Path::new("settings/writing-history.json"),
            b"{\"entries\":[]}",
        )
    });
    assert!(
        matches!(app_file, Err(ChiknError::ReadOnly(_))),
        "app-file write must refuse the externally degraded tree: {app_file:?}"
    );

    let subdir = token.with_write_permit(&root, |permit| {
        writer::ensure_project_subdir(permit, Path::new("characters")).map(|_| ())
    });
    assert!(
        matches!(subdir, Err(ChiknError::ReadOnly(_))),
        "subdirectory creation must refuse the externally degraded tree: {subdir:?}"
    );

    let revision = token.with_write_permit(&root, |permit| {
        git::save_revision(&root, "must not enter", permit).map(|_| ())
    });
    assert!(
        matches!(revision, Err(ChiknError::ReadOnly(_))),
        "revision save must refuse the externally degraded tree: {revision:?}"
    );

    assert_eq!(before, tree_snapshot(&root));
}

#[test]
fn fresh_fidelity_old_session_rejects_newly_corrupt_sidecar() {
    let (_temp, root) = base_fixture();
    let token = acquire_write_token(&root).unwrap();
    let sidecar = root.join("manuscript/chapter-01.meta");

    fs::write(&sidecar, "id: [").unwrap();
    let before = tree_snapshot(&root);
    let entered = std::cell::Cell::new(false);

    assert!(
        !token.is_stale(),
        "external sidecar damage does not bump the in-process epoch"
    );
    let result = token.with_write_permit(&root, |_| {
        entered.set(true);
        Ok(())
    });

    assert!(matches!(result, Err(ChiknError::ReadOnly(_))));
    assert!(
        !entered.get(),
        "a corrupt sidecar must prevent permit issuance"
    );
    assert_eq!(before, tree_snapshot(&root));
    assert_eq!(fs::read(sidecar).unwrap(), b"id: [");
}

#[test]
fn token_for_one_project_is_refused_against_another() {
    let temp = TempDir::new().unwrap();
    let root_a = temp.path().join("A.chikn");
    let root_b = temp.path().join("B.chikn");
    let mut project_a = create_project(&root_a, "A").unwrap();
    let mut project_b = create_project(&root_b, "B").unwrap();
    let token_a = acquire_write_token(&root_a).unwrap();
    let token_b = acquire_write_token(&root_b).unwrap();

    // Sanity: each token works for its own project.
    token_a
        .with_write_permit(&root_a, |permit| write_project(&mut project_a, permit))
        .unwrap();
    token_b
        .with_write_permit(&root_b, |permit| write_project(&mut project_b, permit))
        .unwrap();

    let before_b = tree_snapshot(&root_b);

    // Cross-project use is refused across the mutating surface.
    let write = token_a.with_write_permit(&root_b, |permit| write_project(&mut project_b, permit));
    assert!(
        matches!(write, Err(ChiknError::ReadOnly(_))),
        "token A must not authorize write_project into B: {write:?}"
    );
    let delete = token_a.with_write_permit(&root_b, |permit| {
        delete_document(&root_b, "manuscript/anything.md", permit)
    });
    assert!(
        matches!(delete, Err(ChiknError::ReadOnly(_))),
        "token A must not authorize delete_document in B: {delete:?}"
    );
    let commit = token_a.with_write_permit(&root_b, |permit| {
        git::save_revision(&root_b, "hijack", permit)
    });
    assert!(
        matches!(commit, Err(ChiknError::ReadOnly(_))),
        "token A must not authorize save_revision in B: {commit:?}"
    );

    assert_eq!(
        before_b,
        tree_snapshot(&root_b),
        "project B must stay byte-identical after cross-project attempts"
    );
}

#[test]
fn token_goes_stale_after_tree_replacing_operation() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("Stale.chikn");
    let mut project = create_project(&root, "Stale").unwrap();
    let token = acquire_write_token(&root).unwrap();
    let baseline = token
        .with_write_permit(&root, |permit| {
            write_project(&mut project, permit)?;
            git::save_revision(&root, "Baseline", permit)
        })
        .unwrap();

    // A tree-replacing operation (revision restore) bumps the epoch...
    token
        .with_write_permit(&root, |permit| {
            git::restore_revision(&root, &baseline.id, permit)
        })
        .unwrap();

    // ...so the pre-bump token is refused everywhere.
    assert!(token.is_stale());
    let write = token.with_write_permit(&root, |permit| write_project(&mut project, permit));
    assert!(
        matches!(write, Err(ChiknError::ReadOnly(_))),
        "stale token must be refused by write_project: {write:?}"
    );
    let commit = token.with_write_permit(&root, |permit| {
        git::save_revision(&root, "after restore", permit)
    });
    assert!(
        matches!(commit, Err(ChiknError::ReadOnly(_))),
        "stale token must be refused by save_revision: {commit:?}"
    );

    // Re-probing issues a fresh, working token.
    let fresh = acquire_write_token(&root).unwrap();
    fresh
        .with_write_permit(&root, |permit| write_project(&mut project, permit))
        .unwrap();
}

#[test]
fn zero_byte_document_probes_full_and_roundtrips_untouched() {
    let (_t, root) = base_fixture();
    fs::write(root.join("manuscript/empty.md"), "").unwrap();
    fs::write(
        root.join("manuscript/empty.meta"),
        "id: \"doc-empty\"\ncreated: \"2025-01-01T00:00:00Z\"\nmodified: \"2025-01-01T00:00:00Z\"\n",
    )
    .unwrap();
    let yaml = fs::read_to_string(root.join("project.yaml")).unwrap();
    fs::write(
        root.join("project.yaml"),
        format!(
            "{yaml}  - type: Document\n    id: \"doc-empty\"\n    name: \"Empty\"\n    path: \"manuscript/empty.md\"\n"
        ),
    )
    .unwrap();

    assert_eq!(
        probe_project_fidelity(&root).unwrap(),
        Fidelity::Full,
        "zero-byte documents are valid, not Degraded"
    );

    let mut project = read_project(&root).unwrap();
    assert_eq!(project.documents.get("doc-empty").unwrap().content, "");
    let token = acquire_write_token(&root).unwrap();
    token
        .with_write_permit(&root, |permit| write_project(&mut project, permit))
        .unwrap();

    let reread = read_project(&root).unwrap();
    assert_eq!(
        reread.documents.get("doc-empty").unwrap().content,
        "",
        "zero-byte document must survive a save round-trip empty"
    );
    assert_eq!(fs::read(root.join("manuscript/empty.md")).unwrap(), b"");
    assert_eq!(probe_project_fidelity(&root).unwrap(), Fidelity::Full);
}

fn copy_dir(src: &Path, dest: &Path) {
    fs::create_dir_all(dest).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let target = dest.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// samples/Corn.chikn is current-converter output and must probe Full,
/// open, and write normally. Missing standard folders stay covered here by
/// deleting them from a scratch copy first: they are benign self-heal, not
/// Degraded.
#[test]
fn corn_sample_probes_full_opens_and_writes_normally() {
    let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/Corn.chikn");
    assert!(sample.is_dir(), "samples/Corn.chikn missing at {sample:?}");

    // Probe the checked-in sample in place — the probe is side-effect-free.
    let before = tree_snapshot(&sample);
    assert_eq!(
        probe_project_fidelity(&sample).unwrap(),
        Fidelity::Full,
        "Corn.chikn must probe Full"
    );
    assert_eq!(
        before,
        tree_snapshot(&sample),
        "probing the sample must not modify the repository"
    );

    // Open + self-heal + write on a scratch copy with the content-free
    // standard folders removed (a project missing them must still probe
    // Full and self-heal on a normal open). research/ stays: the sample's
    // binder references an asset inside it, and a missing referenced asset
    // is rightly Degraded.
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("Corn.chikn");
    copy_dir(&sample, &root);
    for folder in ["templates", "settings"] {
        let dir = root.join(folder);
        if dir.is_dir() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
    }
    assert_eq!(
        probe_project_fidelity(&root).unwrap(),
        Fidelity::Full,
        "missing standard folders must not degrade a project"
    );

    let token = acquire_write_token(&root).unwrap();
    let mut project = token
        .with_write_permit(&root, |permit| read_project_with_repair(&root, permit))
        .unwrap();
    assert!(
        root.join("templates").is_dir(),
        "normal open must self-heal missing standard folders"
    );
    assert!(root.join("settings").is_dir());

    token
        .with_write_permit(&root, |permit| write_project(&mut project, permit))
        .unwrap();
    let reread = read_project(&root).unwrap();
    assert_eq!(reread.documents.len(), project.documents.len());
}

/// A binder-referenced binary asset (imported research PDF etc.) is
/// fidelity-neutral while it exists — and Degraded the moment it is missing.
#[test]
fn binder_asset_is_fidelity_neutral_until_missing() {
    let (_temp, root) = base_fixture();
    fs::write(
        root.join("project.yaml"),
        r#"format_version: '1.2'
id: "prj"
name: "Fixture"
created: "2025-01-01T00:00:00Z"
modified: "2025-01-01T00:00:00Z"
hierarchy:
  - type: Document
    id: "doc1"
    name: "Chapter 1"
    path: "manuscript/chapter-01.md"
  - type: Document
    id: "asset1"
    name: "Sample MS"
    path: "research/sample.pdf"
"#,
    )
    .unwrap();
    fs::write(root.join("research/sample.pdf"), b"%PDF-1.4 fake").unwrap();

    assert_eq!(
        probe_project_fidelity(&root).unwrap(),
        Fidelity::Full,
        "an existing binder asset must not degrade the project"
    );

    fs::remove_file(root.join("research/sample.pdf")).unwrap();
    match probe_project_fidelity(&root).unwrap() {
        Fidelity::Degraded { reasons } => assert!(
            reasons.iter().any(|r| matches!(
                r,
                DegradedReason::UnresolvedDocument { path, .. } if path == "research/sample.pdf"
            )),
            "missing asset must degrade, got {reasons:?}"
        ),
        other => panic!("expected Degraded for a missing asset, got {other:?}"),
    }
}

/// The writer must never emit document text into a non-.md file, even on a
/// Full project with a valid token — asset content is opaque; only its
/// sidecar metadata is the writer's to maintain.
#[test]
fn writer_never_writes_content_into_asset_path() {
    let (_temp, root) = base_fixture();
    fs::write(root.join("research/sample.pdf"), b"%PDF-1.4 fake").unwrap();

    let mut project = read_project(&root).unwrap();
    let token = acquire_write_token(&root).unwrap();

    let mut rogue = project.documents.values().next().unwrap().clone();
    rogue.id = "asset1".to_string();
    rogue.path = "research/sample.pdf".to_string();
    rogue.content = "this text must never reach the pdf".to_string();
    project.documents.insert(rogue.id.clone(), rogue);

    token
        .with_write_permit(&root, |permit| write_project(&mut project, permit))
        .unwrap();
    assert_eq!(
        fs::read(root.join("research/sample.pdf")).unwrap(),
        b"%PDF-1.4 fake",
        "asset bytes must be untouched by document writes"
    );
    assert!(
        root.join("research/sample.meta").is_file(),
        "asset sidecar metadata is still maintained"
    );
}
