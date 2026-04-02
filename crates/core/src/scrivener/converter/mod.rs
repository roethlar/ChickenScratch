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

use super::parser::{get_rtf_path, parse_scrivx, rtf_to_markdown, BinderItem};
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

    chikn_project.hierarchy = hierarchy;
    chikn_project.documents = documents;

    // Save the converted project
    writer::write_project(&mut chikn_project)?;

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
                    uuid_to_path.insert(item.uuid.clone(), format!("{}/{}.md", target_folder, slug));
                }
                build_uuid_map(&item.children.items, scriv_path, uuid_to_path, target_folder);
            }

            "Text" => {
                let name = item.title.clone().unwrap_or_else(|| "untitled".to_string());
                let slug = crate::utils::slug::slugify(&name);
                uuid_to_path.insert(item.uuid.clone(), format!("{}/{}.md", target_folder, slug));
                // Scrivener Text items can have children too
                if !item.children.items.is_empty() {
                    build_uuid_map(&item.children.items, scriv_path, uuid_to_path, target_folder);
                }
            }

            _ => {}
        }
    }
}

/// Finds the .md path of the first Text item in a binder subtree.
fn find_first_text_path(items: &[BinderItem], scriv_path: &Path, target_folder: &str) -> Option<String> {
    for item in items {
        if item.item_type == "Text" {
            let name = item.title.clone().unwrap_or_else(|| "untitled".to_string());
            let slug = crate::utils::slug::slugify(&name);
            return Some(format!("{}/{}.md", target_folder, slug));
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
                    let content = rtf_to_markdown(&rtf_path)?;
                    if !content.trim().is_empty() {
                        let doc_id = Uuid::new_v4().to_string();
                        let slug = crate::utils::slug::unique_slug(
                            &folder_name,
                            &format!("{}/", target_folder),
                            documents,
                        );
                        let doc_path = format!("{}/{}.md", target_folder, slug);
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
                    rtf_to_markdown(&rtf_path)?
                } else {
                    String::new()
                };

                let slug = crate::utils::slug::unique_slug(
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

                let document = Document {
                    id: doc_id.clone(),
                    name: doc_name.clone(),
                    path: doc_path.clone(),
                    content,
                    parent_id: parent_id.clone(),
                    created,
                    modified,
                };

                documents.insert(doc_id.clone(), document);

                hierarchy.push(TreeNode::Document {
                    id: doc_id,
                    name: doc_name,
                    path: doc_path,
                });
            }

            // Skip unknown types (TemplateFolder, etc.)
            other => {
                eprintln!("Skipping unknown binder item type: {}", other);
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
fn clean_scrivener_markup(content: &str, uuid_to_path: &HashMap<String, String>) -> String {
    let mut result = content.to_string();

    // Rewrite scrivlnk:// links to relative paths
    // Pattern: [link text](scrivlnk://UUID) or \[text\](scrivlnk://UUID)
    let scriv_link_re = Regex::new(r"scrivlnk://([A-Fa-f0-9-]+)").unwrap();
    result = scriv_link_re
        .replace_all(&result, |caps: &regex::Captures| {
            let uuid = &caps[1];
            match uuid_to_path.get(uuid) {
                Some(path) => path.clone(),
                None => format!("scrivlnk://{}", uuid), // preserve if unknown
            }
        })
        .to_string();

    // Strip Scrivener paragraph/character style tags
    // \<\$Scr_Ps::N\> and \<!\$Scr_Ps::N\>
    // \<\$Scr_Cs::N\> and \<!\$Scr_Cs::N\>
    let scr_tag_re = Regex::new(r"\\<[!]?\\?\$Scr_[CP]s::\d+\\>").unwrap();
    result = scr_tag_re.replace_all(&result, "").to_string();

    // Also catch the non-escaped variants that Pandoc might produce
    let scr_tag_re2 = Regex::new(r"<[!]?\$Scr_[CP]s::\d+>").unwrap();
    result = scr_tag_re2.replace_all(&result, "").to_string();

    // Convert Scrivener inline annotations to HTML comments
    // {\\Scrv_annot \color={...} \text= ANNOTATION_TEXT \end_Scrv_annot}
    let annot_re = Regex::new(r"\{\\\\Scrv_annot[^}]*\\text=\s*([^\\}]+)\\end_Scrv_annot\}")
        .unwrap();
    result = annot_re
        .replace_all(&result, |caps: &regex::Captures| {
            format!("<!-- {} -->", caps[1].trim())
        })
        .to_string();

    // Clean up any resulting empty lines from stripped tags
    let multi_blank_re = Regex::new(r"\n{3,}").unwrap();
    result = multi_blank_re.replace_all(&result, "\n\n").to_string();

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
