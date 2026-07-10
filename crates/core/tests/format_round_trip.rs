//! Full-fidelity round-trip guarantees for the .chikn format.
//!
//! Part of the format lock (docs/plans/PLAN_FORMAT_LOCK_ENGINE.md): the unit
//! suite asserts hand-picked fields, so a writer regression that silently
//! drops a field no assertion names would pass it. These tests assert that a
//! WHOLE document — every field at once — and whole-project state survive
//! write→read, and that consecutive saves produce one canonical byte form.

use chickenscratch_core::core::project::reader::read_project;
use chickenscratch_core::core::project::writer::{create_project, write_project};
use chickenscratch_core::models::{Comment, Document, Project, SessionTarget, Thread, TreeNode};
use std::path::Path;
use tempfile::TempDir;

fn manuscript_folder_id(project: &Project) -> String {
    match &project.hierarchy[0] {
        TreeNode::Folder { id, .. } => id.clone(),
        other => panic!("expected Manuscript folder first in hierarchy, got {other:?}"),
    }
}

/// A document with EVERY field populated with a non-default value, so a
/// writer or reader that drops any one of them breaks equality.
fn full_document(parent_id: &str) -> Document {
    let mut doc = Document {
        id: "doc-full".into(),
        name: "Chapter One".into(),
        path: "manuscript/chapter-one.md".into(),
        content: "# Chapter One\n\nSarah waited by the window.\n".into(),
        parent_id: Some(parent_id.into()),
        created: "2026-07-01T10:00:00+00:00".into(),
        modified: "2026-07-02T11:30:00+00:00".into(),
        synopsis: Some("Sarah discovers the letter.".into()),
        label: Some("Scene".into()),
        status: Some("First Draft".into()),
        keywords: Some(vec!["mystery".into(), "letter".into()]),
        links: Some(vec!["doc-other".into()]),
        include_in_compile: false,
        word_count_target: 1500,
        compile_order: 3,
        comments: vec![Comment {
            id: "c1".into(),
            body: "Tighten this paragraph.".into(),
            resolved: true,
            created: "2026-07-01T12:00:00+00:00".into(),
            modified: "2026-07-01T12:05:00+00:00".into(),
        }],
        ..Default::default()
    };
    doc.fields.insert(
        "pov_character".into(),
        serde_yaml::Value::String("sarah".into()),
    );
    doc.fields.insert(
        "duration_minutes".into(),
        serde_yaml::Value::Number(90.into()),
    );
    doc
}

fn attach(project: &mut Project, doc: Document) {
    if let TreeNode::Folder { children, .. } = &mut project.hierarchy[0] {
        children.push(TreeNode::Document {
            id: doc.id.clone(),
            name: doc.name.clone(),
            path: doc.path.clone(),
        });
    }
    project.documents.insert(doc.id.clone(), doc);
}

#[test]
fn whole_document_survives_round_trip() {
    let temp = TempDir::new().unwrap();
    let project_path = temp.path().join("Full.chikn");
    let mut project = create_project(&project_path, "Full").unwrap();
    let folder_id = manuscript_folder_id(&project);
    let doc = full_document(&folder_id);
    attach(&mut project, doc.clone());
    write_project(&mut project).unwrap();

    let reread = read_project(&project_path).unwrap();
    assert_eq!(
        reread.documents.get("doc-full"),
        Some(&doc),
        "every Document field must survive write→read unchanged"
    );
}

#[test]
fn whole_project_survives_round_trip() {
    let temp = TempDir::new().unwrap();
    let project_path = temp.path().join("Whole.chikn");
    let mut project = create_project(&project_path, "Whole").unwrap();

    project.metadata.title = Some("The Letter".into());
    project.metadata.author = Some("M. Coelho".into());
    project.metadata.project_type = Some("Novel".into());
    project.metadata.genre = Some("Mystery".into());
    project.metadata.theme = Some("Trust".into());
    project.metadata.summary = Some("A letter changes everything.".into());
    project.metadata.session_target = Some(SessionTarget {
        words_per_session: Some(500),
        deadline: Some("2026-12-31".into()),
        total_target: Some(80000),
    });
    project.threads = vec![
        Thread {
            id: "main-plot".into(),
            name: "Main Plot".into(),
            color: Some("#3b82f6".into()),
            description: Some("The letter's origin.".into()),
            extra: Default::default(),
        },
        Thread {
            id: "romance".into(),
            name: "Sarah & Marcus".into(),
            color: None,
            description: None,
            extra: Default::default(),
        },
    ];
    let folder_id = manuscript_folder_id(&project);
    attach(&mut project, full_document(&folder_id));
    write_project(&mut project).unwrap();

    let first = read_project(&project_path).unwrap();
    assert_eq!(
        first.metadata, project.metadata,
        "project metadata must survive"
    );
    assert_eq!(first.threads, project.threads, "threads must survive");
    assert_eq!(first.documents, project.documents, "documents must survive");
    assert_eq!(first.hierarchy, project.hierarchy, "hierarchy must survive");

    // And the whole state is stable across a second save+load.
    let mut first_again = first.clone();
    write_project(&mut first_again).unwrap();
    let second = read_project(&project_path).unwrap();
    assert_eq!(second.metadata, first.metadata);
    assert_eq!(second.threads, first.threads);
    assert_eq!(second.documents, first.documents);
    assert_eq!(second.hierarchy, first.hierarchy);
}

#[test]
fn second_save_is_byte_stable() {
    // Sidecars and threads.yaml must have ONE canonical byte form: saving
    // the same state twice produces identical bytes, so the embedded git
    // history records real edits only. project.yaml may differ solely in
    // its top-level modified: timestamp.
    let temp = TempDir::new().unwrap();
    let project_path = temp.path().join("Stable.chikn");
    let mut project = create_project(&project_path, "Stable").unwrap();
    let folder_id = manuscript_folder_id(&project);
    attach(&mut project, full_document(&folder_id));
    project.threads = vec![Thread {
        id: "main-plot".into(),
        name: "Main Plot".into(),
        color: Some("#3b82f6".into()),
        description: None,
        extra: Default::default(),
    }];
    write_project(&mut project).unwrap();

    let meta_path = project_path.join("manuscript/chapter-one.meta");
    let threads_path = project_path.join("threads.yaml");
    let project_file = project_path.join("project.yaml");
    let meta_1 = std::fs::read_to_string(&meta_path).unwrap();
    let threads_1 = std::fs::read_to_string(&threads_path).unwrap();
    let project_1 = std::fs::read_to_string(&project_file).unwrap();

    let mut reloaded = read_project(&project_path).unwrap();
    write_project(&mut reloaded).unwrap();

    let meta_2 = std::fs::read_to_string(&meta_path).unwrap();
    let threads_2 = std::fs::read_to_string(&threads_path).unwrap();
    let project_2 = std::fs::read_to_string(&project_file).unwrap();

    assert_eq!(meta_1, meta_2, ".meta must be byte-identical across saves");
    assert_eq!(
        threads_1, threads_2,
        "threads.yaml must be byte-identical across saves"
    );
    let strip_modified = |s: &str| {
        s.lines()
            .filter(|l| !l.starts_with("modified:"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(
        strip_modified(&project_1),
        strip_modified(&project_2),
        "project.yaml may differ only in its top-level modified: line"
    );
}

#[test]
fn foreign_and_legacy_keys_survive_full_cycle() {
    // End-to-end fixture: a project touched by "other tools" — unknown keys
    // at every preservation surface plus legacy top-level novelist keys —
    // goes through read→write→read without losing anything, with the legacy
    // keys relocated into fields.
    let temp = TempDir::new().unwrap();
    let project_path = temp.path().join("Foreign.chikn");
    let mut project = create_project(&project_path, "Foreign").unwrap();
    project.metadata.title = Some("Foreign".into());
    let folder_id = manuscript_folder_id(&project);
    attach(&mut project, full_document(&folder_id));
    project.threads = vec![Thread {
        id: "main-plot".into(),
        name: "Main Plot".into(),
        color: None,
        description: None,
        extra: Default::default(),
    }];
    write_project(&mut project).unwrap();

    // Simulate foreign/newer tools and the legacy 10ec683-era writer.
    let append = |path: &Path, s: &str| {
        let existing = std::fs::read_to_string(path).unwrap();
        std::fs::write(path, format!("{}\n{}\n", existing.trim_end(), s)).unwrap();
    };
    let meta_path = project_path.join("manuscript/chapter-one.meta");
    append(
        &meta_path,
        "future_tool_key: keep\nstory_time: Day 3, 22:30",
    );
    let project_file = project_path.join("project.yaml");
    append(&project_file, "future_project_key: keep");
    let threads_path = project_path.join("threads.yaml");
    append(&threads_path, "  arc_stage: rising-action");

    let mut reloaded = read_project(&project_path).unwrap();
    write_project(&mut reloaded).unwrap();
    let final_project = read_project(&project_path).unwrap();

    let doc = final_project.documents.get("doc-full").unwrap();
    assert_eq!(
        doc.fields.get("story_time").and_then(|v| v.as_str()),
        Some("Day 3, 22:30"),
        "legacy top-level novelist key must resurface in fields"
    );
    let meta_text = std::fs::read_to_string(&meta_path).unwrap();
    assert!(
        meta_text.contains("future_tool_key: keep"),
        "unknown .meta key must survive:\n{meta_text}"
    );
    assert!(
        !meta_text.lines().any(|l| l.starts_with("story_time:")),
        "legacy key must be relocated off the top level:\n{meta_text}"
    );
    assert!(
        std::fs::read_to_string(&project_file)
            .unwrap()
            .contains("future_project_key: keep"),
        "unknown project.yaml key must survive"
    );
    assert!(
        std::fs::read_to_string(&threads_path)
            .unwrap()
            .contains("arc_stage: rising-action"),
        "unknown thread-entry key must survive"
    );
    assert_eq!(
        final_project
            .threads
            .first()
            .and_then(|t| t.extra.get("arc_stage"))
            .and_then(|v| v.as_str()),
        Some("rising-action"),
        "thread extras must be visible on the model"
    );
}
