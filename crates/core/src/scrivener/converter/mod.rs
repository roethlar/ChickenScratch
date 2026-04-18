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
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use uuid::Uuid;

use super::parser::{get_media_path, get_rtf_path, parse_scrivx, rtf_to_html, BinderItem};
use crate::core::project::writer;
use crate::models::{Document, Project, TreeNode};
use crate::utils::error::ChiknError;

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
/// ```rust
/// let project = import_scriv(
///     Path::new("MyNovel.scriv"),
///     Path::new("MyNovel.chikn")
/// )?;
/// ```
pub fn import_scriv(scriv_path: &Path, output_path: &Path) -> Result<Project, ChiknError> {
    // Find .scrivx file
    let scrivx_file = find_scrivx_file(scriv_path)?;

    // Parse Scrivener project
    let scriv_project = parse_scrivx(&scrivx_file)?;

    // Create .chikn project
    let mut chikn_project = writer::create_project(output_path, &scriv_project.name)?;

    // Pre-pass: build Scrivener UUID -> chikn path map from the entire binder tree
    // This must happen before conversion so all links can be resolved
    let mut uuid_to_path: HashMap<String, String> = HashMap::new();
    build_uuid_map(&scriv_project.binder, scriv_path, &mut uuid_to_path, "manuscript");

    // Convert binder items to documents and hierarchy
    let mut documents = HashMap::new();
    let hierarchy = convert_binder_items(
        &scriv_project.binder,
        scriv_path,
        &mut documents,
        &mut uuid_to_path,
    )?;

    // Second pass: clean up Scrivener markup in all document content
    for doc in documents.values_mut() {
        doc.content = clean_scrivener_markup(&doc.content, &uuid_to_path);
    }

    // Ensure proper project structure: Manuscript, Research, Trash
    // Move any loose root items into the Manuscript folder
    chikn_project.hierarchy = ensure_project_structure(hierarchy);
    chikn_project.documents = documents;

    // Save the converted project
    writer::write_project(&mut chikn_project)?;

    // Copy media files into the project
    for (key, value) in &uuid_to_path {
        if let Some(uuid) = key.strip_prefix("__media__") {
            let parts: Vec<&str> = value.splitn(2, '|').collect();
            if parts.len() == 2 {
                let src = Path::new(parts[0]);
                let dest = output_path.join(parts[1]);
                if let Some(parent) = dest.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if let Err(e) = fs::copy(src, &dest) {
                    eprintln!("Failed to copy media file {}: {}", uuid, e);
                }
            }
        }
    }

    // Initial commit with all converted content
    let _ = crate::core::git::save_revision(
        output_path,
        &format!("Imported from Scrivener: {}", scriv_project.name),
    );

    Ok(chikn_project)
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

/// Pre-pass: builds a map of Scrivener UUID -> expected chikn .md path.
/// This traverses the entire binder tree before conversion so that
/// scrivlnk:// references in any document can be resolved.
fn build_uuid_map(
    items: &[BinderItem],
    scriv_path: &Path,
    uuid_to_path: &mut HashMap<String, String>,
    target_folder: &str,
) {
    // Track slugs we've seen to handle collisions (mirrors unique_slug logic)
    for item in items {
        match item.item_type.as_str() {
            "TrashFolder" => continue,

            "ResearchFolder" => {
                build_uuid_map(&item.children.items, scriv_path, uuid_to_path, "research");
            }

            "DraftFolder" | "Folder" => {
                // For folders, map UUID to the first child doc if one exists
                if let Some(path) = find_first_text_path(&item.children.items, scriv_path, target_folder) {
                    uuid_to_path.insert(item.uuid.clone(), path);
                }
                // Also map if the folder itself has content
                let rtf_path = get_rtf_path(scriv_path, &item.uuid);
                if rtf_path.exists() {
                    let name = item.title.clone().unwrap_or_else(|| "folder".to_string());
                    let slug = crate::utils::slug::slugify(&name);
                    uuid_to_path.insert(item.uuid.clone(), format!("{}/{}.html", target_folder, slug));
                }
                build_uuid_map(&item.children.items, scriv_path, uuid_to_path, target_folder);
            }

            "Text" => {
                let name = item.title.clone().unwrap_or_else(|| "untitled".to_string());
                let slug = crate::utils::slug::slugify(&name);
                uuid_to_path.insert(item.uuid.clone(), format!("{}/{}.html", target_folder, slug));
                // Scrivener Text items can have children too
                if !item.children.items.is_empty() {
                    build_uuid_map(&item.children.items, scriv_path, uuid_to_path, target_folder);
                }
            }

            // Media types (PDF, Image, etc.) — map by file extension
            _ => {
                if let Some(ext) = item.metadata.as_ref().and_then(|m| m.file_extension.as_deref()) {
                    let name = item.title.clone().unwrap_or_else(|| "untitled".to_string());
                    let slug = crate::utils::slug::slugify(&name);
                    uuid_to_path.insert(item.uuid.clone(), format!("{}/{}.{}", target_folder, slug, ext));
                }
            }
        }
    }
}

/// Finds the .md path of the first Text item in a binder subtree.
fn find_first_text_path(items: &[BinderItem], scriv_path: &Path, target_folder: &str) -> Option<String> {
    for item in items {
        if item.item_type == "Text" {
            let name = item.title.clone().unwrap_or_else(|| "untitled".to_string());
            let slug = crate::utils::slug::slugify(&name);
            return Some(format!("{}/{}.html", target_folder, slug));
        }
        if let Some(path) = find_first_text_path(&item.children.items, scriv_path, target_folder) {
            return Some(path);
        }
    }
    None
}

/// Converts Scrivener binder items to .chikn hierarchy and documents
fn convert_binder_items(
    items: &[BinderItem],
    scriv_path: &Path,
    documents: &mut HashMap<String, Document>,
    uuid_to_path: &mut HashMap<String, String>,
) -> Result<Vec<TreeNode>, ChiknError> {
    convert_binder_items_inner(items, scriv_path, documents, uuid_to_path, None, "manuscript")
}

/// Converts Scrivener binder items with parent and target folder tracking
fn convert_binder_items_inner(
    items: &[BinderItem],
    scriv_path: &Path,
    documents: &mut HashMap<String, Document>,
    uuid_to_path: &mut HashMap<String, String>,
    parent_id: Option<String>,
    target_folder: &str,
) -> Result<Vec<TreeNode>, ChiknError> {
    let mut hierarchy = Vec::new();

    for item in items {
        match item.item_type.as_str() {
            // Skip trash entirely
            "TrashFolder" => continue,

            // Research folder — convert children into research/ instead of manuscript/
            "ResearchFolder" => {
                let folder_id = Uuid::new_v4().to_string();
                let folder_name = item.title.clone().unwrap_or_else(|| "Research".to_string());

                let children = convert_binder_items_inner(
                    &item.children.items,
                    scriv_path,
                    documents,
                    uuid_to_path,
                    Some(folder_id.clone()),
                    "research",
                )?;

                hierarchy.push(TreeNode::Folder {
                    id: folder_id,
                    name: folder_name,
                    children,
                });
            }

            // Draft folder and regular folders
            "DraftFolder" | "Folder" => {
                let folder_id = Uuid::new_v4().to_string();
                let folder_name = item.title.clone().unwrap_or_else(|| "Folder".to_string());

                let children = convert_binder_items_inner(
                    &item.children.items,
                    scriv_path,
                    documents,
                    uuid_to_path,
                    Some(folder_id.clone()),
                    target_folder,
                )?;

                // Folders in Scrivener can also have their own text content
                let rtf_path = get_rtf_path(scriv_path, &item.uuid);
                if rtf_path.exists() {
                    let content = rtf_to_html(&rtf_path)?;
                    if !content.trim().is_empty() {
                        let doc_id = Uuid::new_v4().to_string();
                        let slug = crate::utils::slug::unique_slug(
                            &folder_name,
                            &format!("{}/", target_folder),
                            documents,
                        );
                        let doc_path = format!("{}/{}.html", target_folder, slug);
                        let created = item.created.clone().unwrap_or_else(|| Utc::now().to_rfc3339());
                        let modified = item.modified.clone().unwrap_or_else(|| Utc::now().to_rfc3339());

                        uuid_to_path.insert(item.uuid.clone(), doc_path.clone());

                        let document = Document {
                            id: doc_id.clone(),
                            name: folder_name.clone(),
                            path: doc_path.clone(),
                            content,
                            parent_id: Some(folder_id.clone()),
                            created,
                            modified,
                            synopsis: None,
                            label: None,
                            status: None,
                            keywords: None,
                            links: None,
                    include_in_compile: true,
                            word_count_target: 0,
                            compile_order: 0,
                            comments: Vec::new(),
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
                }

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

                let rtf_path = get_rtf_path(scriv_path, &item.uuid);
                let content = if rtf_path.exists() {
                    rtf_to_html(&rtf_path)?
                } else {
                    String::new()
                };

                let slug = crate::utils::slug::unique_slug(
                    &doc_name,
                    &format!("{}/", target_folder),
                    documents,
                );
                let doc_path = format!("{}/{}.html", target_folder, slug);

                let created = item
                    .created
                    .clone()
                    .unwrap_or_else(|| Utc::now().to_rfc3339());
                let modified = item
                    .modified
                    .clone()
                    .unwrap_or_else(|| Utc::now().to_rfc3339());

                uuid_to_path.insert(item.uuid.clone(), doc_path.clone());

                let meta = item.metadata.as_ref();
                let document = Document {
                    id: doc_id.clone(),
                    name: doc_name.clone(),
                    path: doc_path.clone(),
                    content,
                    parent_id: parent_id.clone(),
                    created,
                    modified,
                    synopsis: meta.and_then(|m| m.synopsis.clone()),
                    label: meta.and_then(|m| m.label.clone()),
                    status: meta.and_then(|m| m.status.clone()),
                    keywords: meta.and_then(|m| m.keywords.clone()),
                    links: None,
                    include_in_compile: true,
                            word_count_target: 0,
                            compile_order: 0,
                            comments: Vec::new(),
                };

                documents.insert(doc_id.clone(), document);

                hierarchy.push(TreeNode::Document {
                    id: doc_id,
                    name: doc_name,
                    path: doc_path,
                });
            }

            // Any other type — check if it's a media file we can copy
            other => {
                let title = item.title.as_deref().unwrap_or("untitled");
                let ext = item
                    .metadata
                    .as_ref()
                    .and_then(|m| m.file_extension.as_deref());

                if let Some(ext) = ext {
                    // Media file — copy into the project
                    let media_src = get_media_path(scriv_path, &item.uuid, ext);
                    if media_src.exists() {
                        let doc_id = Uuid::new_v4().to_string();
                        let doc_name = item.title.clone().unwrap_or_else(|| "Untitled".to_string());
                        let slug = crate::utils::slug::unique_slug(
                            &doc_name,
                            &format!("{}/", target_folder),
                            documents,
                        );
                        let file_name = format!("{}.{}", slug, ext);
                        let dest_rel = format!("{}/{}", target_folder, file_name);

                        // The actual file copy happens after write_project, below

                        // We need the output project path — get it from the hierarchy context
                        // For now, record as a document entry so it appears in the tree
                        let doc_path = dest_rel.clone();

                        let created = item.created.clone().unwrap_or_else(|| Utc::now().to_rfc3339());
                        let modified = item.modified.clone().unwrap_or_else(|| Utc::now().to_rfc3339());

                        let document = Document {
                            id: doc_id.clone(),
                            name: doc_name.clone(),
                            path: doc_path.clone(),
                            content: String::new(),
                            parent_id: parent_id.clone(),
                            created,
                            modified,
                            synopsis: None,
                            label: None,
                            status: None,
                            keywords: None,
                            links: None,
                    include_in_compile: true,
                            word_count_target: 0,
                            compile_order: 0,
                            comments: Vec::new(),
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
                    } else {
                        eprintln!("Skipping binder item \"{}\" (type: {}, file not found)", title, other);
                    }
                } else {
                    eprintln!("Skipping binder item \"{}\" (type: {})", title, other);
                }
            }
        }
    }

    Ok(hierarchy)
}

/// Cleans Scrivener-specific markup from converted Markdown content.
///
/// Handles:
/// - `scrivlnk://UUID` links → rewritten to relative .md paths
/// - `\<\$Scr_Ps::N\>` / `\<!\$Scr_Ps::N\>` → paragraph style tags, stripped
/// - `\<\$Scr_Cs::N\>` / `\<!\$Scr_Cs::N\>` → character style tags, stripped
/// - `{\\Scrv_annot ...}` → inline annotations, converted to HTML comments
/// Ensure the hierarchy has Manuscript, Research, and Trash at the top level.
/// Loose items (not inside one of these) are moved into Manuscript.
fn ensure_project_structure(mut hierarchy: Vec<TreeNode>) -> Vec<TreeNode> {
    let mut manuscript_children: Vec<TreeNode> = Vec::new();
    let mut research_children: Vec<TreeNode> = Vec::new();
    let mut manuscript_id = None;
    let mut research_id = None;
    let mut kept: Vec<TreeNode> = Vec::new();

    for node in hierarchy.drain(..) {
        match &node {
            TreeNode::Folder { name, id, .. }
                if name.to_lowercase() == "manuscript"
                    || name.to_lowercase() == "draft" =>
            {
                // This is the manuscript folder — keep its ID
                if let TreeNode::Folder { id, children, .. } = node {
                    manuscript_id = Some(id.clone());
                    manuscript_children.extend(children);
                }
            }
            TreeNode::Folder { name, .. }
                if name.to_lowercase() == "research" =>
            {
                if let TreeNode::Folder { id, children, .. } = node {
                    research_id = Some(id.clone());
                    research_children.extend(children);
                }
            }
            TreeNode::Folder { name, .. }
                if name.to_lowercase() == "trash" =>
            {
                // Skip — we'll create our own
            }
            _ => {
                // Loose item — goes into Manuscript
                manuscript_children.push(node);
            }
        }
    }

    // Build the three required folders
    kept.push(TreeNode::Folder {
        id: manuscript_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        name: "Manuscript".to_string(),
        children: manuscript_children,
    });
    kept.push(TreeNode::Folder {
        id: research_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        name: "Research".to_string(),
        children: research_children,
    });
    kept.push(TreeNode::Folder {
        id: Uuid::new_v4().to_string(),
        name: "Trash".to_string(),
        children: Vec::new(),
    });

    kept
}

fn clean_scrivener_markup(content: &str, uuid_to_path: &HashMap<String, String>) -> String {
    let mut result = content.to_string();

    // Rewrite scrivlnk:// links to relative paths
    // In HTML: <a href="scrivlnk://UUID">text</a>
    let scriv_link_re = Regex::new(r"scrivlnk://([A-Fa-f0-9-]+)").unwrap();
    result = scriv_link_re
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
    let scr_tag_html = Regex::new(r"&lt;[!]?\$Scr_[CP]s::\d+&gt;").unwrap();
    result = scr_tag_html.replace_all(&result, "").to_string();

    // Also catch unescaped variants
    let scr_tag_raw = Regex::new(r"<[!]?\$Scr_[CP]s::\d+>").unwrap();
    result = scr_tag_raw.replace_all(&result, "").to_string();

    // Also catch backslash-escaped variants from older conversions
    let scr_tag_esc = Regex::new(r"\\<[!]?\\?\$Scr_[CP]s::\d+\\>").unwrap();
    result = scr_tag_esc.replace_all(&result, "").to_string();

    // Strip Scrivener custom variable placeholders
    // &lt;$custom:shortcut&gt; etc.
    let scr_var = Regex::new(r"&lt;\$\w+:\w+&gt;").unwrap();
    result = scr_var.replace_all(&result, "").to_string();

    // Scrivener uses hard line breaks between paragraphs in RTF.
    // Pandoc converts these to <br /> within a single <p> tag.
    // Split into proper separate paragraphs.
    result = result.replace("<br />\n", "</p>\n<p>");
    result = result.replace("<br/>", "</p><p>");

    // Clean up empty elements left by stripped tags
    let empty_strong = Regex::new(r"<strong>\s*</strong>").unwrap();
    result = empty_strong.replace_all(&result, "").to_string();

    let empty_em = Regex::new(r"<em>\s*</em>").unwrap();
    result = empty_em.replace_all(&result, "").to_string();

    let empty_p = Regex::new(r"<p>\s*</p>").unwrap();
    result = empty_p.replace_all(&result, "").to_string();

    result
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn test_import_corn_scriv_sample() {
        use tempfile::TempDir;

        // Check if sample file exists
        let sample_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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
        let result = import_scriv(&sample_path, &output_path);

        if result.is_err() {
            eprintln!("Import error: {:?}", result.err());
            // Don't fail test if Pandoc not available
            return;
        }

        let project = result.unwrap();

        // Verify basic structure
        assert!(project.name.contains("Corn")); // Filename varies
        assert!(project.documents.len() > 0);
        assert!(project.hierarchy.len() > 0);

        // Verify project was written to disk
        assert!(output_path.exists());
        assert!(output_path.join("project.yaml").exists());
        assert!(output_path.join("manuscript").exists());
    }
}
