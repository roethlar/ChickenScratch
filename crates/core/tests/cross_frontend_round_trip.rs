//! Cross-frontend round-trip regression tests.
//!
//! These cover the format-level promise that any frontend that follows the
//! `.meta` shape can hand a project to any other frontend without losing
//! identity, comments, scrivener round-trip data, session targets, or threads.
//! The Windows C# writer is the most divergent in practice (closed POCOs +
//! historic wire-type drift on `include_in_compile`); the project.yaml /
//! `.meta` snippets here intentionally mirror what that writer produces so
//! changes to the Rust reader can't silently regress its behavior.

use chickenscratch_core::core::project::reader::read_project;
use std::fs;
use tempfile::TempDir;

#[test]
fn verify_cross_frontend_harness_project_from_env() {
    let Ok(path) = std::env::var("CHIKN_CROSS_FRONTEND_VERIFY") else {
        eprintln!("skipping: CHIKN_CROSS_FRONTEND_VERIFY is not set");
        return;
    };

    let project = read_project(std::path::Path::new(&path)).expect("Rust reader loads project");
    assert!(
        !project.documents.is_empty(),
        "cross-frontend harness project should contain documents"
    );

    if let Ok(marker) = std::env::var("CHIKN_CROSS_FRONTEND_EXPECT_FIELD") {
        let found = project
            .documents
            .values()
            .any(|doc| doc.fields.contains_key(&marker));
        assert!(
            found,
            "expected at least one document to contain fields.{marker}"
        );
    }

    println!(
        "rust-reader: loaded \"{}\" ({} docs, {} top-level nodes, {} threads)",
        project.name,
        project.documents.len(),
        project.hierarchy.len(),
        project.threads.len()
    );
}

/// Build a project on disk using the wire forms a Windows-style writer
/// produces:
/// * canonical "Yes"/"No" string for `include_in_compile`
/// * meta files carry `id`/`name`/`parent_id` so the cross-frontend reader
///   keys `project.documents` by the same id the hierarchy points at.
fn write_windows_style_project(root: &std::path::Path) {
    // Required by `validate_project_structure` before repair runs (see F-012).
    for sub in [
        "manuscript",
        "research",
        "templates",
        "settings",
        "characters",
    ] {
        fs::create_dir_all(root.join(sub)).unwrap();
    }

    fs::write(
        root.join("project.yaml"),
        r#"id: "proj-1"
name: "Cross-Frontend Test"
created: "2026-05-01T00:00:00Z"
modified: "2026-05-01T00:00:00Z"
metadata:
  title: "Cross-Frontend Test"
  author: "Tester"
  session_target:
    words_per_session: 500
    deadline: "2026-12-31"
hierarchy:
  - type: document
    id: "doc-chapter-01"
    name: "Chapter 1"
    path: "manuscript/chapter-01.md"
"#,
    )
    .unwrap();

    fs::write(
        root.join("manuscript/chapter-01.md"),
        "# Chapter 1\n\nOnce upon a time…",
    )
    .unwrap();

    // The shape the patched Windows writer emits: full identity, "Yes" string,
    // comments, scrivener round-trip ids, fields preserved.
    fs::write(
        root.join("manuscript/chapter-01.meta"),
        r#"id: "doc-chapter-01"
name: "Chapter 1"
created: "2026-05-01T00:00:00Z"
modified: "2026-05-01T00:00:00Z"
synopsis: "The opening scene"
include_in_compile: "Yes"
section_type: "scene"
scrivener_uuid: "abc-123"
word_count_target: 1500
compile_order: 1
comments:
  - id: "c1"
    body: "Reword this paragraph"
    resolved: false
    created: "2026-05-01T00:00:00Z"
    modified: "2026-05-01T00:00:00Z"
fields:
  pov_character: "sarah"
  threads:
    - thread-rebellion
"#,
    )
    .unwrap();

    // Entity document under characters/ — never appears in hierarchy. The
    // disk-walking reader (Rust + Swift + the patched Windows reader) must
    // pick this up; the old hierarchy-only walk dropped it on the floor.
    fs::write(
        root.join("characters/sarah.md"),
        "Sarah Bennett — protagonist.",
    )
    .unwrap();
    fs::write(
        root.join("characters/sarah.meta"),
        r#"id: "char-sarah"
name: "Sarah Bennett"
created: "2026-05-01T00:00:00Z"
modified: "2026-05-01T00:00:00Z"
fields:
  entity_kind: "character"
"#,
    )
    .unwrap();

    fs::write(
        root.join("threads.yaml"),
        // Raw string uses ## delimiters so the embedded `"#ff0000"` doesn't
        // terminate the literal early.
        r##"threads:
  - id: thread-rebellion
    name: The Rebellion
    color: "#ff0000"
"##,
    )
    .unwrap();
}

#[test]
fn windows_style_project_round_trips_identity_and_format_data() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("Test.chikn");
    fs::create_dir(&root).unwrap();
    write_windows_style_project(&root);

    let project = read_project(&root).expect("read");

    // F-001: hierarchy node id matches the documents-map key. If meta.id were
    // missing, the reader would synthesize a fresh UUID and the hierarchy
    // would point at a id that doesn't exist in `documents`.
    let chapter = project
        .documents
        .get("doc-chapter-01")
        .expect("chapter doc keyed by hierarchy id");
    assert_eq!(chapter.name, "Chapter 1");
    assert_eq!(chapter.synopsis.as_deref(), Some("The opening scene"));

    // F-002: "Yes" round-trips to the bool API.
    assert!(chapter.include_in_compile);

    // F-003: scrivener ids, comments, and fields all preserved.
    assert_eq!(chapter.comments.len(), 1);
    assert_eq!(chapter.comments[0].body, "Reword this paragraph");
    assert!(chapter.fields.contains_key("pov_character"));

    // F-004: entity under characters/ found via disk walk, even though it's
    // intentionally not in `project.yaml.hierarchy`.
    assert!(
        project.documents.contains_key("char-sarah"),
        "entity document under characters/ should be loaded even when hierarchy doesn't list it"
    );

    // Threads + session target carried through.
    assert_eq!(project.threads.len(), 1);
    assert_eq!(project.threads[0].name, "The Rebellion");
    assert!(project.metadata.session_target.is_some());
    assert_eq!(
        project
            .metadata
            .session_target
            .as_ref()
            .unwrap()
            .words_per_session,
        Some(500)
    );
}

#[test]
fn legacy_bool_include_in_compile_still_loads() {
    // Older Windows builds (before F-002) wrote `include_in_compile: false`
    // as a YAML boolean. The reader has to accept that for back-compat or
    // those projects fail to open.
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("Legacy.chikn");
    for sub in ["manuscript", "research", "templates", "settings"] {
        fs::create_dir_all(root.join(sub)).unwrap();
    }

    fs::write(
        root.join("project.yaml"),
        r#"id: "p"
name: "Legacy"
created: "2026-05-01T00:00:00Z"
modified: "2026-05-01T00:00:00Z"
hierarchy:
  - type: document
    id: "doc1"
    name: "Excluded"
    path: "manuscript/excluded.md"
"#,
    )
    .unwrap();
    fs::write(root.join("manuscript/excluded.md"), "skip me").unwrap();
    fs::write(
        root.join("manuscript/excluded.meta"),
        r#"id: "doc1"
name: "Excluded"
created: "2026-05-01T00:00:00Z"
modified: "2026-05-01T00:00:00Z"
include_in_compile: false
"#,
    )
    .unwrap();

    let project = read_project(&root).expect("legacy project loads");
    let doc = project.documents.get("doc1").expect("doc keyed by meta.id");
    assert!(
        !doc.include_in_compile,
        "YAML bool `false` should map to include_in_compile == false"
    );
}

#[test]
fn project_self_heals_when_required_folder_missing() {
    // F-012: roadmap claims self-healing, but the previous read pipeline
    // ran strict folder validation BEFORE repair, so a missing
    // `templates/` (or any other required folder) blocked load entirely.
    // The fix runs repair first, then validates only the truly fatal
    // conditions (path missing / not a directory / no project.yaml).
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("Healing.chikn");
    fs::create_dir(&root).unwrap();

    // Only create manuscript/ and settings/ — leave research/ and templates/
    // missing. This used to error with "Missing required folder: templates".
    fs::create_dir(root.join("manuscript")).unwrap();
    fs::create_dir(root.join("settings")).unwrap();

    fs::write(
        root.join("project.yaml"),
        r#"id: "p"
name: "Healing"
created: "2026-05-01T00:00:00Z"
modified: "2026-05-01T00:00:00Z"
hierarchy: []
"#,
    )
    .unwrap();

    let project = read_project(&root).expect("project should self-heal");
    assert_eq!(project.name, "Healing");
    assert!(
        root.join("research").is_dir(),
        "research/ should be created"
    );
    assert!(
        root.join("templates").is_dir(),
        "templates/ should be created"
    );
}
