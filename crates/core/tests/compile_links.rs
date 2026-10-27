//! Links between documents in compiled output (PLAN_SCRIVENER_UPLOAD slice
//! 3). These run the real Pandoc and read the HTML it writes: a destination
//! exists when an element carries the identifier a link points at. Skipped
//! with a printed line where Pandoc is absent (CI).

use chickenscratch_core::core::compile::{compile_with_report, CompileOptions};
use chickenscratch_core::core::project::fidelity::acquire_write_token;
use chickenscratch_core::core::project::writer::{create_project, write_project};
use chickenscratch_core::models::{Document, TreeNode};
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

struct Doc<'a> {
    path: &'a str,
    content: &'a str,
    include: bool,
    source_path: Option<&'a str>,
}

fn doc<'a>(path: &'a str, content: &'a str) -> Doc<'a> {
    Doc {
        path,
        content,
        include: true,
        source_path: None,
    }
}

fn pandoc_present() -> bool {
    std::process::Command::new("pandoc")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// A project with the given documents in binder order under Manuscript
/// (paths may be nested); every document is included unless said otherwise.
fn project(root: &Path, docs: &[Doc]) -> PathBuf {
    let project_path = root.join(format!("Links-{}.chikn", uuid::Uuid::new_v4()));
    let mut project = create_project(&project_path, "Links").unwrap();
    let token = acquire_write_token(&project_path).unwrap();
    let mut children = Vec::new();
    for (i, d) in docs.iter().enumerate() {
        let mut document = Document {
            id: format!("doc-{i}"),
            name: d
                .path
                .rsplit('/')
                .next()
                .unwrap()
                .trim_end_matches(".md")
                .to_string(),
            path: d.path.to_string(),
            content: d.content.to_string(),
            include_in_compile: d.include,
            created: "2026-01-01T00:00:00Z".to_string(),
            modified: "2026-01-01T00:00:00Z".to_string(),
            ..Default::default()
        };
        if let Some(sp) = d.source_path {
            document.fields.insert(
                "source_path".to_string(),
                serde_yaml::Value::String(sp.to_string()),
            );
        }
        children.push(TreeNode::Document {
            id: document.id.clone(),
            name: document.name.clone(),
            path: document.path.clone(),
        });
        project.documents.insert(document.id.clone(), document);
    }
    project.hierarchy.push(TreeNode::Folder {
        id: "manuscript-folder".to_string(),
        name: "Manuscript".to_string(),
        children,
    });
    token
        .with_write_permit(&project_path, |permit| write_project(&mut project, permit))
        .unwrap();
    project_path
}

/// Compile to HTML with a blank separator and no title page; returns the
/// HTML and the warnings.
fn compile_html(project_path: &Path, dir: &Path) -> (String, Vec<String>) {
    let out = dir.join(format!("{}.html", uuid::Uuid::new_v4()));
    let report = compile_with_report(
        project_path,
        &out,
        "html",
        None,
        None,
        Some(CompileOptions {
            section_separator: Some("* * *".to_string()),
            ..Default::default()
        }),
    )
    .unwrap();
    (fs::read_to_string(&out).unwrap(), report.warnings)
}

fn ids(html: &str) -> HashSet<String> {
    Regex::new(r#"\bid="([^"]+)""#)
        .unwrap()
        .captures_iter(html)
        .map(|c| c[1].to_string())
        .collect()
}

/// The fragment each `<a href="#…">` with the given text points at.
fn href_of(html: &str, text: &str) -> Option<String> {
    // Pandoc wraps long lines, so the tag may break after `<a`.
    let re = Regex::new(&format!(
        r#"<a\s+href="([^"]*)"[^>]*>{}</a>"#,
        regex::escape(text)
    ))
    .unwrap();
    re.captures(html).map(|c| c[1].to_string())
}

/// The text of the element carrying `id`.
fn element_text(html: &str, id: &str) -> Option<String> {
    let re = Regex::new(&format!(
        r#"<(h[1-6]|span|div|p)[^>]*\bid="{}"[^>]*>(.*?)</(h[1-6]|span|div|p)>"#,
        regex::escape(id)
    ))
    .unwrap();
    re.captures(html).map(|c| c[2].to_string())
}

#[test]
fn explicit_relative_root_anchored_and_legacy_links_resolve() {
    if !pandoc_present() {
        eprintln!("skipping: pandoc not installed");
        return;
    }
    let dir = TempDir::new().unwrap();
    let p = project(
        dir.path(),
        &[
            doc(
                "manuscript/one.md",
                "See [frag](two.md#S1), [whole](/manuscript/two.md), [legacy](manuscript/two.md) \
                 and [outside](https://example.com/two.md).",
            ),
            doc("manuscript/two.md", "# Two {#S1}\n\nText of two."),
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    assert!(warnings.is_empty(), "{warnings:?}");
    let ids = ids(&html);
    for text in ["frag", "whole", "legacy"] {
        let href = href_of(&html, text).unwrap_or_else(|| panic!("link {text} missing"));
        let target = href
            .strip_prefix('#')
            .unwrap_or_else(|| panic!("{text} still external: {href}"));
        assert!(ids.contains(target), "{text}: no element carries {target}");
        assert!(
            element_text(&html, target).unwrap().contains("Two"),
            "{text} lands on the heading of two"
        );
    }
    assert_eq!(
        href_of(&html, "outside").as_deref(),
        Some("https://example.com/two.md"),
        "an external link is untouched"
    );
    assert!(!html.contains("chikn:begin"), "markers are removed");
}

#[test]
fn a_generated_identifier_resolves_by_block_path_not_by_name() {
    if !pandoc_present() {
        eprintln!("skipping: pandoc not installed");
        return;
    }
    let dir = TempDir::new().unwrap();
    // A's own `# Introduction` shifts B's generated identifiers to
    // introduction-1 and introduction-2 in the combined parse; the link
    // still means B's second heading (plan-4 R8-2, R9-1).
    let p = project(
        dir.path(),
        &[
            doc(
                "manuscript/a.md",
                "# Introduction\n\nGo to [second](b.md#introduction-1) and [first](b.md#introduction).",
            ),
            doc(
                "manuscript/b.md",
                "# Introduction\n\nFirst of b.\n\n# Introduction\n\nSecond of b.",
            ),
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    assert!(warnings.is_empty(), "{warnings:?}");
    let second = href_of(&html, "second").unwrap();
    let first = href_of(&html, "first").unwrap();
    assert_ne!(second, first);
    // Both land on headings of b, in b's order: the text after each heading
    // tells which.
    let pos = |frag: &str| {
        html.find(&format!("id=\"{}\"", frag.trim_start_matches('#')))
            .unwrap()
    };
    assert!(pos(&first) < pos(&second));
    assert!(
        pos(&first) > html.find("Go to").unwrap(),
        "both lie in b, after a"
    );
}

#[test]
fn unresolvable_excluded_and_footnote_targets_become_text_with_warnings() {
    if !pandoc_present() {
        eprintln!("skipping: pandoc not installed");
        return;
    }
    let dir = TempDir::new().unwrap();
    let p = project(
        dir.path(),
        &[
            doc(
                "manuscript/a.md",
                "[missing](b.md#nowhere), [gone](c.md), [note](b.md#spot), [ok](b.md#S1).",
            ),
            doc(
                "manuscript/b.md",
                "# B {#S1}\n\nBody[^n].\n\n[^n]: Note start\n\n    # Inside {#spot}\n\n    Note end",
            ),
            Doc {
                path: "manuscript/c.md",
                content: "Excluded.",
                include: false,
                source_path: None,
            },
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    assert!(
        href_of(&html, "missing").is_none(),
        "unresolvable link is text"
    );
    assert!(
        href_of(&html, "gone").is_none(),
        "link to an excluded document is text"
    );
    assert!(
        href_of(&html, "note").is_none(),
        "link into a footnote is text"
    );
    assert!(html.contains("missing") && html.contains("gone") && html.contains("note"));
    assert!(href_of(&html, "ok").is_some_and(|h| h.starts_with('#')));
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("b.md#nowhere") && w.contains("not found")),
        "{warnings:?}"
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("points into a footnote")),
        "{warnings:?}"
    );
    assert_eq!(warnings.len(), 2, "{warnings:?}");
}

#[test]
fn duplicate_identifiers_relied_on_by_a_bare_link_keep_their_strings() {
    if !pandoc_present() {
        eprintln!("skipping: pandoc not installed");
        return;
    }
    let dir = TempDir::new().unwrap();
    let p = project(
        dir.path(),
        &[
            doc(
                "manuscript/a.md",
                "# Start {#same}\n\nBare [b](#same) and a document link [two](b.md#S1).",
            ),
            doc(
                "manuscript/b.md",
                "# Start {#same}\n\n## Inner {#S1}\n\nText.",
            ),
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(
        html.matches("id=\"same\"").count(),
        2,
        "both duplicates keep `same`"
    );
    assert_eq!(
        href_of(&html, "b").as_deref(),
        Some("#same"),
        "the bare link is left as written"
    );
    let two = href_of(&html, "two").unwrap();
    assert!(ids(&html).contains(two.trim_start_matches('#')));
}

#[test]
fn a_relative_link_inside_a_footnote_resolves_only_when_every_origin_agrees() {
    if !pandoc_present() {
        eprintln!("skipping: pandoc not installed");
        return;
    }
    let dir = TempDir::new().unwrap();
    // manuscript/two.md and manuscript/part/two.md: `two.md` from the note
    // means different documents from different folders.
    let p = project(
        dir.path(),
        &[
            doc("manuscript/a.md", "Body[^n].\n\n[^n]: See [t](two.md#S1)."),
            doc("manuscript/two.md", "# Two {#S1}"),
            doc("manuscript/part/two.md", "# Other two {#S1}"),
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    assert!(
        href_of(&html, "t").is_none(),
        "ambiguous footnote link is text"
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("could be read from more than one document")),
        "{warnings:?}"
    );

    // With one `two.md` beside every document, the origins agree.
    let p = project(
        dir.path(),
        &[
            doc("manuscript/a.md", "Body[^n].\n\n[^n]: See [t](two.md#S1)."),
            doc("manuscript/two.md", "# Two {#S1}"),
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    assert!(warnings.is_empty(), "{warnings:?}");
    let href = href_of(&html, "t").unwrap();
    assert!(ids(&html).contains(href.trim_start_matches('#')));
}

#[test]
fn whole_document_links_get_a_destination_whatever_the_document_opens_with() {
    if !pandoc_present() {
        eprintln!("skipping: pandoc not installed");
        return;
    }
    let dir = TempDir::new().unwrap();
    let p = project(
        dir.path(),
        &[
            doc(
                "manuscript/a.md",
                "[heading](h.md), [para](p.md), [list](l.md), [code](c.md), [quote](q.md), [defs](d.md).",
            ),
            doc("manuscript/h.md", "# Opens with a heading\n\nText."),
            doc("manuscript/p.md", "Opens with a paragraph."),
            doc("manuscript/l.md", "- one\n- two"),
            doc("manuscript/c.md", "```\ncode\n```"),
            doc("manuscript/q.md", "> quoted"),
            doc("manuscript/d.md", "[ref]: https://example.com/only-a-definition"),
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    assert!(warnings.is_empty(), "{warnings:?}");
    let ids = ids(&html);
    for text in ["heading", "para", "list", "code", "quote", "defs"] {
        let href = href_of(&html, text).unwrap_or_else(|| panic!("{text} missing"));
        let target = href
            .strip_prefix('#')
            .unwrap_or_else(|| panic!("{text}: {href}"));
        assert!(ids.contains(target), "{text}: destination {target} exists");
    }
    // The heading's destination is the heading itself, not an extra span.
    let h = href_of(&html, "heading").unwrap();
    assert!(element_text(&html, h.trim_start_matches('#'))
        .unwrap()
        .contains("Opens with a heading"));
}

#[test]
fn source_space_links_from_a_folder_import_resolve() {
    if !pandoc_present() {
        eprintln!("skipping: pandoc not installed");
        return;
    }
    let dir = TempDir::new().unwrap();
    let p = project(
        dir.path(),
        &[
            Doc {
                path: "manuscript/one.md",
                content: "See [two](two.txt#S1) and [intro](../intro.markdown).",
                include: true,
                source_path: Some("part/one.txt"),
            },
            Doc {
                path: "manuscript/two.md",
                content: "# Two {#S1}",
                include: true,
                source_path: Some("part/two.txt"),
            },
            Doc {
                path: "manuscript/intro.md",
                content: "Intro.",
                include: true,
                source_path: Some("intro.markdown"),
            },
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    assert!(warnings.is_empty(), "{warnings:?}");
    let ids = ids(&html);
    for text in ["two", "intro"] {
        let href = href_of(&html, text).unwrap();
        assert!(ids.contains(href.trim_start_matches('#')), "{text}: {href}");
    }
}

#[test]
fn a_swallowed_marker_warns_and_leaves_links_alone() {
    if !pandoc_present() {
        eprintln!("skipping: pandoc not installed");
        return;
    }
    let dir = TempDir::new().unwrap();
    // a opens a fence that b closes: a's end marker and b's begin marker
    // land inside the code block.
    let p = project(
        dir.path(),
        &[
            doc("manuscript/a.md", "See [two](b.md#S1).\n\n```\nunclosed"),
            doc("manuscript/b.md", "```\n\n# B {#S1}"),
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("runs into the next document")),
        "{warnings:?}"
    );
    assert_eq!(
        href_of(&html, "two").as_deref(),
        Some("b.md#S1"),
        "left as written"
    );
    assert!(!html.contains("chikn:"), "no marker text in today's output");
}

/// The editor percent-encodes a non-ASCII fragment on save; Pandoc's
/// identifier is the decoded form (review finding s3-1).
#[test]
fn percent_encoded_fragments_resolve_like_their_decoded_form() {
    if !pandoc_present() {
        eprintln!("skipping: pandoc not installed");
        return;
    }
    let dir = TempDir::new().unwrap();
    let p = project(
        dir.path(),
        &[
            doc(
                "manuscript/a.md",
                "Go to [cafe](b.md#caf%C3%A9) and the bare [local](#caf%C3%A9-here).\n\n# Café here",
            ),
            doc("manuscript/b.md", "# Café\n\nText."),
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    assert!(warnings.is_empty(), "{warnings:?}");
    let ids = ids(&html);
    let cafe = href_of(&html, "cafe").unwrap();
    assert!(ids.contains(cafe.trim_start_matches('#')), "{cafe}");
    assert!(element_text(&html, cafe.trim_start_matches('#'))
        .unwrap()
        .contains("Café"));
    // The bare link to a unique element follows that element's new name.
    let local = href_of(&html, "local").unwrap();
    assert!(ids.contains(local.trim_start_matches('#')), "{local}");
    assert!(element_text(&html, local.trim_start_matches('#'))
        .unwrap()
        .contains("Café here"));
}
