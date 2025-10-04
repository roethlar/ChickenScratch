//! # .scrivx XML Parser
//!
//! Parses Scrivener project XML files (.scrivx).
//!
//! ## Responsibilities
//! - Parse XML structure into Rust types
//! - Extract document hierarchy (Binder)
//! - Extract metadata (labels, status, keywords)
//! - Map UUIDs to file paths
//!
//! ## Example .scrivx Structure
//! ```xml
//! <ScrivenerProject>
//!   <Binder>
//!     <BinderItem UUID="..." Type="Text">
//!       <Title>Chapter 1</Title>
//!       <MetaData>
//!         <Label>Scene</Label>
//!         <Status>First Draft</Status>
//!       </MetaData>
//!     </BinderItem>
//!   </Binder>
//! </ScrivenerProject>
//! ```

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use quick_xml::de::from_str;
use crate::utils::error::ChiknError;

/// Parsed Scrivener project structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrivenerProject {
    /// Project name
    pub name: String,

    /// Scrivener version
    pub version: String,

    /// Root binder items (document hierarchy)
    pub binder: Vec<BinderItem>,
}

/// Binder item (document or folder in Scrivener hierarchy)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "BinderItem")]
pub struct BinderItem {
    /// Unique identifier
    #[serde(rename = "@UUID")]
    pub uuid: String,

    /// Item type (Text, Folder, DraftFolder, etc.)
    #[serde(rename = "@Type")]
    pub item_type: String,

    /// Creation timestamp (optional)
    #[serde(rename = "@Created", default)]
    pub created: Option<String>,

    /// Modified timestamp (optional)
    #[serde(rename = "@Modified", default)]
    pub modified: Option<String>,

    /// Document title
    #[serde(rename = "Title", default)]
    pub title: Option<String>,

    /// Child items (for folders) - flatten the Children wrapper
    #[serde(rename = "Children", default)]
    pub children: ChildrenContainer,

    /// Metadata (labels, status, etc.)
    #[serde(rename = "MetaData", default)]
    pub metadata: Option<BinderMetadata>,
}

/// Container for child BinderItems
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChildrenContainer {
    #[serde(rename = "BinderItem", default)]
    pub items: Vec<BinderItem>,
}

/// Scrivener metadata for a binder item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinderMetadata {
    /// Label (e.g., "Scene", "Chapter")
    #[serde(rename = "Label")]
    pub label: Option<String>,

    /// Status (e.g., "First Draft", "Revised")
    #[serde(rename = "Status")]
    pub status: Option<String>,

    /// Keywords/tags
    #[serde(rename = "Keywords")]
    pub keywords: Option<Vec<String>>,

    /// Synopsis/summary
    #[serde(rename = "Synopsis")]
    pub synopsis: Option<String>,

    /// Section type UUID (Scrivener document type)
    #[serde(rename = "SectionType")]
    pub section_type: Option<String>,

    /// Include in compile flag
    #[serde(rename = "IncludeInCompile")]
    pub include_in_compile: Option<String>,

    /// Icon filename
    #[serde(rename = "IconFileName")]
    pub icon_file_name: Option<String>,
}

/// Parses a .scrivx XML file into ScrivenerProject
///
/// # Arguments
/// * `scrivx_path` - Path to .scrivx file
///
/// # Returns
/// * `Ok(ScrivenerProject)` - Parsed project structure
/// * `Err(ChiknError)` on parse failure
///
/// # Example
/// ```rust
/// let project = parse_scrivx(Path::new("MyNovel.scriv/MyNovel.scrivx"))?;
/// ```
pub fn parse_scrivx(scrivx_path: &Path) -> Result<ScrivenerProject, ChiknError> {
    // Read XML file
    let xml_content = fs::read_to_string(scrivx_path)?;

    // Parse XML
    let project: ScrivenerProjectXml = from_str(&xml_content)
        .map_err(|e| ChiknError::InvalidFormat(format!("Failed to parse .scrivx: {}", e)))?;

    // Extract name from filename
    let name = scrivx_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string();

    Ok(ScrivenerProject {
        name,
        version: project.version.unwrap_or_else(|| "3.0".to_string()),
        binder: project.binder.items,
    })
}

/// XML structure for deserialization
#[derive(Debug, Deserialize)]
#[serde(rename = "ScrivenerProject")]
struct ScrivenerProjectXml {
    #[serde(rename = "@Version")]
    version: Option<String>,

    #[serde(rename = "@Identifier")]
    identifier: Option<String>,

    #[serde(rename = "Binder")]
    binder: BinderContainer,
}

#[derive(Debug, Deserialize)]
struct BinderContainer {
    #[serde(rename = "BinderItem", default)]
    items: Vec<BinderItem>,
}

/// Gets the RTF file path for a given UUID
///
/// # Arguments
/// * `scriv_path` - Root .scriv directory
/// * `uuid` - Document UUID
///
/// # Returns
/// PathBuf to content.rtf file
pub fn get_rtf_path(scriv_path: &Path, uuid: &str) -> std::path::PathBuf {
    scriv_path
        .join("Files")
        .join("Data")
        .join(uuid)
        .join("content.rtf")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_simple_scrivx() {
        let temp_dir = TempDir::new().unwrap();
        let scriv_path = temp_dir.path().join("Test.scriv");
        fs::create_dir(&scriv_path).unwrap();

        let scrivx_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<ScrivenerProject Version="3.0">
    <Binder>
        <BinderItem UUID="test-uuid-1" Type="Text">
            <Title>Chapter 1</Title>
            <MetaData>
                <Label>Scene</Label>
                <Status>First Draft</Status>
            </MetaData>
        </BinderItem>
    </Binder>
</ScrivenerProject>"#;

        let scrivx_file = scriv_path.join("Test.scrivx");
        fs::write(&scrivx_file, scrivx_content).unwrap();

        let result = parse_scrivx(&scrivx_file);
        if let Err(ref e) = result {
            eprintln!("Parse error: {:?}", e);
        }
        assert!(result.is_ok());

        let project = result.unwrap();
        assert_eq!(project.name, "Test");
        assert_eq!(project.version, "3.0");
        assert_eq!(project.binder.len(), 1);
        assert_eq!(project.binder[0].uuid, "test-uuid-1");
        assert_eq!(project.binder[0].title, Some("Chapter 1".to_string()));
    }

    #[test]
    fn test_get_rtf_path() {
        let scriv_path = Path::new("/path/to/Project.scriv");
        let uuid = "ABC123";

        let rtf_path = get_rtf_path(scriv_path, uuid);

        assert_eq!(
            rtf_path,
            Path::new("/path/to/Project.scriv/Files/Data/ABC123/content.rtf")
        );
    }
}
