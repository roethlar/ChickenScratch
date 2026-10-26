//! # Scrivener to Chikn Converter
//!
//! Converts .scriv projects to .chikn format.
//!
//! ## Responsibilities
//! - Map Scrivener hierarchy to .chikn tree
//! - Convert RTF documents to Markdown
//! - Preserve metadata (labels, status, keywords)
//! - Generate .chikn project structure
//!
//! ## Conversion Process
//! 1. Parse .scrivx XML structure
//! 2. Read all RTF files from Files/Data/{UUID}/
//! 3. Convert RTF → Markdown
//! 4. Map Scrivener metadata to .chikn format
//! 5. Write .chikn project structure

use chrono::Utc;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

static SCRIV_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"scrivlnk://([A-Fa-f0-9-]+)").unwrap());
static SCR_TAG_HTML_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"&lt;[!]?\$Scr_[CP]s::\d+&gt;").unwrap());
static SCR_TAG_RAW_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<[!]?\$Scr_[CP]s::\d+>").unwrap());
static SCR_TAG_ESC_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\<[!]?\\?\$Scr_[CP]s::\d+\\>").unwrap());
static SCR_VAR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"&lt;\$\w+:\w+&gt;").unwrap());
static EMPTY_STRONG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<strong>\s*</strong>").unwrap());
static EMPTY_EM_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<em>\s*</em>").unwrap());
static EMPTY_P_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<p>\s*</p>").unwrap());
use std::fs;
use std::path::Path;
use uuid::Uuid;

use super::parser::comments::{read_comments, ScrivComment};
use super::parser::rtf_prepare::InlineNote;
use super::parser::scrivx::{get_data_file_path, sanitize_file_extension};
use super::parser::{
    get_media_path, get_rtf_path, parse_scrivx, rtf_file_to_markdown, BinderItem, ScrivenerProject,
};
use chickenscratch_core::core::project::{fidelity, writer};
use chickenscratch_core::models::{Comment, Document, Project, TreeNode};
use chickenscratch_core::utils::error::ChiknError;
use std::collections::BTreeMap;

/// Converts a Scrivener .scriv project to .chikn format.
///
/// # Arguments
/// * `scriv_path` - Path to .scriv directory
/// * `output_path` - Path where .chikn project will be created
///
/// # Returns
/// * `Ok(Project)` - Converted .chikn project
/// * `Err(ChiknError)` on conversion failure
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use chikn_converter::import_scriv;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let project = import_scriv(
///     Path::new("MyNovel.scriv"),
///     Path::new("MyNovel.chikn"),
///     None,  // optional override for the pandoc binary path
/// )?;
/// # Ok(()) }
/// ```
pub fn import_scriv(
    scriv_path: &Path,
    output_path: &Path,
    pandoc_path: Option<&Path>,
) -> Result<Project, ChiknError> {
    let report = import_scriv_with_report(scriv_path, output_path, pandoc_path)?;
    for warning in &report.warnings {
        eprintln!("Import warning: {warning}");
    }
    Ok(report.project)
}

/// A finished import plus plain-English problems that did not stop it
/// (an attachment that could not be copied, a first revision that could
/// not be saved). Callers with a UI show `warnings` to the writer.
#[derive(Debug)]
pub struct ImportReport {
    pub project: Project,
    pub warnings: Vec<String>,
}

/// [`import_scriv`], returning the problems that did not stop the import
/// instead of printing them (PLAN_IMPORT_MEDIA_SAFETY step 2).
pub fn import_scriv_with_report(
    scriv_path: &Path,
    output_path: &Path,
    pandoc_path: Option<&Path>,
) -> Result<ImportReport, ChiknError> {
    // Find .scrivx file
    let scrivx_file = find_scrivx_file(scriv_path)?;

    // Parse Scrivener project
    let scriv_project = parse_scrivx(&scrivx_file)?;
    validate_media_file_extensions(&scriv_project.binder)?;

    // Create .chikn project. A project the engine itself just initialized
    // probes Full by construction, so it yields a write token.
    let mut chikn_project = writer::create_project(output_path, &scriv_project.name)?;
    let token = fidelity::acquire_write_token(output_path)?;

    // Pre-pass: build Scrivener UUID -> chikn path map from the entire binder tree
    // This must happen before conversion so all links can be resolved
    let mut uuid_to_path: HashMap<String, String> = HashMap::new();
    build_uuid_map_root(&scriv_project.binder, scriv_path, &mut uuid_to_path)?;

    // Convert binder items to documents and hierarchy
    let mut documents = HashMap::new();
    let roots = convert_binder_items(
        &scriv_project,
        &scriv_project.binder,
        scriv_path,
        &mut documents,
        &mut uuid_to_path,
        pandoc_path,
    )?;

    // Second pass: clean up Scrivener markup in all document content
    for doc in documents.values_mut() {
        doc.content = clean_scrivener_markup(&doc.content, &uuid_to_path);
    }

    // Shape the binder: Manuscript, Research, Trash at the top (slice 2).
    chikn_project.hierarchy = ensure_project_structure(roots);
    chikn_project.documents = documents;

    let revision_message = format!("Imported from Scrivener: {}", scriv_project.name);
    let mut warnings = Vec::new();
    token.with_write_permit(output_path, |permit| {
        // Copy media files into the project FIRST, as part of the same
        // logical import operation (the writer never writes non-.md content,
        // so the later write_project cannot clobber them). An attachment
        // that fails is dropped from the project before it is written: a
        // hierarchy entry for a missing file probes Degraded, which would
        // leave the writer an "editable" project whose saves are refused
        // (review finding ar-1). The operation permit is intentionally
        // reused through the initial revision so no intermediate tree is
        // re-probed.
        let mut failed_paths: HashSet<String> = HashSet::new();
        for (key, value) in &uuid_to_path {
            if let Some(uuid) = key.strip_prefix("__media__") {
                let parts: Vec<&str> = value.splitn(2, '|').collect();
                if parts.len() == 2 {
                    let src = Path::new(parts[0]);
                    // Through core: safe-path parents, atomic write (I2).
                    let copied = fs::read(src).map_err(ChiknError::from).and_then(|bytes| {
                        writer::write_project_app_file(permit, Path::new(parts[1]), &bytes)
                    });
                    if let Err(e) = copied {
                        warnings.push(format!(
                            "The attachment '{}' (Scrivener item {}) could not be copied: {}",
                            parts[1], uuid, e
                        ));
                        failed_paths.insert(parts[1].to_string());
                    }
                }
            }
        }
        for (key, message) in &uuid_to_path {
            if key.starts_with("__skipped__") {
                warnings.push(message.clone());
            }
        }
        if !failed_paths.is_empty() {
            chikn_project
                .documents
                .retain(|_, doc| !failed_paths.contains(&doc.path));
            remove_hierarchy_paths(&mut chikn_project.hierarchy, &failed_paths);
        }

        // Save the converted project.
        writer::write_project(&mut chikn_project, permit)?;

        // Initial commit with all converted content.
        if let Err(e) =
            chickenscratch_core::core::git::save_revision(output_path, &revision_message, permit)
        {
            warnings.push(format!(
                "Your text was imported, but the first saved version could not be created ({e}). Use Save Revision to create one."
            ));
        }
        Ok(())
    })?;

    Ok(ImportReport {
        project: chikn_project,
        warnings,
    })
}

/// Drop hierarchy document nodes whose path is in `paths`, at any depth.
fn remove_hierarchy_paths(nodes: &mut Vec<TreeNode>, paths: &HashSet<String>) {
    nodes.retain(|node| !matches!(node, TreeNode::Document { path, .. } if paths.contains(path)));
    for node in nodes.iter_mut() {
        if let TreeNode::Folder { children, .. } = node {
            remove_hierarchy_paths(children, paths);
        }
    }
}

/// Finds the .scrivx file in a .scriv directory
fn find_scrivx_file(scriv_path: &Path) -> Result<std::path::PathBuf, ChiknError> {
    for entry in fs::read_dir(scriv_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("scrivx") {
            return Ok(path);
        }
    }

    Err(ChiknError::NotFound(format!(
        "No .scrivx file found in {}",
        scriv_path.display()
    )))
}

/// Which of the three top-level `.chikn` folders a Scrivener root item
/// belongs to (PLAN_SCRIVENER_FIDELITY slice 2). Only the Draft folder is
/// the manuscript; Research and Trash keep their roles; everything else at
/// the root (Characters, Places, Notes, Front Matter, Template Sheets) is
/// reference material and goes under Research.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RootKind {
    Draft,
    Research,
    Trash,
    Other,
}

fn root_kind(item: &BinderItem) -> RootKind {
    match item.item_type.as_str() {
        "DraftFolder" => RootKind::Draft,
        "ResearchFolder" => RootKind::Research,
        "TrashFolder" => RootKind::Trash,
        _ => RootKind::Other,
    }
}

/// The on-disk folder for a root item's documents. The engine reads
/// documents from `manuscript/` and `research/` (and the entity folders)
/// only, and compiles `manuscript/` paths only, so trashed and reference
/// documents live under `research/` whatever their binder folder.
fn root_target_folder(item: &BinderItem) -> &'static str {
    match root_kind(item) {
        RootKind::Draft => "manuscript",
        _ => "research",
    }
}

/// Is this Scrivener item compiled? `<IncludeInCompile>No</IncludeInCompile>`
/// excludes it; absent means yes (review finding plan-3 R1-3).
fn include_in_compile(item: &BinderItem) -> bool {
    item.metadata
        .as_ref()
        .and_then(|m| m.include_in_compile.as_deref())
        .map(|v| !v.trim().eq_ignore_ascii_case("no"))
        .unwrap_or(true)
}

/// Pre-pass over the root items, each with its own target folder.
fn build_uuid_map_root(
    items: &[BinderItem],
    scriv_path: &Path,
    uuid_to_path: &mut HashMap<String, String>,
) -> Result<(), ChiknError> {
    for item in items {
        build_uuid_map(
            std::slice::from_ref(item),
            scriv_path,
            uuid_to_path,
            root_target_folder(item),
        )?;
    }
    Ok(())
}

/// Pre-pass: builds a map of Scrivener UUID -> expected chikn .md path.
/// This traverses the entire binder tree before conversion so that
/// scrivlnk:// references in any document can be resolved.
fn build_uuid_map(
    items: &[BinderItem],
    scriv_path: &Path,
    uuid_to_path: &mut HashMap<String, String>,
    target_folder: &str,
) -> Result<(), ChiknError> {
    // Track slugs we've seen to handle collisions (mirrors unique_slug logic)
    for item in items {
        match item.item_type.as_str() {
            "ResearchFolder" => {
                build_uuid_map(&item.children.items, scriv_path, uuid_to_path, "research")?;
            }

            "DraftFolder" | "Folder" | "TrashFolder" => {
                // For folders, map UUID to the first child doc if one exists
                if let Some(path) = find_first_text_path(&item.children.items, target_folder) {
                    uuid_to_path.insert(item.uuid.clone(), path);
                }
                // Also map if the folder itself has content
                let rtf_path = get_rtf_path(scriv_path, &item.uuid)?;
                if rtf_path.exists() {
                    let name = item.title.clone().unwrap_or_else(|| "folder".to_string());
                    let slug = chickenscratch_core::utils::slug::slugify(&name);
                    uuid_to_path
                        .insert(item.uuid.clone(), format!("{}/{}.md", target_folder, slug));
                }
                build_uuid_map(
                    &item.children.items,
                    scriv_path,
                    uuid_to_path,
                    target_folder,
                )?;
            }

            "Text" => {
                let name = item.title.clone().unwrap_or_else(|| "untitled".to_string());
                let slug = chickenscratch_core::utils::slug::slugify(&name);
                uuid_to_path.insert(item.uuid.clone(), format!("{}/{}.md", target_folder, slug));
                // Scrivener Text items can have children too
                if !item.children.items.is_empty() {
                    build_uuid_map(
                        &item.children.items,
                        scriv_path,
                        uuid_to_path,
                        target_folder,
                    )?;
                }
            }

            // Media types (PDF, Image, etc.) — map by file extension
            _ => {
                if let Some(ext) = item
                    .metadata
                    .as_ref()
                    .and_then(|m| m.file_extension.as_deref())
                {
                    let ext = sanitize_file_extension(ext)?;
                    let name = item.title.clone().unwrap_or_else(|| "untitled".to_string());
                    let slug = chickenscratch_core::utils::slug::slugify(&name);
                    uuid_to_path.insert(
                        item.uuid.clone(),
                        format!("{}/{}.{}", target_folder, slug, ext),
                    );
                }
            }
        }
    }

    Ok(())
}

/// A folder without text of its own links to its first text descendant. The
/// pre-pass predicted that descendant's path; after the descendant has been
/// converted its actual path is known, so the alias is refreshed from it
/// (review finding s2-1: a prediction could name a same-titled document
/// elsewhere).
fn alias_folder_to_first_text(item: &BinderItem, uuid_to_path: &mut HashMap<String, String>) {
    if let Some(path) =
        first_text_uuid(&item.children.items).and_then(|uuid| uuid_to_path.get(uuid).cloned())
    {
        uuid_to_path.insert(item.uuid.clone(), path);
    }
}

fn first_text_uuid(items: &[BinderItem]) -> Option<&str> {
    for item in items {
        if item.item_type == "Text" {
            return Some(item.uuid.as_str());
        }
        if let Some(uuid) = first_text_uuid(&item.children.items) {
            return Some(uuid);
        }
    }
    None
}

/// Finds the .md path of the first Text item in a binder subtree.
fn find_first_text_path(items: &[BinderItem], target_folder: &str) -> Option<String> {
    for item in items {
        if item.item_type == "Text" {
            let name = item.title.clone().unwrap_or_else(|| "untitled".to_string());
            let slug = chickenscratch_core::utils::slug::slugify(&name);
            return Some(format!("{}/{}.md", target_folder, slug));
        }
        if let Some(path) = find_first_text_path(&item.children.items, target_folder) {
            return Some(path);
        }
    }
    None
}

/// Converts the Scrivener root items to .chikn nodes, each tagged with the
/// top-level folder it belongs to; `ensure_project_structure` shapes them.
fn convert_binder_items(
    project: &ScrivenerProject,
    items: &[BinderItem],
    scriv_path: &Path,
    documents: &mut HashMap<String, Document>,
    uuid_to_path: &mut HashMap<String, String>,
    pandoc_path: Option<&Path>,
) -> Result<Vec<(RootKind, TreeNode)>, ChiknError> {
    let mut roots = Vec::new();
    for item in items {
        let nodes = convert_binder_items_inner(
            project,
            std::slice::from_ref(item),
            scriv_path,
            documents,
            uuid_to_path,
            None,
            root_target_folder(item),
            pandoc_path,
        )?;
        roots.extend(nodes.into_iter().map(|node| (root_kind(item), node)));
    }
    Ok(roots)
}

/// Converts Scrivener binder items with parent and target folder tracking
#[allow(clippy::too_many_arguments)]
fn convert_binder_items_inner(
    project: &ScrivenerProject,
    items: &[BinderItem],
    scriv_path: &Path,
    documents: &mut HashMap<String, Document>,
    uuid_to_path: &mut HashMap<String, String>,
    parent_id: Option<String>,
    target_folder: &str,
    pandoc_path: Option<&Path>,
) -> Result<Vec<TreeNode>, ChiknError> {
    let mut hierarchy = Vec::new();

    for item in items {
        match item.item_type.as_str() {
            // Every folder kind, the Research root and the Trash included:
            // nothing a writer parked in Scrivener's Trash is dropped
            // (slice 2), and the Research root carries notes and metadata
            // like any folder (review finding s3-2). The target folder on
            // disk was chosen at the root.
            "DraftFolder" | "Folder" | "TrashFolder" | "ResearchFolder" => {
                let folder_id = Uuid::new_v4().to_string();
                let folder_name = item.title.clone().unwrap_or_else(|| "Folder".to_string());

                let children = convert_binder_items_inner(
                    project,
                    &item.children.items,
                    scriv_path,
                    documents,
                    uuid_to_path,
                    Some(folder_id.clone()),
                    target_folder,
                    pandoc_path,
                )?;

                // Folders in Scrivener can also have their own text, and
                // carry a synopsis, label, status, keywords, custom
                // metadata or notes. A `.chikn` folder node holds none of
                // that, so a folder with any of it gets a companion
                // document of its own name, first in the folder, even when
                // the text is empty (review finding plan-3 R2-2).
                let rtf_path = get_rtf_path(scriv_path, &item.uuid)?;
                let DocumentText { content, comments } = if rtf_path.exists() {
                    read_document_text(scriv_path, &rtf_path, item, uuid_to_path, pandoc_path)?
                } else {
                    DocumentText::default()
                };
                let side = side_values(project, scriv_path, item, uuid_to_path, pandoc_path)?;
                if !content.trim().is_empty() || !side.is_empty() {
                    let doc_id = Uuid::new_v4().to_string();
                    let slug = chickenscratch_core::utils::slug::unique_slug(
                        &folder_name,
                        &format!("{}/", target_folder),
                        documents,
                    );
                    let doc_path = format!("{}/{}.md", target_folder, slug);
                    let created = item
                        .created
                        .clone()
                        .unwrap_or_else(|| Utc::now().to_rfc3339());
                    let modified = item
                        .modified
                        .clone()
                        .unwrap_or_else(|| Utc::now().to_rfc3339());

                    uuid_to_path.insert(item.uuid.clone(), doc_path.clone());

                    let document = Document {
                        id: doc_id.clone(),
                        name: folder_name.clone(),
                        path: doc_path.clone(),
                        content,
                        comments,
                        parent_id: Some(folder_id.clone()),
                        created,
                        modified,
                        synopsis: side.synopsis,
                        label: side.label,
                        status: side.status,
                        keywords: side.keywords,
                        include_in_compile: include_in_compile(item),
                        fields: side.fields,
                        ..Document::default()
                    };
                    documents.insert(doc_id.clone(), document);

                    let mut all_children = vec![TreeNode::Document {
                        id: doc_id,
                        name: folder_name.clone(),
                        path: doc_path,
                    }];
                    all_children.extend(children);

                    hierarchy.push(TreeNode::Folder {
                        id: folder_id,
                        name: folder_name,
                        children: all_children,
                    });
                    continue;
                }

                alias_folder_to_first_text(item, uuid_to_path);
                hierarchy.push(TreeNode::Folder {
                    id: folder_id,
                    name: folder_name,
                    children,
                });
            }

            // Text documents
            "Text" => {
                let doc_id = Uuid::new_v4().to_string();
                let doc_name = item.title.clone().unwrap_or_else(|| "Untitled".to_string());

                // A Scrivener text item can have children. It then becomes
                // a folder of its own name whose first document is the
                // item's text, followed by the children in order (review
                // finding plan-3 R1-2: the children used to be dropped).
                let container_id = if item.children.items.is_empty() {
                    None
                } else {
                    Some(Uuid::new_v4().to_string())
                };
                let doc_parent = container_id.clone().or_else(|| parent_id.clone());

                let rtf_path = get_rtf_path(scriv_path, &item.uuid)?;
                let DocumentText { content, comments } = if rtf_path.exists() {
                    read_document_text(scriv_path, &rtf_path, item, uuid_to_path, pandoc_path)?
                } else {
                    DocumentText::default()
                };

                let slug = chickenscratch_core::utils::slug::unique_slug(
                    &doc_name,
                    &format!("{}/", target_folder),
                    documents,
                );
                let doc_path = format!("{}/{}.md", target_folder, slug);

                let created = item
                    .created
                    .clone()
                    .unwrap_or_else(|| Utc::now().to_rfc3339());
                let modified = item
                    .modified
                    .clone()
                    .unwrap_or_else(|| Utc::now().to_rfc3339());

                uuid_to_path.insert(item.uuid.clone(), doc_path.clone());

                let side = side_values(project, scriv_path, item, uuid_to_path, pandoc_path)?;
                let document = Document {
                    id: doc_id.clone(),
                    name: doc_name.clone(),
                    path: doc_path.clone(),
                    content,
                    comments,
                    parent_id: doc_parent,
                    created,
                    modified,
                    synopsis: side.synopsis,
                    label: side.label,
                    status: side.status,
                    keywords: side.keywords,
                    include_in_compile: include_in_compile(item),
                    fields: side.fields,
                    ..Document::default()
                };

                documents.insert(doc_id.clone(), document);

                let doc_node = TreeNode::Document {
                    id: doc_id,
                    name: doc_name.clone(),
                    path: doc_path,
                };
                match container_id {
                    Some(folder_id) => {
                        let mut children = vec![doc_node];
                        children.extend(convert_binder_items_inner(
                            project,
                            &item.children.items,
                            scriv_path,
                            documents,
                            uuid_to_path,
                            Some(folder_id.clone()),
                            target_folder,
                            pandoc_path,
                        )?);
                        hierarchy.push(TreeNode::Folder {
                            id: folder_id,
                            name: doc_name,
                            children,
                        });
                    }
                    None => hierarchy.push(doc_node),
                }
            }

            // Any other type — check if it's a media file we can copy
            other => {
                let title = item.title.as_deref().unwrap_or("untitled");
                let ext = item
                    .metadata
                    .as_ref()
                    .and_then(|m| m.file_extension.as_deref());

                if let Some(ext) = ext {
                    let ext = sanitize_file_extension(ext)?;
                    // Media file — copy into the project
                    let media_src = get_media_path(scriv_path, &item.uuid, &ext)?;
                    if media_src.exists() {
                        let doc_id = Uuid::new_v4().to_string();
                        let doc_name = item.title.clone().unwrap_or_else(|| "Untitled".to_string());
                        // unique_slug only checks `.md` paths; two attachments
                        // with the same title and extension must still land
                        // at distinct files (review finding ar-1, reopen).
                        let base_slug = chickenscratch_core::utils::slug::unique_slug(
                            &doc_name,
                            &format!("{}/", target_folder),
                            documents,
                        );
                        let mut dest_rel = format!("{}/{}.{}", target_folder, base_slug, ext);
                        let mut counter = 1;
                        while documents
                            .values()
                            .any(|d| d.path.eq_ignore_ascii_case(&dest_rel))
                        {
                            dest_rel =
                                format!("{}/{}-{}.{}", target_folder, base_slug, counter, ext);
                            counter += 1;
                        }

                        // The actual file copy happens after write_project, below

                        // We need the output project path — get it from the hierarchy context
                        // For now, record as a document entry so it appears in the tree
                        let doc_path = dest_rel.clone();

                        let created = item
                            .created
                            .clone()
                            .unwrap_or_else(|| Utc::now().to_rfc3339());
                        let modified = item
                            .modified
                            .clone()
                            .unwrap_or_else(|| Utc::now().to_rfc3339());

                        // An attachment carries side values too; they live
                        // in its `.meta` sidecar (review finding s3-2).
                        let side =
                            side_values(project, scriv_path, item, uuid_to_path, pandoc_path)?;
                        let document = Document {
                            id: doc_id.clone(),
                            name: doc_name.clone(),
                            path: doc_path.clone(),
                            content: String::new(),
                            parent_id: parent_id.clone(),
                            created,
                            modified,
                            synopsis: side.synopsis,
                            label: side.label,
                            status: side.status,
                            keywords: side.keywords,
                            fields: side.fields,
                            ..Document::default()
                        };

                        documents.insert(doc_id.clone(), document);

                        hierarchy.push(TreeNode::Document {
                            id: doc_id,
                            name: doc_name,
                            path: doc_path,
                        });

                        // Track the source file to copy after project is created
                        // Store in uuid_to_path so we can find it later
                        uuid_to_path.insert(
                            format!("__media__{}", item.uuid),
                            format!("{}|{}", media_src.display(), dest_rel),
                        );
                        // Links resolve against the destination actually
                        // chosen, not the pre-pass prediction (review finding
                        // s2-1: two attachments titled alike collided).
                        uuid_to_path.insert(item.uuid.clone(), dest_rel.clone());
                    } else {
                        // Reported to the writer via ImportReport (ar-2):
                        // the attachment is not in the Scrivener package.
                        uuid_to_path.insert(
                            format!("__skipped__{}", item.uuid),
                            format!("The attachment '{title}' was not found in the Scrivener project and was not imported."),
                        );
                    }
                } else {
                    uuid_to_path.insert(
                        format!("__skipped__{}", item.uuid),
                        format!("The Scrivener item '{title}' ({other}) is not a kind ChickenScratch can import and was left out."),
                    );
                }
            }
        }
    }

    Ok(hierarchy)
}

/// The side values Scrivener keeps for an item (PLAN_SCRIVENER_FIDELITY
/// slice 3): synopsis, label, status, keywords, custom metadata and notes.
#[derive(Default)]
struct SideValues {
    synopsis: Option<String>,
    label: Option<String>,
    status: Option<String>,
    keywords: Option<Vec<String>>,
    fields: BTreeMap<String, serde_yaml::Value>,
}

impl SideValues {
    fn is_empty(&self) -> bool {
        self.synopsis.is_none()
            && self.label.is_none()
            && self.status.is_none()
            && self.keywords.is_none()
            && self.fields.is_empty()
    }
}

/// Custom metadata keys are `custom.<title>` while every field title in the
/// project is unique; two fields sharing a title switch the whole project
/// to `custom.<id>` plus one `custom_field_titles` map, so no key can
/// collide (review finding plan-3 R1-7).
fn custom_keys_by_title(project: &ScrivenerProject) -> bool {
    let mut seen = HashSet::new();
    project
        .custom_fields
        .iter()
        .all(|(_, title)| !title.trim().is_empty() && seen.insert(title.trim().to_string()))
}

/// Gathers an item's side values: `synopsis.txt` (else the `<Synopsis>`
/// element), label, status and keywords resolved through the project
/// tables, custom metadata, and `notes.rtf` as Markdown under `notes`.
fn side_values(
    project: &ScrivenerProject,
    scriv_path: &Path,
    item: &BinderItem,
    uuid_to_path: &mut HashMap<String, String>,
    pandoc_path: Option<&Path>,
) -> Result<SideValues, ChiknError> {
    let meta = item.metadata.as_ref();
    let mut side = SideValues {
        label: project.label_name(meta),
        status: project.status_name(meta),
        ..SideValues::default()
    };

    let clean = |s: String| {
        let t = s.trim().to_string();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    };
    let synopsis_path = get_data_file_path(scriv_path, &item.uuid, "synopsis.txt")?;
    // A missing file is no synopsis; any other failure is reported, not
    // swallowed (review finding s3-3).
    side.synopsis = match fs::read_to_string(&synopsis_path) {
        Ok(text) => clean(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            let title = item.title.as_deref().unwrap_or("Untitled");
            uuid_to_path.insert(
                format!("__skipped__synopsis-{}", item.uuid),
                format!(
                    "'{title}': its synopsis could not be read ({e}); the synopsis was left out."
                ),
            );
            None
        }
    }
    .or_else(|| meta.and_then(|m| m.synopsis.clone()).and_then(clean));

    let keywords = project.keyword_names(meta);
    if !keywords.is_empty() {
        side.keywords = Some(keywords);
    }

    if let Some(custom) = meta.and_then(|m| m.custom.as_ref()) {
        let by_title = custom_keys_by_title(project);
        let titles: HashMap<&str, &str> = project
            .custom_fields
            .iter()
            .map(|(id, title)| (id.as_str(), title.as_str()))
            .collect();
        let mut used_ids = false;
        for entry in &custom.items {
            let Some(value) = entry.value.clone().and_then(clean) else {
                continue;
            };
            let key = match (by_title, titles.get(entry.field_id.as_str())) {
                (true, Some(title)) => format!("custom.{}", title.trim()),
                _ => {
                    used_ids = true;
                    format!("custom.{}", entry.field_id)
                }
            };
            side.fields.insert(key, serde_yaml::Value::String(value));
        }
        if used_ids {
            let mut map = serde_yaml::Mapping::new();
            for (id, title) in &project.custom_fields {
                map.insert(
                    serde_yaml::Value::String(id.clone()),
                    serde_yaml::Value::String(title.clone()),
                );
            }
            side.fields.insert(
                "custom_field_titles".to_string(),
                serde_yaml::Value::Mapping(map),
            );
        }
    }

    let notes_path = get_data_file_path(scriv_path, &item.uuid, "notes.rtf")?;
    if notes_path.exists() {
        let converted = rtf_file_to_markdown(&notes_path, pandoc_path)?;
        if converted.fell_back {
            let title = item.title.as_deref().unwrap_or("Untitled");
            uuid_to_path.insert(
                format!("__skipped__notes-{}", item.uuid),
                format!("'{title}': the formatting of its notes could not be read, so they were imported plain."),
            );
        }
        if let Some(notes) = clean(converted.markdown) {
            side.fields
                .insert("notes".to_string(), serde_yaml::Value::String(notes));
        }
    }

    Ok(side)
}

/// A document's converted text and the comments attached to it.
#[derive(Default)]
struct DocumentText {
    content: String,
    comments: Vec<Comment>,
}

/// One document's RTF as Markdown with its comments and footnotes attached.
/// When Pandoc refuses the file, the text comes in plain and the document
/// is named in the import warnings (PLAN_SCRIVENER_FIDELITY slice 1): the
/// import goes on instead of dying on one bad file.
fn read_document_text(
    scriv_path: &Path,
    rtf_path: &Path,
    item: &BinderItem,
    uuid_to_path: &mut HashMap<String, String>,
    pandoc_path: Option<&Path>,
) -> Result<DocumentText, ChiknError> {
    let converted = rtf_file_to_markdown(rtf_path, pandoc_path)?;
    if converted.fell_back {
        let title = item.title.as_deref().unwrap_or("Untitled");
        uuid_to_path.insert(
            format!("__skipped__plain-{}", item.uuid),
            format!("'{title}': its formatting could not be read, so the text was imported plain."),
        );
    }
    let scriv_comments = read_comments(scriv_path, &item.uuid)?;
    Ok(attach_annotations(
        &converted.markdown,
        scriv_comments,
        converted.inline_notes,
        item,
        uuid_to_path,
    ))
}

// The label may contain backslash-escaped characters: Pandoc writes `a\] b`
// for anchor text `a] b` (review finding s4-5).
static ANCHOR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[((?:\\.|[^\]\\])*)\]\((scrivcmt|scrivfn|scrivannot)://([^)\s]+)\)").unwrap()
});

/// Turns the anchors Pandoc (or the fallback) left in the Markdown into the
/// editor's own comments and footnotes (PLAN_SCRIVENER_FIDELITY slice 4):
///
/// - `[text](scrivcmt://ID)` with a comment body → `<span class="comment"
///   data-comment-id="ID">text</span>` plus a `Comment`; with a footnote
///   body → `text` followed by the footnote element; with no body → the
///   text alone, and a warning.
/// - `[​](scrivfn://ID)` (a lifted inline footnote) → the footnote element.
/// - `[​](scrivannot://ID)` (a lifted inline annotation) → a comment span
///   around the word before it (or after it when the line starts there).
/// - A comment body with no anchor in the text is kept as an unanchored
///   comment, marked as such, and named in the warnings. Nothing in
///   `content.comments` is dropped.
fn attach_annotations(
    markdown: &str,
    mut scriv_comments: Vec<ScrivComment>,
    inline_notes: Vec<InlineNote>,
    item: &BinderItem,
    uuid_to_path: &mut HashMap<String, String>,
) -> DocumentText {
    let title = item.title.clone().unwrap_or_else(|| "Untitled".to_string());
    let created = item
        .created
        .clone()
        .unwrap_or_else(|| Utc::now().to_rfc3339());
    let modified = item.modified.clone().unwrap_or_else(|| created.clone());
    let mut comments: Vec<Comment> = Vec::new();
    let mut inline: Vec<InlineNote> = inline_notes;
    let mut out = String::with_capacity(markdown.len());
    let mut last = 0;
    let mut warn = |key: &str, message: String| {
        uuid_to_path.insert(format!("__skipped__{key}-{}", item.uuid), message);
    };
    let mut orphan_anchors = 0;

    // All anchors first, so the look-ahead for a word after an annotation
    // can never run past the next anchor (review finding s4-2: it consumed
    // the next marker as the "word" and the cursor overtook the match).
    let matches: Vec<regex::Captures> = ANCHOR_RE.captures_iter(markdown).collect();
    for (index, caps) in matches.iter().enumerate() {
        let whole = caps.get(0).unwrap();
        let text = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let scheme = &caps[2];
        let id = &caps[3];
        let next_anchor_start = matches
            .get(index + 1)
            .map(|c| c.get(0).unwrap().start())
            .unwrap_or(markdown.len());
        out.push_str(&markdown[last..whole.start()]);
        last = whole.end();
        match scheme {
            "scrivcmt" => {
                if let Some(pos) = scriv_comments.iter().position(|c| c.id == id) {
                    let c = scriv_comments.remove(pos);
                    if c.footnote {
                        out.push_str(text);
                        out.push_str(&footnote_element(&c.body));
                    } else {
                        out.push_str(&comment_span(&c.id, text));
                        comments.push(Comment {
                            id: c.id,
                            body: c.body,
                            resolved: false,
                            created: created.clone(),
                            modified: modified.clone(),
                        });
                    }
                } else {
                    orphan_anchors += 1;
                    out.push_str(text);
                }
            }
            "scrivfn" => {
                if let Some(pos) = inline.iter().position(|n| n.id == id) {
                    let n = inline.remove(pos);
                    out.push_str(&footnote_element(&n.body));
                }
            }
            _ => {
                // An inline annotation: attach it to the word before it.
                if let Some(pos) = inline.iter().position(|n| n.id == id) {
                    let n = inline.remove(pos);
                    let anchored = wrap_previous_word(&mut out, &n.id);
                    if !anchored {
                        // Nothing before it on this line: take the next word,
                        // looking no further than the next anchor.
                        let rest = &markdown[last..next_anchor_start];
                        let trimmed = rest.trim_start();
                        let lead = rest.len() - trimmed.len();
                        let word_len = trimmed
                            .find(|ch: char| ch.is_whitespace())
                            .unwrap_or(trimmed.len());
                        if word_len > 0 {
                            out.push_str(&rest[..lead]);
                            out.push_str(&comment_span(&n.id, &trimmed[..word_len]));
                            last += lead + word_len;
                        } else {
                            out.push_str(&comment_span(&n.id, "•"));
                        }
                    }
                    comments.push(Comment {
                        id: n.id,
                        body: n.body,
                        resolved: false,
                        created: created.clone(),
                        modified: modified.clone(),
                    });
                }
            }
        }
    }
    out.push_str(&markdown[last..]);

    if orphan_anchors > 0 {
        warn(
            "orphan-anchor",
            format!("'{title}': {orphan_anchors} comment marker(s) had no comment text in the Scrivener project; the marked text was kept."),
        );
    }
    let leftovers: Vec<ScrivComment> = scriv_comments;
    if !leftovers.is_empty() {
        warn(
            "unanchored-comment",
            format!("'{title}': {} comment(s) had no place in the text; they are kept on the document as unanchored comments.", leftovers.len()),
        );
        for c in leftovers {
            let kind = if c.footnote { "Footnote" } else { "Comment" };
            comments.push(Comment {
                id: c.id,
                body: format!("{kind} with no place in the text: {}", c.body),
                resolved: false,
                created: created.clone(),
                modified: modified.clone(),
            });
        }
    }
    for n in inline {
        comments.push(Comment {
            id: n.id,
            body: format!("Note with no place in the text: {}", n.body),
            resolved: false,
            created: created.clone(),
            modified: modified.clone(),
        });
    }

    DocumentText {
        content: out,
        comments,
    }
}

/// Wraps the last word of `out` (on its current line) in a comment span;
/// false when the line has no word yet.
fn wrap_previous_word(out: &mut String, id: &str) -> bool {
    let line_start = out.rfind('\n').map(|p| p + 1).unwrap_or(0);
    let line = &out[line_start..];
    let trimmed_end = line.trim_end();
    if trimmed_end.is_empty() {
        return false;
    }
    let trailing = line.len() - trimmed_end.len();
    // The whitespace before the word may be more than one byte (a
    // non-breaking space from `\~`), so step over its real length (review
    // finding s4-1: a byte offset of one panicked on it).
    let word_start_rel = trimmed_end
        .rfind(|ch: char| ch.is_whitespace())
        .map(|p| p + trimmed_end[p..].chars().next().map_or(1, char::len_utf8))
        .unwrap_or(0);
    let word = &trimmed_end[word_start_rel..];
    if word.contains('<') || word.contains('>') {
        // Already markup (another span or element): do not nest into it.
        return false;
    }
    let span = comment_span(id, word);
    let tail = line[trimmed_end.len()..].to_string();
    let abs_start = line_start + word_start_rel;
    let _ = trailing;
    out.truncate(abs_start);
    out.push_str(&span);
    out.push_str(&tail);
    true
}

fn comment_span(id: &str, text: &str) -> String {
    format!(
        "<span class=\"comment\" data-comment-id=\"{}\">{text}</span>",
        html_attr(id)
    )
}

/// The editor's footnote element (`ui/src/components/editor/FootnoteNode.ts`).
fn footnote_element(body: &str) -> String {
    format!(
        "<sup class=\"footnote\" data-body=\"{}\">●</sup>",
        html_attr(body)
    )
}

fn html_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\n', "&#10;")
}

fn validate_media_file_extensions(items: &[BinderItem]) -> Result<(), ChiknError> {
    for item in items {
        match item.item_type.as_str() {
            "ResearchFolder" | "DraftFolder" | "Folder" | "TrashFolder" | "Text" => {
                validate_media_file_extensions(&item.children.items)?;
            }
            _ => {
                if let Some(ext) = item
                    .metadata
                    .as_ref()
                    .and_then(|metadata| metadata.file_extension.as_deref())
                {
                    sanitize_file_extension(ext)?;
                }
            }
        }
    }

    Ok(())
}

/// Cleans Scrivener-specific markup from converted Markdown content.
///
/// Handles:
/// - `scrivlnk://UUID` links → rewritten to relative .md paths
/// - `\<\$Scr_Ps::N\>` / `\<!\$Scr_Ps::N\>` → paragraph style tags, stripped
/// - `\<\$Scr_Cs::N\>` / `\<!\$Scr_Cs::N\>` → character style tags, stripped
/// - `{\\Scrv_annot ...}` → inline annotations, converted to HTML comments
///
/// Shape the converted roots into the three top-level folders: the Draft
/// folder's contents become Manuscript (by Scrivener type, whatever the
/// writer named the folder), the Research folder's contents Research, the
/// Trash folder's contents Trash, and every other root item goes under
/// Research as the reference material it is (slice 2; it used to be filed
/// under Manuscript).
fn ensure_project_structure(roots: Vec<(RootKind, TreeNode)>) -> Vec<TreeNode> {
    let mut manuscript_children: Vec<TreeNode> = Vec::new();
    let mut research_children: Vec<TreeNode> = Vec::new();
    let mut trash_children: Vec<TreeNode> = Vec::new();
    let mut manuscript_id = None;
    let mut research_id = None;
    let mut trash_id = None;

    for (kind, node) in roots {
        match (kind, node) {
            (RootKind::Draft, TreeNode::Folder { id, children, .. }) => {
                manuscript_id.get_or_insert(id);
                manuscript_children.extend(children);
            }
            (RootKind::Research, TreeNode::Folder { id, children, .. }) => {
                research_id.get_or_insert(id);
                research_children.extend(children);
            }
            (RootKind::Trash, TreeNode::Folder { id, children, .. }) => {
                trash_id.get_or_insert(id);
                trash_children.extend(children);
            }
            (RootKind::Draft, node) => manuscript_children.push(node),
            (RootKind::Trash, node) => trash_children.push(node),
            (_, node) => research_children.push(node),
        }
    }

    vec![
        TreeNode::Folder {
            id: manuscript_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            name: "Manuscript".to_string(),
            children: manuscript_children,
        },
        TreeNode::Folder {
            id: research_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            name: "Research".to_string(),
            children: research_children,
        },
        TreeNode::Folder {
            id: trash_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            name: "Trash".to_string(),
            children: trash_children,
        },
    ]
}

fn clean_scrivener_markup(content: &str, uuid_to_path: &HashMap<String, String>) -> String {
    let mut result = content.to_string();

    // Rewrite scrivlnk:// links to relative paths
    // In HTML: <a href="scrivlnk://UUID">text</a>
    result = SCRIV_LINK_RE
        .replace_all(&result, |caps: &regex::Captures| {
            let uuid = &caps[1];
            match uuid_to_path.get(uuid) {
                Some(path) => path.clone(),
                None => format!("scrivlnk://{}", uuid),
            }
        })
        .to_string();

    // Strip Scrivener compile style tags (HTML-escaped by Pandoc)
    // &lt;$Scr_Ps::N&gt; and &lt;!$Scr_Ps::N&gt;
    // &lt;$Scr_Cs::N&gt; and &lt;!$Scr_Cs::N&gt;
    result = SCR_TAG_HTML_RE.replace_all(&result, "").to_string();

    // Also catch unescaped variants
    result = SCR_TAG_RAW_RE.replace_all(&result, "").to_string();

    // Also catch backslash-escaped variants from older conversions
    result = SCR_TAG_ESC_RE.replace_all(&result, "").to_string();

    // Strip Scrivener custom variable placeholders
    // &lt;$custom:shortcut&gt; etc.
    result = SCR_VAR_RE.replace_all(&result, "").to_string();

    // Scrivener uses hard line breaks between paragraphs in RTF.
    // Pandoc converts these to <br /> within a single <p> tag.
    // Split into proper separate paragraphs.
    result = result.replace("<br />\n", "</p>\n<p>");
    result = result.replace("<br/>", "</p><p>");

    // Clean up empty elements left by stripped tags
    result = EMPTY_STRONG_RE.replace_all(&result, "").to_string();
    result = EMPTY_EM_RE.replace_all(&result, "").to_string();
    result = EMPTY_P_RE.replace_all(&result, "").to_string();

    result
}

#[cfg(test)]
mod tests {
    /// I2 guard (PLAN_IMPORT_MEDIA_SAFETY step 1): the importer must write
    /// into the project only through `chickenscratch-core`. A raw
    /// `fs::copy` bypassed the safe-path and atomic-write machinery.
    #[test]
    fn importer_writes_project_files_only_through_core() {
        let source = include_str!("mod.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        for needle in [
            concat!("fs::", "copy("),
            concat!("fs::", "write("),
            concat!("fs::", "create_dir"),
            concat!("File::", "create("),
        ] {
            assert!(
                !production.contains(needle),
                "importer production code must not call {needle}"
            );
        }
    }

    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_find_scrivx_file() {
        let temp_dir = TempDir::new().unwrap();
        let scriv_path = temp_dir.path().join("Test.scriv");
        fs::create_dir(&scriv_path).unwrap();

        // Create .scrivx file
        let scrivx_file = scriv_path.join("Test.scrivx");
        fs::write(&scrivx_file, "<?xml version=\"1.0\"?><ScrivenerProject/>").unwrap();

        let result = find_scrivx_file(&scriv_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), scrivx_file);
    }

    fn pandoc_present() -> bool {
        std::process::Command::new("pandoc")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// PLAN_SCRIVENER_FIDELITY slice 1: a document Pandoc refuses comes in
    /// as plain text with a warning naming it; the import does not die.
    #[test]
    fn unreadable_rtf_imports_plain_with_a_warning() {
        if !pandoc_present() {
            eprintln!("Skipping test: Pandoc not installed");
            return;
        }
        let temp_dir = TempDir::new().unwrap();
        let scriv_path = temp_dir.path().join("Broken.scriv");
        let output_path = temp_dir.path().join("Broken.chikn");
        let good = "A1B2C3D4-0000-4000-8000-000000000001";
        let bad = "A1B2C3D4-0000-4000-8000-000000000002";
        for (uuid, rtf) in [
            (good, "{\\rtf1\\ansi Fine text.\\\n}"),
            // Truncated file: Pandoc stops with "unexpected end of input".
            (bad, "{\\rtf1\\ansi Rescued *text* with {\\field{\\*\\fldinst{HYPERLINK \"scrivcmt://X-1\"}}{\\fldrslt a link}}"),
        ] {
            let dir = scriv_path.join("Files").join("Data").join(uuid);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("content.rtf"), rtf).unwrap();
        }
        fs::write(
            scriv_path.join("Broken.scrivx"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<ScrivenerProject Version="2.0">
    <Binder>
        <BinderItem UUID="00000000-0000-4000-8000-00000000000D" Type="DraftFolder">
            <Title>Draft</Title>
            <Children>
                <BinderItem UUID="{good}" Type="Text"><Title>Fine</Title></BinderItem>
                <BinderItem UUID="{bad}" Type="Text"><Title>Damaged</Title></BinderItem>
            </Children>
        </BinderItem>
    </Binder>
</ScrivenerProject>"#
            ),
        )
        .unwrap();

        let report = import_scriv_with_report(&scriv_path, &output_path, None)
            .expect("one unreadable document must not fail the import");
        let damaged = report
            .project
            .documents
            .values()
            .find(|d| d.name == "Damaged")
            .expect("the damaged document is imported");
        // The comment anchor survived the fallback (slice 1) and, having no
        // comment body in the project, was unwrapped and warned (slice 4).
        assert_eq!(
            damaged.content, "Rescued \\*text\\* with a link",
            "plain text, Markdown-escaped, anchor text kept"
        );
        let fine = report
            .project
            .documents
            .values()
            .find(|d| d.name == "Fine")
            .unwrap();
        assert_eq!(fine.content.trim(), "Fine text.");
        assert_eq!(report.warnings.len(), 2, "{:?}", report.warnings);
        assert!(
            report.warnings.iter().all(|w| w.starts_with("'Damaged':")),
            "the warnings name the document: {:?}",
            report.warnings
        );
        assert!(
            report.warnings.iter().any(|w| w.contains("imported plain")),
            "{:?}",
            report.warnings
        );
        assert!(
            report.warnings.iter().any(|w| w.contains("comment marker")),
            "{:?}",
            report.warnings
        );
    }

    /// PLAN_SCRIVENER_FIDELITY slice 2: the binder lands where Scrivener had
    /// it. Only the Draft folder (whatever its title) is the Manuscript;
    /// other root folders go under Research; the Trash is kept; a text item
    /// with children becomes a folder with its text first; IncludeInCompile
    /// No is honoured, and all of it survives reopening and compile.
    #[test]
    fn binder_lands_where_scrivener_had_it() {
        if !pandoc_present() {
            eprintln!("Skipping test: Pandoc not installed");
            return;
        }
        let temp_dir = TempDir::new().unwrap();
        let scriv_path = temp_dir.path().join("Shape.scriv");
        let output_path = temp_dir.path().join("Shape.chikn");
        let ids: Vec<String> = (1..=8)
            .map(|n| format!("B1B2C3D4-0000-4000-8000-00000000000{n}"))
            .collect();
        let texts = [
            (&ids[0], "Chapter one text."),
            (&ids[1], "Container text."),
            (&ids[2], "Scene text."),
            (&ids[3], "Deep text."),
            (&ids[4], "Hero sketch."),
            (&ids[5], "Research note."),
            (&ids[6], "Old draft, binned."),
            (&ids[7], "Private note, not compiled."),
        ];
        for (uuid, body) in texts {
            let dir = scriv_path.join("Files").join("Data").join(uuid);
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join("content.rtf"),
                format!("{{\\rtf1\\ansi {body}\\\n}}"),
            )
            .unwrap();
        }
        fs::write(
            scriv_path.join("Shape.scrivx"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<ScrivenerProject Version="2.0">
    <Binder>
        <BinderItem UUID="00000000-0000-4000-8000-00000000000D" Type="DraftFolder">
            <Title>Novel</Title>
            <Children>
                <BinderItem UUID="{a}" Type="Text"><Title>Chapter One</Title></BinderItem>
                <BinderItem UUID="{b}" Type="Text">
                    <Title>Container</Title>
                    <Children>
                        <BinderItem UUID="{c}" Type="Text"><Title>Scene</Title></BinderItem>
                        <BinderItem UUID="00000000-0000-4000-8000-00000000000F" Type="Folder">
                            <Title>Part</Title>
                            <Children>
                                <BinderItem UUID="{d}" Type="Text"><Title>Deep</Title></BinderItem>
                            </Children>
                        </BinderItem>
                    </Children>
                </BinderItem>
                <BinderItem UUID="{h}" Type="Text">
                    <Title>Private</Title>
                    <MetaData><IncludeInCompile>No</IncludeInCompile></MetaData>
                </BinderItem>
            </Children>
        </BinderItem>
        <BinderItem UUID="00000000-0000-4000-8000-00000000000C" Type="Folder">
            <Title>Characters</Title>
            <Children>
                <BinderItem UUID="{e}" Type="Text"><Title>Hero</Title></BinderItem>
            </Children>
        </BinderItem>
        <BinderItem UUID="00000000-0000-4000-8000-00000000000E" Type="ResearchFolder">
            <Title>Research</Title>
            <Children>
                <BinderItem UUID="{f}" Type="Text"><Title>Note</Title></BinderItem>
            </Children>
        </BinderItem>
        <BinderItem UUID="00000000-0000-4000-8000-00000000000A" Type="TrashFolder">
            <Title>Trash</Title>
            <Children>
                <BinderItem UUID="{g}" Type="Text"><Title>Old</Title></BinderItem>
            </Children>
        </BinderItem>
    </Binder>
</ScrivenerProject>"#,
                a = ids[0],
                b = ids[1],
                c = ids[2],
                d = ids[3],
                e = ids[4],
                f = ids[5],
                g = ids[6],
                h = ids[7],
            ),
        )
        .unwrap();

        let report = import_scriv_with_report(&scriv_path, &output_path, None).expect("imports");
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);

        // Reopen through the engine so persistence is what is checked.
        let project = chickenscratch_core::core::project::reader::read_project(&output_path)
            .expect("reopens");
        let names: Vec<&str> = project.hierarchy.iter().map(|n| n.name()).collect();
        assert_eq!(names, ["Manuscript", "Research", "Trash"]);
        let folder = |name: &str| -> &[TreeNode] {
            match project.hierarchy.iter().find(|n| n.name() == name) {
                Some(TreeNode::Folder { children, .. }) => children,
                _ => panic!("{name} folder missing"),
            }
        };
        let doc = |name: &str| -> &Document {
            project
                .documents
                .values()
                .find(|d| d.name == name)
                .unwrap_or_else(|| panic!("document {name} missing"))
        };

        // Manuscript: Chapter One, the Container folder, Private.
        let manuscript: Vec<&str> = folder("Manuscript").iter().map(|n| n.name()).collect();
        assert_eq!(manuscript, ["Chapter One", "Container", "Private"]);
        match &folder("Manuscript")[1] {
            TreeNode::Folder { children, .. } => {
                let inner: Vec<&str> = children.iter().map(|n| n.name()).collect();
                assert_eq!(inner, ["Container", "Scene", "Part"]);
                assert!(matches!(children[0], TreeNode::Document { .. }));
                match &children[2] {
                    TreeNode::Folder { children, .. } => {
                        assert_eq!(
                            children.iter().map(|n| n.name()).collect::<Vec<_>>(),
                            ["Deep"]
                        );
                    }
                    other => panic!("Part should be a folder, got {other:?}"),
                }
            }
            other => panic!("Container should be a folder, got {other:?}"),
        }
        assert_eq!(doc("Container").content.trim(), "Container text.");
        assert_eq!(doc("Deep").content.trim(), "Deep text.");
        assert!(doc("Chapter One").path.starts_with("manuscript/"));

        // Research: the Characters folder and the Research folder's note.
        let research: Vec<&str> = folder("Research").iter().map(|n| n.name()).collect();
        assert_eq!(research, ["Characters", "Note"]);
        assert!(
            doc("Hero").path.starts_with("research/"),
            "{}",
            doc("Hero").path
        );

        // Trash: kept, with its text, filed under research/ on disk.
        let trash: Vec<&str> = folder("Trash").iter().map(|n| n.name()).collect();
        assert_eq!(trash, ["Old"]);
        assert_eq!(doc("Old").content.trim(), "Old draft, binned.");
        assert!(
            doc("Old").path.starts_with("research/"),
            "{}",
            doc("Old").path
        );

        // IncludeInCompile persisted and honoured by compile.
        assert!(!doc("Private").include_in_compile);
        assert!(doc("Chapter One").include_in_compile);
        let html = temp_dir.path().join("out.html");
        chickenscratch_core::core::compile::compile(&output_path, &html, "html", None, None, None)
            .expect("compiles");
        let compiled = fs::read_to_string(&html).unwrap();
        assert!(compiled.contains("Chapter one text."));
        assert!(
            compiled.contains("Deep text."),
            "container children compile"
        );
        assert!(
            !compiled.contains("Private note"),
            "excluded document must not compile"
        );
        assert!(!compiled.contains("Old draft"), "trash must not compile");
        assert!(
            !compiled.contains("Hero sketch"),
            "research must not compile"
        );
    }

    fn write_scriv_files(scriv_path: &Path, uuid: &str, files: &[(&str, &str)]) {
        let dir = scriv_path.join("Files").join("Data").join(uuid);
        fs::create_dir_all(&dir).unwrap();
        for (name, body) in files {
            fs::write(dir.join(name), body).unwrap();
        }
    }

    /// PLAN_SCRIVENER_FIDELITY slice 3: label, status and keywords resolve
    /// through the Scrivener 3 tables (the `-1` sentinel is none, a real
    /// default label is kept), the synopsis comes from `synopsis.txt`, custom
    /// metadata lands under `custom.<title>`, notes under `notes`, and a
    /// folder with only side values gets a companion document; all of it
    /// survives reopening.
    #[test]
    fn metadata_and_notes_arrive_on_the_document() {
        if !pandoc_present() {
            eprintln!("Skipping test: Pandoc not installed");
            return;
        }
        let temp_dir = TempDir::new().unwrap();
        let scriv_path = temp_dir.path().join("Meta.scriv");
        let output_path = temp_dir.path().join("Meta.chikn");
        let a = "C1B2C3D4-0000-4000-8000-000000000001";
        let b = "C1B2C3D4-0000-4000-8000-000000000002";
        let part = "C1B2C3D4-0000-4000-8000-000000000003";
        let c = "C1B2C3D4-0000-4000-8000-000000000004";
        write_scriv_files(
            &scriv_path,
            a,
            &[
                ("content.rtf", "{\\rtf1\\ansi Scene text.\\\n}"),
                ("synopsis.txt", "Alice meets the dragon.\n"),
                (
                    "notes.rtf",
                    "{\\rtf1\\ansi Remember the egg.\\\nSecond note.\\\n}",
                ),
            ],
        );
        write_scriv_files(
            &scriv_path,
            b,
            &[("content.rtf", "{\\rtf1\\ansi Plain scene.}")],
        );
        // The folder has notes but no text of its own.
        write_scriv_files(
            &scriv_path,
            part,
            &[("notes.rtf", "{\\rtf1\\ansi Part notes.}")],
        );
        // The Research root has notes; an attachment has a synopsis and notes.
        let research_root = "C1B2C3D4-0000-4000-8000-000000000005";
        let scan = "C1B2C3D4-0000-4000-8000-000000000006";
        write_scriv_files(
            &scriv_path,
            research_root,
            &[("notes.rtf", "{\\rtf1\\ansi Root notes.}")],
        );
        write_scriv_files(
            &scriv_path,
            scan,
            &[
                ("synopsis.txt", "A scanned letter.\n"),
                ("notes.rtf", "{\\rtf1\\ansi Scan notes.}"),
            ],
        );
        fs::write(
            scriv_path
                .join("Files")
                .join("Data")
                .join(scan)
                .join("content.pdf"),
            b"%PDF sample",
        )
        .unwrap();
        write_scriv_files(
            &scriv_path,
            c,
            &[("content.rtf", "{\\rtf1\\ansi Bare scene.}")],
        );
        fs::write(
            scriv_path.join("Meta.scrivx"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<ScrivenerProject Version="2.0">
    <Binder>
        <BinderItem UUID="00000000-0000-4000-8000-00000000000D" Type="DraftFolder">
            <Title>Draft</Title>
            <Children>
                <BinderItem UUID="{a}" Type="Text">
                    <Title>Alice</Title>
                    <MetaData>
                        <LabelID>7</LabelID>
                        <StatusID>2</StatusID>
                        <Keywords><KeywordID>1</KeywordID><KeywordID>2</KeywordID></Keywords>
                        <CustomMetaData>
                            <MetaDataItem><FieldID>pov</FieldID><Value>Alice</Value></MetaDataItem>
                            <MetaDataItem><FieldID>mood</FieldID><Value>Dark</Value></MetaDataItem>
                        </CustomMetaData>
                    </MetaData>
                </BinderItem>
                <BinderItem UUID="{b}" Type="Text">
                    <Title>Bob</Title>
                    <MetaData><LabelID>-1</LabelID><StatusID>-1</StatusID></MetaData>
                </BinderItem>
                <BinderItem UUID="{part}" Type="Folder">
                    <Title>Part</Title>
                    <MetaData><LabelID>7</LabelID></MetaData>
                    <Children>
                        <BinderItem UUID="{c}" Type="Text"><Title>Carol</Title></BinderItem>
                    </Children>
                </BinderItem>
            </Children>
        </BinderItem>
        <BinderItem UUID="{research_root}" Type="ResearchFolder">
            <Title>Research</Title>
            <MetaData><LabelID>7</LabelID></MetaData>
            <Children>
                <BinderItem UUID="{scan}" Type="PDF">
                    <Title>Scan</Title>
                    <MetaData><LabelID>7</LabelID><FileExtension>pdf</FileExtension></MetaData>
                </BinderItem>
            </Children>
        </BinderItem>
    </Binder>
    <LabelSettings>
        <Title>Label</Title>
        <DefaultLabelID>7</DefaultLabelID>
        <Labels>
            <Label ID="-1">No Label</Label>
            <Label ID="7" Color="0.99 0.70 0.73">Red</Label>
        </Labels>
    </LabelSettings>
    <StatusSettings>
        <Title>Status</Title>
        <DefaultStatusID>-1</DefaultStatusID>
        <StatusItems>
            <Status ID="-1">No Status</Status>
            <Status ID="2">First Draft</Status>
        </StatusItems>
    </StatusSettings>
    <Keywords>
        <Keyword ID="1" Color="0.1 0.2 0.3">
            <Title>Fantasy</Title>
            <Children>
                <Keyword ID="2"><Title>Dragons</Title></Keyword>
            </Children>
        </Keyword>
    </Keywords>
    <CustomMetaDataSettings>
        <MetaDataField ID="pov" Type="Text"><Title>POV</Title></MetaDataField>
        <MetaDataField ID="mood" Type="Text"><Title>Mood</Title></MetaDataField>
    </CustomMetaDataSettings>
</ScrivenerProject>"#
            ),
        )
        .unwrap();

        let report = import_scriv_with_report(&scriv_path, &output_path, None).expect("imports");
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        let project = chickenscratch_core::core::project::reader::read_project(&output_path)
            .expect("reopens");
        let doc = |name: &str| -> &Document {
            project
                .documents
                .values()
                .find(|d| d.name == name)
                .unwrap_or_else(|| panic!("document {name} missing"))
        };
        let text = |v: Option<&serde_yaml::Value>| v.and_then(|v| v.as_str()).map(str::to_string);

        let alice = doc("Alice");
        assert_eq!(
            alice.label.as_deref(),
            Some("Red"),
            "a real default label is kept"
        );
        assert_eq!(alice.status.as_deref(), Some("First Draft"));
        assert_eq!(
            alice.keywords.as_deref(),
            Some(&["Fantasy".to_string(), "Dragons".to_string()][..])
        );
        assert_eq!(alice.synopsis.as_deref(), Some("Alice meets the dragon."));
        assert_eq!(
            text(alice.fields.get("custom.POV")).as_deref(),
            Some("Alice")
        );
        assert_eq!(
            text(alice.fields.get("custom.Mood")).as_deref(),
            Some("Dark")
        );
        let notes = text(alice.fields.get("notes")).expect("notes imported");
        assert!(notes.contains("Remember the egg."), "{notes}");
        assert!(notes.contains("Second note."), "{notes}");

        let bob = doc("Bob");
        assert_eq!(bob.label, None, "the -1 sentinel is no label");
        assert_eq!(bob.status, None);
        assert!(bob.fields.is_empty());

        // Review finding s3-2: the Research root and an attachment carry
        // their side values too.
        let research_root = doc("Research");
        assert_eq!(
            text(research_root.fields.get("notes")).as_deref(),
            Some("Root notes.")
        );
        assert_eq!(research_root.label.as_deref(), Some("Red"));
        // The engine lists only `.md` documents on reopen, so the attachment
        // is checked on the import result and in its sidecar on disk.
        let scan = report
            .project
            .documents
            .values()
            .find(|d| d.name == "Scan")
            .expect("attachment entry");
        let scan_meta = fs::read_to_string(output_path.join(scan.path.replace(".pdf", ".meta")))
            .expect("attachment sidecar");
        assert!(scan_meta.contains("A scanned letter."), "{scan_meta}");
        assert!(scan_meta.contains("Red"), "{scan_meta}");
        assert!(scan_meta.contains("Scan notes."), "{scan_meta}");
        assert!(scan.path.ends_with(".pdf"), "{}", scan.path);
        assert_eq!(scan.synopsis.as_deref(), Some("A scanned letter."));
        assert_eq!(scan.label.as_deref(), Some("Red"));
        assert_eq!(
            text(scan.fields.get("notes")).as_deref(),
            Some("Scan notes.")
        );

        // The folder's companion document carries its notes and label.
        let part = doc("Part");
        assert_eq!(part.content.trim(), "");
        assert_eq!(part.label.as_deref(), Some("Red"));
        assert_eq!(
            text(part.fields.get("notes")).as_deref(),
            Some("Part notes.")
        );
        let part_folder = project
            .hierarchy
            .iter()
            .find_map(|n| match n {
                TreeNode::Folder { children, name, .. } if name == "Manuscript" => {
                    children.iter().find(|c| c.name() == "Part")
                }
                _ => None,
            })
            .expect("Part folder under Manuscript");
        match part_folder {
            TreeNode::Folder { children, .. } => {
                assert_eq!(
                    children.iter().map(|c| c.name()).collect::<Vec<_>>(),
                    ["Part", "Carol"]
                );
            }
            other => panic!("Part should be a folder: {other:?}"),
        }
        assert!(doc("Carol").fields.is_empty());
    }

    /// PLAN_SCRIVENER_FIDELITY slice 4: Scrivener 3 comments and footnotes
    /// (a `content.comments` side file plus `scrivcmt://` anchors in the text)
    /// and the inline annotation/footnote forms become the editor's own
    /// comment spans and footnote elements; bodies with no anchor are kept
    /// and warned; an anchor with no body is unwrapped and warned; the
    /// fallback path gives the same anchors; compile carries the footnotes.
    #[test]
    fn comments_and_footnotes_become_the_editors_own() {
        if !pandoc_present() {
            eprintln!("Skipping test: Pandoc not installed");
            return;
        }
        let temp_dir = TempDir::new().unwrap();
        let scriv_path = temp_dir.path().join("Notes.scriv");
        let output_path = temp_dir.path().join("Notes.chikn");
        let a = "E1B2C3D4-0000-4000-8000-000000000001";
        let b = "E1B2C3D4-0000-4000-8000-000000000002";
        let field = |id: &str, text: &str| {
            format!(
                "{{\\field{{\\*\\fldinst{{HYPERLINK \"scrivcmt://{id}\"}}}}{{\\fldrslt {text}}}}}"
            )
        };
        let body_a = format!(
            "{{\\rtf1\\ansi Alpha {} {} delta {} word \\{{\\\\Scrv_annot \\\\color=\\{{\\\\R=0\\\\G=0\\\\B=1\\}} \\\\text=inline note\\\\end_Scrv_annot\\}} tail\\{{\\\\Scrv_fn=inline foot\\\\end_Scrv_fn\\}} end.\\\n}}",
            field("CMT-1", "beta"),
            field("FN-1", "gamma"),
            field("MISSING-9", "orphan")
        );
        // The same anchors in a truncated file, so the fallback path runs.
        let body_b = format!(
            "{{\\rtf1\\ansi Bravo {} {} rest",
            field("CMT-1", "beta"),
            field("FN-1", "gamma")
        );
        let comments_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Comments>
    <Comment ID="CMT-1" Color="0.99 0.99 0.70"><![CDATA[{\rtf1\ansi Tighten this.}]]></Comment>
    <Comment ID="FN-1" Footnote="Yes"><![CDATA[{\rtf1\ansi See p. 12] and on.}]]></Comment>
    <Comment ID="EXTRA-5">Never anchored.</Comment>
</Comments>"#;
        write_scriv_files(
            &scriv_path,
            a,
            &[
                ("content.rtf", body_a.as_str()),
                ("content.comments", comments_xml),
            ],
        );
        write_scriv_files(
            &scriv_path,
            b,
            &[
                ("content.rtf", body_b.as_str()),
                ("content.comments", comments_xml),
            ],
        );
        fs::write(
            scriv_path.join("Notes.scrivx"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<ScrivenerProject Version="2.0">
    <Binder>
        <BinderItem UUID="00000000-0000-4000-8000-00000000000D" Type="DraftFolder">
            <Title>Draft</Title>
            <Children>
                <BinderItem UUID="{a}" Type="Text"><Title>Alpha</Title></BinderItem>
                <BinderItem UUID="{b}" Type="Text"><Title>Bravo</Title></BinderItem>
            </Children>
        </BinderItem>
    </Binder>
</ScrivenerProject>"#
            ),
        )
        .unwrap();

        let report = import_scriv_with_report(&scriv_path, &output_path, None).expect("imports");
        let project = chickenscratch_core::core::project::reader::read_project(&output_path)
            .expect("reopens");
        let doc = |name: &str| -> &Document {
            project
                .documents
                .values()
                .find(|d| d.name == name)
                .unwrap_or_else(|| panic!("document {name} missing"))
        };

        let alpha = doc("Alpha");
        let c = &alpha.content;
        assert!(
            c.contains("<span class=\"comment\" data-comment-id=\"CMT-1\">beta</span>"),
            "{c}"
        );
        assert!(
            c.contains("gamma<sup class=\"footnote\" data-body=\"See p. 12] and on.\">●</sup>"),
            "{c}"
        );
        assert!(
            c.contains("delta orphan <span"),
            "orphan anchor unwrapped: {c}"
        );
        assert!(
            c.contains("<span class=\"comment\" data-comment-id=\"inline-1\">word</span>"),
            "inline annotation anchored on the word before it: {c}"
        );
        assert!(
            c.contains("tail<sup class=\"footnote\" data-body=\"inline foot\">●</sup>"),
            "inline footnote: {c}"
        );
        assert!(
            !c.contains("scrivcmt://") && !c.contains("scrivfn://") && !c.contains("scrivannot://"),
            "{c}"
        );
        let body_of = |id: &str| {
            alpha
                .comments
                .iter()
                .find(|cm| cm.id == id)
                .map(|cm| cm.body.clone())
                .unwrap_or_else(|| panic!("comment {id} missing: {:?}", alpha.comments))
        };
        assert_eq!(body_of("CMT-1"), "Tighten this.");
        assert_eq!(body_of("inline-1"), "inline note");
        assert_eq!(
            body_of("EXTRA-5"),
            "Comment with no place in the text: Never anchored."
        );
        assert_eq!(alpha.comments.len(), 3, "{:?}", alpha.comments);

        // The fallback path (truncated file) yields the same anchors.
        let bravo = doc("Bravo");
        assert!(
            bravo
                .content
                .contains("<span class=\"comment\" data-comment-id=\"CMT-1\">beta</span>"),
            "{}",
            bravo.content
        );
        assert!(
            bravo.content.contains("<sup class=\"footnote\""),
            "{}",
            bravo.content
        );

        // Warnings name the document for the orphan anchor, the unanchored
        // body (both documents) and the plain-text fallback (Bravo).
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.starts_with("'Alpha':") && w.contains("comment marker")),
            "{:?}",
            report.warnings
        );
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.starts_with("'Alpha':") && w.contains("no place in the text")),
            "{:?}",
            report.warnings
        );
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.starts_with("'Bravo':") && w.contains("imported plain")),
            "{:?}",
            report.warnings
        );

        // Compile carries the footnote bodies as footnotes, not as prose.
        let html = temp_dir.path().join("out.html");
        chickenscratch_core::core::compile::compile(&output_path, &html, "html", None, None, None)
            .expect("compiles");
        let compiled = fs::read_to_string(&html).unwrap();
        assert!(
            compiled.contains("See p. 12] and on."),
            "footnote body compiled: {compiled}"
        );
        assert!(compiled.contains("inline foot"), "{compiled}");
        assert!(
            compiled.contains("class=\"footnotes"),
            "a footnotes section exists: {compiled}"
        );
        assert!(
            !compiled.contains("data-body"),
            "the raw element is gone: {compiled}"
        );
    }

    /// Review finding s2-1: internal links resolve against the paths the
    /// conversion actually chose, so same-titled attachments and a folder
    /// whose first document shares a title with another document link to
    /// the right place.
    #[test]
    fn links_resolve_to_actual_paths_despite_duplicate_titles() {
        if !pandoc_present() {
            eprintln!("Skipping test: Pandoc not installed");
            return;
        }
        let temp_dir = TempDir::new().unwrap();
        let scriv_path = temp_dir.path().join("Links.scriv");
        let output_path = temp_dir.path().join("Links.chikn");
        let linker = "F1B2C3D4-0000-4000-8000-000000000001";
        let scan_root = "F1B2C3D4-0000-4000-8000-000000000002";
        let scan_research = "F1B2C3D4-0000-4000-8000-000000000003";
        let hero_char = "F1B2C3D4-0000-4000-8000-000000000004";
        let notes_folder = "F1B2C3D4-0000-4000-8000-000000000005";
        let hero_notes = "F1B2C3D4-0000-4000-8000-000000000006";
        let link = |uuid: &str, text: &str| {
            format!("{{\\field{{\\*\\fldinst{{HYPERLINK \"scrivlnk://{uuid}\"}}}}{{\\fldrslt {text}}}}}")
        };
        let body = format!(
            "{{\\rtf1\\ansi See {} and {} and {}.\\\n}}",
            link(scan_root, "root scan"),
            link(scan_research, "research scan"),
            link(notes_folder, "the notes")
        );
        write_scriv_files(&scriv_path, linker, &[("content.rtf", body.as_str())]);
        for uuid in [scan_root, scan_research] {
            let dir = scriv_path.join("Files").join("Data").join(uuid);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("content.pdf"), b"%PDF sample").unwrap();
        }
        write_scriv_files(
            &scriv_path,
            hero_char,
            &[("content.rtf", "{\\rtf1\\ansi Hero sketch.}")],
        );
        write_scriv_files(
            &scriv_path,
            hero_notes,
            &[("content.rtf", "{\\rtf1\\ansi Hero notes.}")],
        );
        fs::write(
            scriv_path.join("Links.scrivx"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<ScrivenerProject Version="2.0">
    <Binder>
        <BinderItem UUID="00000000-0000-4000-8000-00000000000D" Type="DraftFolder">
            <Title>Draft</Title>
            <Children>
                <BinderItem UUID="{linker}" Type="Text"><Title>Linker</Title></BinderItem>
            </Children>
        </BinderItem>
        <BinderItem UUID="{scan_root}" Type="PDF">
            <Title>Scan</Title>
            <MetaData><FileExtension>pdf</FileExtension></MetaData>
        </BinderItem>
        <BinderItem UUID="00000000-0000-4000-8000-00000000000C" Type="Folder">
            <Title>Characters</Title>
            <Children>
                <BinderItem UUID="{hero_char}" Type="Text"><Title>Hero</Title></BinderItem>
            </Children>
        </BinderItem>
        <BinderItem UUID="00000000-0000-4000-8000-00000000000E" Type="ResearchFolder">
            <Title>Research</Title>
            <Children>
                <BinderItem UUID="{scan_research}" Type="PDF">
                    <Title>Scan</Title>
                    <MetaData><FileExtension>pdf</FileExtension></MetaData>
                </BinderItem>
                <BinderItem UUID="{notes_folder}" Type="Folder">
                    <Title>Notes</Title>
                    <Children>
                        <BinderItem UUID="{hero_notes}" Type="Text"><Title>Hero</Title></BinderItem>
                    </Children>
                </BinderItem>
            </Children>
        </BinderItem>
    </Binder>
</ScrivenerProject>"#
            ),
        )
        .unwrap();

        let report = import_scriv_with_report(&scriv_path, &output_path, None).expect("imports");
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        let docs = &report.project.documents;
        let path_of = |name: &str, ext: &str| -> Vec<String> {
            let mut v: Vec<String> = docs
                .values()
                .filter(|d| d.name == name && d.path.ends_with(ext))
                .map(|d| d.path.clone())
                .collect();
            v.sort();
            v
        };
        let scans = path_of("Scan", ".pdf");
        assert_eq!(scans.len(), 2, "{scans:?}");
        let heroes = path_of("Hero", ".md");
        assert_eq!(heroes.len(), 2, "{heroes:?}");
        let hero_notes_doc = docs
            .values()
            .find(|d| d.name == "Hero" && d.content.contains("Hero notes"))
            .unwrap();
        let linker_doc = docs.values().find(|d| d.name == "Linker").unwrap();
        let c = &linker_doc.content;
        // Each link names a distinct, existing file.
        for scan in &scans {
            assert!(
                c.contains(&format!("]({scan})")),
                "link to {scan} missing: {c}"
            );
        }
        assert!(
            c.contains(&format!("[the notes]({})", hero_notes_doc.path)),
            "the folder link must reach its own first document: {c}"
        );
        for path in scans.iter().chain(std::iter::once(&hero_notes_doc.path)) {
            assert!(output_path.join(path).exists(), "{path} exists on disk");
        }
    }

    fn bare_item(title: &str) -> BinderItem {
        BinderItem {
            uuid: "A1A1A1A1-0000-4000-8000-000000000001".to_string(),
            item_type: "Text".to_string(),
            created: None,
            modified: None,
            title: Some(title.to_string()),
            children: super::super::parser::scrivx::ChildrenContainer { items: Vec::new() },
            metadata: None,
        }
    }

    fn note(id: &str, body: &str) -> InlineNote {
        InlineNote {
            id: id.to_string(),
            body: body.to_string(),
            footnote: false,
        }
    }

    /// Review finding s4-1: a non-breaking space (two bytes) before the word an
    /// inline annotation attaches to must not panic the import.
    #[test]
    fn annotation_after_a_non_breaking_space_attaches_to_the_word() {
        let item = bare_item("Alpha");
        let mut map = HashMap::new();
        let md = "alpha\u{A0}word [\u{200B}](scrivannot://inline-1) rest";
        let text = attach_annotations(md, Vec::new(), vec![note("inline-1", "n")], &item, &mut map);
        assert_eq!(
            text.content,
            "alpha\u{A0}<span class=\"comment\" data-comment-id=\"inline-1\">word</span>  rest"
        );
        assert_eq!(text.comments.len(), 1);
    }

    /// Review finding s4-2: two inline annotations at the start of a paragraph
    /// must not make the cursor overtake the next anchor (it panicked).
    #[test]
    fn adjacent_annotations_at_a_paragraph_start_attach_without_panic() {
        let item = bare_item("Alpha");
        let mut map = HashMap::new();
        let md = "[\u{200B}](scrivannot://inline-1)[\u{200B}](scrivannot://inline-2) word more";
        let text = attach_annotations(
            md,
            Vec::new(),
            vec![note("inline-1", "a"), note("inline-2", "b")],
            &item,
            &mut map,
        );
        // The first annotation has no word before it and none before the
        // next anchor, so it marks a bullet; the second takes "word".
        assert_eq!(
            text.content,
            "<span class=\"comment\" data-comment-id=\"inline-1\">•</span> <span class=\"comment\" data-comment-id=\"inline-2\">word</span> more"
        );
        assert_eq!(text.comments.len(), 2);
    }

    /// Review finding s4-5: anchor text with an escaped bracket still attaches
    /// its footnote.
    #[test]
    fn anchor_text_with_an_escaped_bracket_attaches() {
        let item = bare_item("Alpha");
        let mut map = HashMap::new();
        let md = r"see [a\] b](scrivcmt://FN-1) here";
        let comments = vec![ScrivComment {
            id: "FN-1".to_string(),
            body: "the note".to_string(),
            footnote: true,
        }];
        let text = attach_annotations(md, comments, Vec::new(), &item, &mut map);
        assert_eq!(
            text.content,
            "see a\\] b<sup class=\"footnote\" data-body=\"the note\">●</sup> here"
        );
        assert!(text.comments.is_empty(), "{:?}", text.comments);
        assert!(map.is_empty(), "no warnings: {map:?}");
    }

    /// Review finding s3-3: a synopsis file that exists but cannot be read is
    /// reported in the warnings, not imported as "no synopsis".
    #[cfg(unix)]
    #[test]
    fn unreadable_synopsis_is_reported() {
        use std::os::unix::fs::PermissionsExt;
        if !pandoc_present() {
            eprintln!("Skipping test: Pandoc not installed");
            return;
        }
        let temp_dir = TempDir::new().unwrap();
        let scriv_path = temp_dir.path().join("Syn.scriv");
        let output_path = temp_dir.path().join("Syn.chikn");
        let a = "A7B2C3D4-0000-4000-8000-000000000001";
        write_scriv_files(
            &scriv_path,
            a,
            &[
                ("content.rtf", "{\\rtf1\\ansi Text.}"),
                ("synopsis.txt", "Secret summary."),
            ],
        );
        let synopsis = scriv_path
            .join("Files")
            .join("Data")
            .join(a)
            .join("synopsis.txt");
        fs::set_permissions(&synopsis, fs::Permissions::from_mode(0o000)).unwrap();
        if fs::read_to_string(&synopsis).is_ok() {
            eprintln!("Skipping: running with permission to read mode-000 files");
            return;
        }
        fs::write(
            scriv_path.join("Syn.scrivx"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<ScrivenerProject Version="2.0">
    <Binder>
        <BinderItem UUID="00000000-0000-4000-8000-00000000000D" Type="DraftFolder">
            <Title>Draft</Title>
            <Children>
                <BinderItem UUID="{a}" Type="Text"><Title>Guarded</Title></BinderItem>
            </Children>
        </BinderItem>
    </Binder>
</ScrivenerProject>"#
            ),
        )
        .unwrap();
        let report = import_scriv_with_report(&scriv_path, &output_path, None).expect("imports");
        fs::set_permissions(&synopsis, fs::Permissions::from_mode(0o644)).unwrap();
        let doc = report
            .project
            .documents
            .values()
            .find(|d| d.name == "Guarded")
            .unwrap();
        assert_eq!(doc.synopsis, None);
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.starts_with("'Guarded':") && w.contains("synopsis could not be read")),
            "{:?}",
            report.warnings
        );
    }

    /// Two custom fields sharing a title switch the project to ID keys plus a
    /// titles map, so neither value overwrites the other (plan-3 R1-7).
    #[test]
    fn same_titled_custom_fields_keep_both_values() {
        if !pandoc_present() {
            eprintln!("Skipping test: Pandoc not installed");
            return;
        }
        let temp_dir = TempDir::new().unwrap();
        let scriv_path = temp_dir.path().join("Dup.scriv");
        let output_path = temp_dir.path().join("Dup.chikn");
        let a = "D1B2C3D4-0000-4000-8000-000000000001";
        write_scriv_files(&scriv_path, a, &[("content.rtf", "{\\rtf1\\ansi Text.}")]);
        fs::write(
            scriv_path.join("Dup.scrivx"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<ScrivenerProject Version="2.0">
    <Binder>
        <BinderItem UUID="00000000-0000-4000-8000-00000000000D" Type="DraftFolder">
            <Title>Draft</Title>
            <Children>
                <BinderItem UUID="{a}" Type="Text">
                    <Title>Scene</Title>
                    <MetaData>
                        <CustomMetaData>
                            <MetaDataItem><FieldID>f1</FieldID><Value>one</Value></MetaDataItem>
                            <MetaDataItem><FieldID>f2</FieldID><Value>two</Value></MetaDataItem>
                        </CustomMetaData>
                    </MetaData>
                </BinderItem>
            </Children>
        </BinderItem>
    </Binder>
    <CustomMetaDataSettings>
        <MetaDataField ID="f1" Type="Text"><Title>Scene</Title></MetaDataField>
        <MetaDataField ID="f2" Type="Text"><Title>Scene</Title></MetaDataField>
    </CustomMetaDataSettings>
</ScrivenerProject>"#
            ),
        )
        .unwrap();
        let report = import_scriv_with_report(&scriv_path, &output_path, None).expect("imports");
        let scene = report
            .project
            .documents
            .values()
            .find(|d| d.name == "Scene")
            .unwrap();
        assert_eq!(
            scene.fields.get("custom.f1").and_then(|v| v.as_str()),
            Some("one")
        );
        assert_eq!(
            scene.fields.get("custom.f2").and_then(|v| v.as_str()),
            Some("two")
        );
        assert!(!scene.fields.contains_key("custom.Scene"));
        let titles = scene
            .fields
            .get("custom_field_titles")
            .and_then(|v| v.as_mapping())
            .unwrap();
        assert_eq!(titles.get("f1").and_then(|v| v.as_str()), Some("Scene"));
        assert_eq!(titles.get("f2").and_then(|v| v.as_str()), Some("Scene"));
    }

    /// The Zoe 2 sample used to abort the whole import on one hex escape
    /// followed by an apostrophe (`Zo\'eb's`).
    #[test]
    fn zoe_sample_imports() {
        let sample_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("samples/Zoe 2.scriv");
        if !sample_path.exists() || !pandoc_present() {
            eprintln!("Skipping test: Zoe 2 sample or Pandoc not available");
            return;
        }
        let temp_dir = TempDir::new().unwrap();
        let report =
            import_scriv_with_report(&sample_path, &temp_dir.path().join("Zoe.chikn"), None)
                .expect("Zoe 2 imports");
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        assert!(
            report
                .project
                .documents
                .values()
                .any(|d| d.content.contains("Zoë's first")),
            "the accented name with its apostrophe survives"
        );
    }

    /// Scrivener paragraphs (backslash-newline) stay separate paragraphs and
    /// apostrophes are written plain, not as `\'`.
    #[test]
    fn corn_opening_keeps_paragraphs_and_plain_apostrophes() {
        let sample_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("samples/Corn.scriv");
        if !sample_path.exists() || !pandoc_present() {
            eprintln!("Skipping test: Corn sample or Pandoc not available");
            return;
        }
        let temp_dir = TempDir::new().unwrap();
        let report =
            import_scriv_with_report(&sample_path, &temp_dir.path().join("Corn.chikn"), None)
                .expect("Corn imports");
        let opening = report
            .project
            .documents
            .values()
            .find(|d| d.name == "Opening")
            .expect("Opening document");
        let paragraphs = opening
            .content
            .split("\n\n")
            .filter(|p| !p.trim().is_empty())
            .count();
        assert!(
            paragraphs >= 4,
            "expected separate paragraphs, got {paragraphs}:\n{}",
            opening.content
        );
        assert!(
            !opening.content.contains("\\'"),
            "no escaped apostrophes:\n{}",
            opening.content
        );
        assert!(
            opening.content.contains("Corn Henks' daddy"),
            "{}",
            opening.content
        );
        assert!(
            !opening.content.lines().any(|l| l.ends_with('\\')),
            "no hard-break backslashes at line ends:\n{}",
            opening.content
        );
    }

    #[test]
    fn test_import_corn_scriv_sample() {
        use tempfile::TempDir;

        // Check if sample file exists (two levels up: crates/cli → crates →
        // repo root; one level used to point at a folder that does not
        // exist, so this test always skipped).
        let sample_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("samples/Corn.scriv");

        if !sample_path.exists() {
            eprintln!("Skipping test: Corn.scriv sample not found");
            return;
        }

        // Create output directory
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("Corn.chikn");

        // Import Scrivener project
        let result = import_scriv(&sample_path, &output_path, None);

        if result.is_err() {
            eprintln!("Import error: {:?}", result.err());
            // Don't fail test if Pandoc not available
            return;
        }

        let project = result.unwrap();

        // Verify basic structure
        assert!(project.name.contains("Corn")); // Filename varies
        assert!(!project.documents.is_empty());
        assert!(!project.hierarchy.is_empty());

        // Verify project was written to disk
        assert!(output_path.exists());
        assert!(output_path.join("project.yaml").exists());
        assert!(output_path.join("manuscript").exists());
    }

    #[test]
    fn test_import_rejects_absolute_uuid_before_resolving_host_path() {
        let temp_dir = TempDir::new().unwrap();
        let scriv_path = temp_dir.path().join("Bad.scriv");
        let host_path = temp_dir.path().join("host-secret");
        let output_path = temp_dir.path().join("Bad.chikn");
        fs::create_dir_all(&scriv_path).unwrap();
        fs::create_dir_all(&host_path).unwrap();
        fs::write(host_path.join("content.rtf"), "{\\rtf1 secret}").unwrap();

        fs::write(
            scriv_path.join("Bad.scrivx"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<ScrivenerProject Version="3.0">
    <Binder>
        <BinderItem UUID="{}" Type="Text">
            <Title>Escape</Title>
        </BinderItem>
    </Binder>
</ScrivenerProject>"#,
                host_path.display()
            ),
        )
        .unwrap();

        let result = import_scriv(&scriv_path, &output_path, None);

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
        // A root-level text item lands under research/ (slice 2); neither
        // folder may hold it.
        assert!(!output_path.join("manuscript/escape.md").exists());
        assert!(!output_path.join("research/escape.md").exists());
    }

    #[test]
    fn test_import_rejects_parent_dir_uuid() {
        let temp_dir = TempDir::new().unwrap();
        let scriv_path = temp_dir.path().join("Bad.scriv");
        let output_path = temp_dir.path().join("Bad.chikn");
        fs::create_dir_all(&scriv_path).unwrap();

        fs::write(
            scriv_path.join("Bad.scrivx"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<ScrivenerProject Version="3.0">
    <Binder>
        <BinderItem UUID="../../host-secret" Type="Text">
            <Title>Escape</Title>
        </BinderItem>
    </Binder>
</ScrivenerProject>"#,
        )
        .unwrap();

        let result = import_scriv(&scriv_path, &output_path, None);

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
        assert!(!output_path.join("manuscript/escape.md").exists());
        assert!(!output_path.join("research/escape.md").exists());
    }

    /// PLAN_IMPORT_MEDIA_SAFETY step 2: an attachment that cannot be copied
    /// is reported to the caller (the app shows it) instead of only being
    /// printed to a hidden console; the rest of the import still succeeds.
    #[cfg(unix)]
    #[test]
    fn test_import_reports_attachment_that_could_not_be_copied() {
        use std::os::unix::fs::PermissionsExt;
        let temp_dir = TempDir::new().unwrap();
        let scriv_path = temp_dir.path().join("Media.scriv");
        let output_path = temp_dir.path().join("Media.chikn");
        let uuid = "F8F9FDEF-FD9F-4A8C-B33D-3434A1220ADD";
        let media_dir = scriv_path.join("Files").join("Data").join(uuid);
        fs::create_dir_all(&media_dir).unwrap();
        let media = media_dir.join("content.pdf");
        fs::write(&media, b"%PDF sample").unwrap();
        fs::set_permissions(&media, fs::Permissions::from_mode(0o000)).unwrap();
        if fs::read(&media).is_ok() {
            eprintln!("Skipping: running with permission to read mode-000 files");
            return;
        }

        fs::write(
            scriv_path.join("Media.scrivx"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<ScrivenerProject Version="3.0">
    <Binder>
        <BinderItem UUID="{uuid}" Type="Image">
            <Title>Reference PDF</Title>
            <MetaData>
                <FileExtension>pdf</FileExtension>
            </MetaData>
        </BinderItem>
    </Binder>
</ScrivenerProject>"#
            ),
        )
        .unwrap();

        let report = import_scriv_with_report(&scriv_path, &output_path, None)
            .expect("one unreadable attachment must not fail the import");
        fs::set_permissions(&media, fs::Permissions::from_mode(0o644)).unwrap();

        // A root-level attachment is reference material: research/ (slice 2).
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("research/reference-pdf.pdf")),
            "the skipped attachment must be named: {:?}",
            report.warnings
        );
        // ar-1: the failed attachment must not stay listed in the project —
        // a hierarchy entry for a missing file probes Degraded, so the app
        // would present an editable project whose every save is refused.
        assert!(
            !report
                .project
                .documents
                .values()
                .any(|d| d.path == "research/reference-pdf.pdf"),
            "a failed attachment must be dropped from the project"
        );
        assert!(matches!(
            fidelity::probe_project_fidelity(&output_path).unwrap(),
            fidelity::Fidelity::Full
        ));
    }

    /// ar-1 reopen: two attachments with the same title must get distinct
    /// files, so one failing copy cannot remove the other from the project.
    #[cfg(unix)]
    #[test]
    fn test_import_same_titled_attachments_fail_independently() {
        use std::os::unix::fs::PermissionsExt;
        let temp_dir = TempDir::new().unwrap();
        let scriv_path = temp_dir.path().join("Twins.scriv");
        let output_path = temp_dir.path().join("Twins.chikn");
        let good = "F8F9FDEF-FD9F-4A8C-B33D-3434A1220AE1";
        let bad = "F8F9FDEF-FD9F-4A8C-B33D-3434A1220AE2";
        for (uuid, bytes) in [(good, b"%PDF good" as &[u8]), (bad, b"%PDF bad")] {
            let dir = scriv_path.join("Files").join("Data").join(uuid);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("content.pdf"), bytes).unwrap();
        }
        let bad_file = scriv_path.join("Files/Data").join(bad).join("content.pdf");
        fs::set_permissions(&bad_file, fs::Permissions::from_mode(0o000)).unwrap();
        if fs::read(&bad_file).is_ok() {
            eprintln!("Skipping: running with permission to read mode-000 files");
            return;
        }
        fs::write(
            scriv_path.join("Twins.scrivx"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<ScrivenerProject Version="3.0">
    <Binder>
        <BinderItem UUID="{good}" Type="PDF">
            <Title>Scan</Title>
            <MetaData><FileExtension>pdf</FileExtension></MetaData>
        </BinderItem>
        <BinderItem UUID="{bad}" Type="PDF">
            <Title>Scan</Title>
            <MetaData><FileExtension>pdf</FileExtension></MetaData>
        </BinderItem>
    </Binder>
</ScrivenerProject>"#
            ),
        )
        .unwrap();

        let report = import_scriv_with_report(&scriv_path, &output_path, None)
            .expect("one failed twin must not fail the import");
        fs::set_permissions(&bad_file, fs::Permissions::from_mode(0o644)).unwrap();

        let kept: Vec<_> = report
            .project
            .documents
            .values()
            .filter(|d| d.name == "Scan")
            .collect();
        assert_eq!(kept.len(), 1, "exactly the readable twin stays listed");
        assert_eq!(
            fs::read(output_path.join(&kept[0].path)).unwrap(),
            b"%PDF good"
        );
        assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
        assert!(matches!(
            fidelity::probe_project_fidelity(&output_path).unwrap(),
            fidelity::Fidelity::Full
        ));
    }

    /// ar-2: an attachment the binder lists but the package lacks must be
    /// reported, not just printed to a hidden console.
    #[test]
    fn test_import_reports_attachment_missing_from_scrivener_package() {
        let temp_dir = TempDir::new().unwrap();
        let scriv_path = temp_dir.path().join("Missing.scriv");
        let output_path = temp_dir.path().join("Missing.chikn");
        fs::create_dir_all(&scriv_path).unwrap();
        fs::write(
            scriv_path.join("Missing.scrivx"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<ScrivenerProject Version="3.0">
    <Binder>
        <BinderItem UUID="F8F9FDEF-FD9F-4A8C-B33D-3434A1220ADE" Type="Image">
            <Title>Lost Map</Title>
            <MetaData>
                <FileExtension>png</FileExtension>
            </MetaData>
        </BinderItem>
    </Binder>
</ScrivenerProject>"#,
        )
        .unwrap();

        let report = import_scriv_with_report(&scriv_path, &output_path, None)
            .expect("a missing attachment must not fail the import");
        assert!(
            report.warnings.iter().any(|w| w.contains("Lost Map")),
            "the missing attachment must be named: {:?}",
            report.warnings
        );
    }

    #[test]
    fn test_import_copies_media_with_valid_common_extension() {
        let temp_dir = TempDir::new().unwrap();
        let scriv_path = temp_dir.path().join("Media.scriv");
        let output_path = temp_dir.path().join("Media.chikn");
        let uuid = "F8F9FDEF-FD9F-4A8C-B33D-3434A1220ADC";
        let media_dir = scriv_path.join("Files").join("Data").join(uuid);
        fs::create_dir_all(&media_dir).unwrap();
        fs::write(media_dir.join("content.pdf"), b"%PDF sample").unwrap();

        fs::write(
            scriv_path.join("Media.scrivx"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<ScrivenerProject Version="3.0">
    <Binder>
        <BinderItem UUID="{uuid}" Type="Image">
            <Title>Reference PDF</Title>
            <MetaData>
                <FileExtension>pdf</FileExtension>
            </MetaData>
        </BinderItem>
    </Binder>
</ScrivenerProject>"#
            ),
        )
        .unwrap();

        let result = import_scriv(&scriv_path, &output_path, None);

        assert!(result.is_ok(), "import failed: {result:?}");
        // A root-level attachment is reference material: research/ (slice 2).
        assert_eq!(
            fs::read(output_path.join("research/reference-pdf.pdf")).unwrap(),
            b"%PDF sample"
        );
    }

    #[test]
    fn test_import_rejects_malicious_media_extension_before_project_write() {
        let temp_dir = TempDir::new().unwrap();
        let scriv_path = temp_dir.path().join("BadMedia.scriv");
        let output_path = temp_dir.path().join("BadMedia.chikn");
        let uuid = "F8F9FDEF-FD9F-4A8C-B33D-3434A1220ADC";
        fs::create_dir_all(&scriv_path).unwrap();

        fs::write(
            scriv_path.join("BadMedia.scrivx"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<ScrivenerProject Version="3.0">
    <Binder>
        <BinderItem UUID="{uuid}" Type="Image">
            <Title>Escape</Title>
            <MetaData>
                <FileExtension>../md</FileExtension>
            </MetaData>
        </BinderItem>
    </Binder>
</ScrivenerProject>"#
            ),
        )
        .unwrap();

        let result = import_scriv(&scriv_path, &output_path, None);

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
        assert!(!output_path.exists());
    }
}
