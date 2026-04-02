//! # Chikn to Scrivener Exporter
//!
//! Exports .chikn projects to .scriv format.
//!
//! ## Responsibilities
//! - Generate .scrivx XML from .chikn hierarchy
//! - Convert Markdown documents to RTF
//! - Create Scrivener directory structure
//! - Export metadata to .scrivx format
//!
//! ## Export Process
//! 1. Create .scriv directory structure
//! 2. Convert Markdown → RTF for all documents
//! 3. Generate .scrivx XML with hierarchy
//! 4. Write metadata and settings files

use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use uuid::Uuid;
// XML serialization done manually for better control

use super::parser::{markdown_to_rtf, BinderItem, BinderMetadata, ScrivenerProject};
use crate::models::{Project, TreeNode};
use crate::utils::error::ChiknError;

/// Exports a .chikn project to Scrivener .scriv format.
///
/// # Arguments
/// * `project` - .chikn project to export
/// * `output_path` - Path where .scriv directory will be created
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(ChiknError)` on export failure
///
/// # Example
/// ```rust
/// export_to_scriv(&project, Path::new("MyNovel.scriv"))?;
/// ```
pub fn export_to_scriv(project: &Project, output_path: &Path) -> Result<(), ChiknError> {
    // Create .scriv directory structure
    create_scriv_structure(output_path)?;

    // Map .chikn hierarchy to Scrivener binder items
    let mut uuid_map = HashMap::new();
    let binder_items = convert_to_binder_items(&project.hierarchy, project, &mut uuid_map)?;

    // Convert documents to RTF and write them
    write_rtf_documents(project, output_path, &uuid_map)?;

    // Generate .scrivx XML
    write_scrivx(output_path, &project.name, &binder_items)?;

    Ok(())
}

/// Creates Scrivener directory structure
fn create_scriv_structure(scriv_path: &Path) -> Result<(), ChiknError> {
    fs::create_dir_all(scriv_path)?;
    fs::create_dir_all(scriv_path.join("Files/Data"))?;
    fs::create_dir_all(scriv_path.join("Settings"))?;
    fs::create_dir_all(scriv_path.join("QuickLook"))?;

    // Create version.txt
    fs::write(scriv_path.join("Files/version.txt"), "3")?;

    Ok(())
}

/// Converts .chikn hierarchy to Scrivener BinderItems
fn convert_to_binder_items(
    hierarchy: &[TreeNode],
    project: &Project,
    uuid_map: &mut HashMap<String, String>,
) -> Result<Vec<BinderItem>, ChiknError> {
    let mut items = Vec::new();

    for node in hierarchy {
        match node {
            TreeNode::Document { id, name, .. } => {
                // Generate Scrivener UUID
                let scriv_uuid = Uuid::new_v4().to_string().to_uppercase();
                uuid_map.insert(id.clone(), scriv_uuid.clone());

                // Get document from project
                let doc = project
                    .documents
                    .get(id)
                    .ok_or_else(|| ChiknError::NotFound(format!("Document not found: {}", id)))?;

                let item = BinderItem {
                    uuid: scriv_uuid,
                    item_type: "Text".to_string(),
                    created: Some(doc.created.clone()),
                    modified: Some(doc.modified.clone()),
                    title: Some(name.clone()),
                    children: super::parser::scrivx::ChildrenContainer { items: Vec::new() },
                    metadata: Some(BinderMetadata {
                        label: None,
                        status: None,
                        keywords: None,
                        synopsis: None,
                        section_type: None,
                        include_in_compile: Some("Yes".to_string()),
                        icon_file_name: None,
                        file_extension: None,
                    }),
                };

                items.push(item);
            }
            TreeNode::Folder { id, name, children } => {
                let scriv_uuid = Uuid::new_v4().to_string().to_uppercase();
                uuid_map.insert(id.clone(), scriv_uuid.clone());

                let child_items = convert_to_binder_items(children, project, uuid_map)?;

                let item = BinderItem {
                    uuid: scriv_uuid,
                    item_type: "Folder".to_string(),
                    created: Some(Utc::now().to_rfc3339()),
                    modified: Some(Utc::now().to_rfc3339()),
                    title: Some(name.clone()),
                    children: super::parser::scrivx::ChildrenContainer { items: child_items },
                    metadata: None,
                };

                items.push(item);
            }
        }
    }

    Ok(items)
}

/// Writes RTF files for all documents
fn write_rtf_documents(
    project: &Project,
    scriv_path: &Path,
    uuid_map: &HashMap<String, String>,
) -> Result<(), ChiknError> {
    for (doc_id, document) in &project.documents {
        if let Some(scriv_uuid) = uuid_map.get(doc_id) {
            // Create UUID directory
            let uuid_dir = scriv_path.join("Files/Data").join(scriv_uuid);
            fs::create_dir_all(&uuid_dir)?;

            // Write markdown to temp file
            let temp_dir = std::env::temp_dir();
            let temp_md = temp_dir.join(format!("temp_{}.md", Uuid::new_v4()));
            let rtf_path = uuid_dir.join("content.rtf");

            fs::write(&temp_md, &document.content)?;

            // Convert to RTF and ensure cleanup
            let result = markdown_to_rtf(&temp_md, &rtf_path);

            // Always clean up temp file
            let _ = fs::remove_file(&temp_md);

            // Propagate error after cleanup
            result?;
        }
    }

    Ok(())
}

/// Writes .scrivx XML file
fn write_scrivx(
    scriv_path: &Path,
    project_name: &str,
    binder_items: &[BinderItem],
) -> Result<(), ChiknError> {
    let _scriv_project = ScrivenerProject {
        name: project_name.to_string(),
        version: "3.0".to_string(),
        binder: binder_items.to_vec(),
    };

    // Generate XML (simplified - real .scrivx has more fields)
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ScrivenerProject Version="3.0" Identifier="{}">
    <Binder>
{}
    </Binder>
</ScrivenerProject>"#,
        Uuid::new_v4().to_string().to_uppercase(),
        generate_binder_xml(binder_items, 2)
    );

    let scrivx_path = scriv_path.join(format!("{}.scrivx", project_name));
    fs::write(&scrivx_path, xml)?;

    Ok(())
}

/// Generates XML for binder items (recursive)
fn generate_binder_xml(items: &[BinderItem], indent_level: usize) -> String {
    let indent = "    ".repeat(indent_level);
    let mut xml = String::new();

    for item in items {
        let title = item
            .title
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("Untitled");
        let created = item.created.as_ref().map(|s| s.as_str()).unwrap_or("");
        let modified = item.modified.as_ref().map(|s| s.as_str()).unwrap_or("");

        xml.push_str(&format!(
            r#"{}<BinderItem UUID="{}" Type="{}" Created="{}" Modified="{}">
"#,
            indent, item.uuid, item.item_type, created, modified
        ));

        xml.push_str(&format!(
            r#"{}    <Title>{}</Title>
"#,
            indent,
            escape_xml(title)
        ));

        // Add metadata if present
        if let Some(ref metadata) = item.metadata {
            xml.push_str(&format!(
                r#"{}    <MetaData>
"#,
                indent
            ));
            if let Some(ref include) = metadata.include_in_compile {
                xml.push_str(&format!(
                    r#"{}        <IncludeInCompile>{}</IncludeInCompile>
"#,
                    indent, include
                ));
            }
            xml.push_str(&format!(
                r#"{}    </MetaData>
"#,
                indent
            ));
        }

        // Add children if present
        if !item.children.items.is_empty() {
            xml.push_str(&format!(
                r#"{}    <Children>
"#,
                indent
            ));
            xml.push_str(&generate_binder_xml(&item.children.items, indent_level + 2));
            xml.push_str(&format!(
                r#"{}    </Children>
"#,
                indent
            ));
        }

        xml.push_str(&format!(
            r#"{}</BinderItem>
"#,
            indent
        ));
    }

    xml
}

/// Escapes XML special characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("Shelly & Marcus"), "Shelly &amp; Marcus");
        assert_eq!(
            escape_xml("Chapter 1: The <Beginning>"),
            "Chapter 1: The &lt;Beginning&gt;"
        );
    }

    #[test]
    fn test_create_scriv_structure() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let scriv_path = temp_dir.path().join("Test.scriv");

        let result = create_scriv_structure(&scriv_path);
        assert!(result.is_ok());

        assert!(scriv_path.join("Files/Data").exists());
        assert!(scriv_path.join("Settings").exists());
        assert!(scriv_path.join("Files/version.txt").exists());
    }
}
