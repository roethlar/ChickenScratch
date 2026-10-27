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

/// A footnote link that no document resolves to a document (an attachment)
/// is not a document link and stays exactly as written, even while another
/// link fires the resolution (review finding s3-4).
#[test]
fn ordinary_links_inside_footnotes_are_left_alone() {
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
                "Body[^n] and [two](b.md#S1).\n\n[^n]: See the [appendix](research/notes.pdf) and [site](https://example.com).",
            ),
            doc("manuscript/b.md", "# Two {#S1}"),
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(
        href_of(&html, "appendix").as_deref(),
        Some("research/notes.pdf"),
        "an attachment link inside a note is untouched"
    );
    assert_eq!(
        href_of(&html, "site").as_deref(),
        Some("https://example.com")
    );
    assert!(href_of(&html, "two").is_some_and(|h| h.starts_with('#')));
}

/// A rootless target that names one document relative to its own and
/// another from the project root resolves relatively, as the plan says, and
/// the writer is warned about the competing reading (review finding s3-10).
#[test]
fn a_relative_root_ambiguity_resolves_relatively_and_warns() {
    if !pandoc_present() {
        eprintln!("skipping: pandoc not installed");
        return;
    }
    let dir = TempDir::new().unwrap();
    let p = project(
        dir.path(),
        &[
            doc("manuscript/a.md", "See [b](manuscript/b.md)."),
            doc("manuscript/b.md", "Root b."),
            doc("manuscript/manuscript/b.md", "Nested b."),
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    let href = href_of(&html, "b").unwrap();
    let target = href.trim_start_matches('#');
    // The relative reading wins: the destination sits in the nested b.
    let dest = html.find(&format!("id=\"{target}\"")).unwrap();
    let nested = html.find("Nested b.").unwrap();
    let root = html.find("Root b.").unwrap();
    assert!(
        dest > root && dest <= nested + 20,
        "destination is the nested document: {html}"
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("manuscript/b.md") && w.contains("project root")),
        "{warnings:?}"
    );
}

/// A link that becomes text keeps its own identifier: another link that
/// pointed at it still has a destination (review finding s3-7).
#[test]
fn a_link_turned_into_text_keeps_the_identifier_other_links_use() {
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
                "See [target](/manuscript/c.md){#anchor} and jump [back](#anchor).",
            ),
            Doc {
                path: "manuscript/c.md",
                content: "Excluded.",
                include: false,
                source_path: None,
            },
        ],
    );
    let (html, _warnings) = compile_html(&p, dir.path());
    assert!(
        href_of(&html, "target").is_none(),
        "the excluded link is text"
    );
    let back = href_of(&html, "back").unwrap();
    assert!(
        ids(&html).contains(back.trim_start_matches('#')),
        "the bare link's destination exists: {back}"
    );
    assert!(element_text(&html, back.trim_start_matches('#'))
        .unwrap()
        .contains("target"));
}

/// Identifiers and links in metadata (a `%` title block) take part: a title
/// span and a body heading sharing `same` with a bare link keep both
/// strings, and a document link found only in the title fires the gate
/// (review finding s3-8).
#[test]
fn metadata_identifiers_and_links_take_part() {
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
                "% [Title]{#same}\n\n# Head {#same}\n\nBare [b](#same) and [two](b.md#S1).",
            ),
            doc("manuscript/b.md", "# Two {#S1}"),
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(
        html.matches("id=\"same\"").count(),
        2,
        "both `same` kept: {html}"
    );
    assert_eq!(href_of(&html, "b").as_deref(), Some("#same"));

    let p = project(
        dir.path(),
        &[
            doc("manuscript/a.md", "% See [two](/manuscript/b.md)\n\nBody."),
            doc("manuscript/b.md", "Second."),
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    assert!(warnings.is_empty(), "{warnings:?}");
    let two = href_of(&html, "two").unwrap();
    assert!(ids(&html).contains(two.trim_start_matches('#')), "{two}");
}

/// A percent-encoded bare link relies on duplicated identifiers just as a
/// plain one does: both duplicates keep their string and the link is left
/// as written (review finding s3-1, repair review).
#[test]
fn encoded_bare_links_keep_duplicated_identifiers() {
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
                "# Start {#café}\n\nBare [b](#caf%C3%A9) and a document link [two](b.md#S1).",
            ),
            doc(
                "manuscript/b.md",
                "# Start {#café}\n\n## Inner {#S1}\n\nText.",
            ),
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(
        html.matches("id=\"café\"").count(),
        2,
        "both duplicates keep `café`"
    );
    assert_eq!(
        href_of(&html, "b").as_deref(),
        Some("#caf%C3%A9"),
        "the bare link is left as written"
    );
}

/// A writer's own metadata key named `t`, at the root or inside a nested
/// map, must not stop the metadata walk: identifiers and links elsewhere in
/// the metadata still take part (review finding s3-8, repair review).
#[test]
fn metadata_keys_named_t_do_not_hide_the_rest() {
    if !pandoc_present() {
        eprintln!("skipping: pandoc not installed");
        return;
    }
    let dir = TempDir::new().unwrap();
    // Root-level `t` beside a title carrying a duplicated identifier.
    let p = project(
        dir.path(),
        &[
            doc(
                "manuscript/a.md",
                "---\ntitle: \"[Title]{#same}\"\nt: auxiliary value\n---\n\n# Head {#same}\n\nBare [b](#same) and [two](b.md#S1).",
            ),
            doc("manuscript/b.md", "# Two {#S1}"),
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(
        html.matches("id=\"same\"").count(),
        2,
        "both `same` kept despite a `t` key: {html}"
    );
    assert_eq!(href_of(&html, "b").as_deref(), Some("#same"));

    // The only document link lives in the title, beside a `t` key.
    let p = project(
        dir.path(),
        &[
            doc(
                "manuscript/a.md",
                "---\ntitle: \"See [two](/manuscript/b.md)\"\nt: auxiliary value\n---\n\nBody.",
            ),
            doc("manuscript/b.md", "Second."),
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    assert!(warnings.is_empty(), "{warnings:?}");
    let two = href_of(&html, "two").unwrap();
    assert!(ids(&html).contains(two.trim_start_matches('#')), "{two}");

    // A `t` key inside a nested map, next to an identifier the body shares.
    let p = project(
        dir.path(),
        &[
            doc(
                "manuscript/a.md",
                "---\nextra:\n  t: nested\n  more: \"[Aside]{#same}\"\n---\n\n# Head {#same}\n\nBare [b](#same) and [two](b.md#S1).",
            ),
            doc("manuscript/b.md", "# Two {#S1}"),
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(
        href_of(&html, "b").as_deref(),
        Some("#same"),
        "the heading kept `same` because the nested metadata duplicate counted: {html}"
    );
}

/// A writer's quoted identifier may itself hold a literal `%`
/// (`{id="caf%C3%A9"}`): a bare link written the same way still protects
/// such duplicates and still follows a unique one, beside the decoded case
/// (review finding s3-1, repair review 2).
#[test]
fn literal_percent_identifiers_are_kept_and_followed_too() {
    if !pandoc_present() {
        eprintln!("skipping: pandoc not installed");
        return;
    }
    let dir = TempDir::new().unwrap();
    // Duplicated literal identifiers relied on by a bare link: both kept.
    let p = project(
        dir.path(),
        &[
            doc(
                "manuscript/a.md",
                "# One {id=\"caf%C3%A9\"}\n\nBare [b](#caf%C3%A9) and [two](b.md#S1).",
            ),
            doc(
                "manuscript/b.md",
                "# Two {id=\"caf%C3%A9\"}\n\n## Inner {#S1}\n\nText.",
            ),
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(
        html.matches("id=\"caf%C3%A9\"").count(),
        2,
        "both literal duplicates keep their string: {html}"
    );
    assert_eq!(href_of(&html, "b").as_deref(), Some("#caf%C3%A9"));

    // A unique literal identifier: the bare link follows its new name.
    let p = project(
        dir.path(),
        &[
            doc(
                "manuscript/a.md",
                "# One {id=\"caf%C3%A9\"}\n\nBare [b](#caf%C3%A9) and [two](b.md#S1).",
            ),
            doc("manuscript/b.md", "## Inner {#S1}\n\nText."),
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    assert!(warnings.is_empty(), "{warnings:?}");
    let b = href_of(&html, "b").unwrap();
    assert!(
        ids(&html).contains(b.trim_start_matches('#')),
        "{b}: {html}"
    );
    assert!(element_text(&html, b.trim_start_matches('#'))
        .unwrap()
        .contains("One"));
}

/// One element carries `{#café}` and another `{id="caf%C3%A9"}`: a bare
/// `#caf%C3%A9` link names both spellings at once, so it is left as written
/// and both identifiers keep their strings (review finding s3-1, repair
/// review 3).
#[test]
fn mixed_spellings_of_one_bare_fragment_keep_every_candidate() {
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
                "# One {#café}\n\n# Two {id=\"caf%C3%A9\"}\n\nBare [b](#caf%C3%A9) and [two](b.md#S1).",
            ),
            doc("manuscript/b.md", "## Inner {#S1}\n\nText."),
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    assert!(warnings.is_empty(), "{warnings:?}");
    let ids = ids(&html);
    assert!(ids.contains("café"), "decoded spelling kept: {html}");
    assert!(ids.contains("caf%C3%A9"), "literal spelling kept: {html}");
    assert_eq!(href_of(&html, "b").as_deref(), Some("#caf%C3%A9"));
}

/// Beside an ambiguous bare link that keeps both spellings, a second bare
/// link that reaches the kept literal identifier uniquely (`#caf%25C3%25A9`
/// decodes to `caf%C3%A9`) is rewritten to that identifier instead of being
/// left pointing at a spelling no element carries (review finding s3-1,
/// repair review 4).
#[test]
fn a_unique_bare_link_to_a_kept_identifier_is_rewritten_to_it() {
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
                "# One {#café}\n\n# Two {id=\"caf%C3%A9\"}\n\nBoth [b](#caf%C3%A9), literal [c](#caf%25C3%25A9) and [two](b.md#S1).",
            ),
            doc("manuscript/b.md", "## Inner {#S1}\n\nText."),
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    assert!(warnings.is_empty(), "{warnings:?}");
    let ids = ids(&html);
    assert!(ids.contains("café") && ids.contains("caf%C3%A9"), "{html}");
    assert_eq!(href_of(&html, "b").as_deref(), Some("#caf%C3%A9"));
    let c = href_of(&html, "c").unwrap();
    assert!(
        follows_heading(&html, c.trim_start_matches('#'), "Two"),
        "the unique link lands on the kept literal identifier's heading: {c} in {html}"
    );
}

/// Whether `id` is carried by the `<h1>` whose text contains `text`, or sits
/// right after it with no other heading in between (a kept heading's alias).
fn follows_heading(html: &str, id: &str, text: &str) -> bool {
    let Some(pos) = html.find(&format!("id=\"{id}\"")) else {
        return false;
    };
    let Some(open) = html[..pos].rfind("<h1") else {
        return false;
    };
    let Some(close) = html[open..].find("</h1>") else {
        return false;
    };
    html[open..open + close].contains(text)
}

/// With `café` and `caf%C3%A9` both kept, a browser may match either
/// spelling of `#caf%C3%A9` before decoding, so a bare link that resolves
/// uniquely to one of them must not point at a kept string: it gets that
/// element's alias, like a document link (review finding s3-1, repair
/// review 5).
#[test]
fn unique_links_to_kept_identifiers_get_an_unambiguous_alias() {
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
                "# One {#café}\n\n# Two {id=\"caf%C3%A9\"}\n\nBoth [b](#caf%C3%A9), literal [c](#caf%25C3%25A9), lower [d](#caf%c3%a9) and [two](b.md#S1).",
            ),
            doc("manuscript/b.md", "## Inner {#S1}\n\nText."),
        ],
    );
    let (html, warnings) = compile_html(&p, dir.path());
    assert!(warnings.is_empty(), "{warnings:?}");
    let ids = ids(&html);
    assert!(ids.contains("café") && ids.contains("caf%C3%A9"), "{html}");
    assert_eq!(href_of(&html, "b").as_deref(), Some("#caf%C3%A9"));
    for (link, heading) in [("c", "Two"), ("d", "One")] {
        let href = href_of(&html, link).unwrap();
        let id = href.trim_start_matches('#');
        assert!(ids.contains(id), "{link} → {href}: {html}");
        assert!(
            id != "café" && id != "caf%C3%A9",
            "{link} must not point at a spelling a browser could confuse: {href}"
        );
        assert!(
            follows_heading(&html, id, heading),
            "{link} → {href} should reach {heading}: {html}"
        );
    }
}
