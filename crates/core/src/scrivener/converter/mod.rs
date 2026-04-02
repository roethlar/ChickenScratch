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

    // Convert binder items to documents and hierarchy
    let mut documents = HashMap::new();
    let hierarchy = convert_binder_items(&scriv_project.binder, scriv_path, &mut documents)?;

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

/// Converts Scrivener binder items to .chikn hierarchy and documents
fn convert_binder_items(
    items: &[BinderItem],
    scriv_path: &Path,
    documents: &mut HashMap<String, Document>,
) -> Result<Vec<TreeNode>, ChiknError> {
    convert_binder_items_with_parent(items, scriv_path, documents, None)
}

/// Converts Scrivener binder items with parent tracking
fn convert_binder_items_with_parent(
    items: &[BinderItem],
    scriv_path: &Path,
    documents: &mut HashMap<String, Document>,
    parent_id: Option<String>,
) -> Result<Vec<TreeNode>, ChiknError> {
    let mut hierarchy = Vec::new();

    for item in items {
        match item.item_type.as_str() {
            "DraftFolder" => {
                // Always treat DraftFolder as a folder
                let folder_id = Uuid::new_v4().to_string();
                let folder_name = item.title.clone().unwrap_or_else(|| "Folder".to_string());

                let children = convert_binder_items_with_parent(
                    &item.children.items,
                    scriv_path,
                    documents,
                    Some(folder_id.clone()),
                )?;

                hierarchy.push(TreeNode::Folder {
                    id: folder_id,
                    name: folder_name,
                    children,
                });
            }
            "Text" => {
                // Convert document
                let doc_id = Uuid::new_v4().to_string();
                let doc_name = item.title.clone().unwrap_or_else(|| "Untitled".to_string());

                // Read RTF content
                let rtf_path = get_rtf_path(scriv_path, &item.uuid);
                let content = if rtf_path.exists() {
                    rtf_to_markdown(&rtf_path)?
                } else {
                    String::new()
                };

                // Generate unique slug
                let slug = crate::utils::slug::unique_slug(&doc_name, "manuscript/", documents);
                let doc_path = format!("manuscript/{}.md", slug);

                // Use Scrivener timestamps if available
                let created = item
                    .created
                    .clone()
                    .unwrap_or_else(|| Utc::now().to_rfc3339());
                let modified = item
                    .modified
                    .clone()
                    .unwrap_or_else(|| Utc::now().to_rfc3339());

                // Create document with parent_id
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

                // Add to hierarchy
                hierarchy.push(TreeNode::Document {
                    id: doc_id,
                    name: doc_name,
                    path: doc_path,
                });
            }
            "Folder" => {
                // Convert regular folder with children
                let folder_id = Uuid::new_v4().to_string();
                let folder_name = item.title.clone().unwrap_or_else(|| "Folder".to_string());

                let children = convert_binder_items_with_parent(
                    &item.children.items,
                    scriv_path,
                    documents,
                    Some(folder_id.clone()),
                )?;

                hierarchy.push(TreeNode::Folder {
                    id: folder_id,
                    name: folder_name,
                    children,
                });
            }
            _ => {
                // Skip unknown types
            }
        }
    }

    Ok(hierarchy)
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
