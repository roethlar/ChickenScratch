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

use crate::utils::error::ChiknError;
use quick_xml::de::from_str;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};

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

    /// File extension for non-text items (e.g., "pdf", "png", "jpg")
    #[serde(rename = "FileExtension")]
    pub file_extension: Option<String>,
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
/// ```no_run
/// use std::path::Path;
/// use chickenscratch_core::scrivener::parser::parse_scrivx;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let project = parse_scrivx(Path::new("MyNovel.scriv/MyNovel.scrivx"))?;
/// # Ok(()) }
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
    _identifier: Option<String>,

    #[serde(rename = "Binder")]
    binder: BinderContainer,
}

#[derive(Debug, Deserialize)]
struct BinderContainer {
    #[serde(rename = "BinderItem", default)]
    items: Vec<BinderItem>,
}

/// Validates a Scrivener BinderItem UUID before using it as a path component.
pub fn validate_scrivener_uuid(uuid: &str) -> Result<(), ChiknError> {
    let is_hyphenated_uuid = uuid.len() == 36
        && uuid.char_indices().all(|(idx, ch)| match idx {
            8 | 13 | 18 | 23 => ch == '-',
            _ => ch.is_ascii_hexdigit(),
        });

    if !is_hyphenated_uuid {
        return Err(ChiknError::InvalidFormat(format!(
            "Invalid Scrivener BinderItem UUID: {uuid:?}"
        )));
    }

    let path = Path::new(uuid);
    if path.components().any(|component| {
        matches!(
            component,
            Component::Prefix(_) | Component::RootDir | Component::ParentDir
        )
    }) || path.components().count() != 1
    {
        return Err(ChiknError::InvalidFormat(format!(
            "Invalid Scrivener BinderItem UUID path component: {uuid:?}"
        )));
    }

    Ok(())
}

/// Gets the RTF file path for a given UUID
pub fn get_rtf_path(scriv_path: &Path, uuid: &str) -> Result<PathBuf, ChiknError> {
    validate_scrivener_uuid(uuid)?;

    Ok(scriv_path
        .join("Files")
        .join("Data")
        .join(uuid)
        .join("content.rtf"))
}

/// Gets the content file path for a given UUID and file extension (e.g., "pdf", "png")
pub fn get_media_path(
    scriv_path: &Path,
    uuid: &str,
    extension: &str,
) -> Result<PathBuf, ChiknError> {
    validate_scrivener_uuid(uuid)?;
    let extension = sanitize_file_extension(extension)?;

    Ok(scriv_path
        .join("Files")
        .join("Data")
        .join(uuid)
        .join(format!("content.{}", extension)))
}

/// Sanitizes a Scrivener media file extension before path interpolation.
pub fn sanitize_file_extension(extension: &str) -> Result<String, ChiknError> {
    let trimmed = extension.trim();
    let extension = trimmed.strip_prefix('.').unwrap_or(trimmed);

    if extension.is_empty() {
        return Err(invalid_file_extension(extension));
    }

    if extension.len() > 32 {
        return Err(invalid_file_extension(extension));
    }

    let path = Path::new(extension);
    let mut components = path.components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        return Err(invalid_file_extension(extension));
    }

    if !extension.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return Err(invalid_file_extension(extension));
    }

    Ok(extension.to_string())
}

fn invalid_file_extension(extension: &str) -> ChiknError {
    ChiknError::InvalidFormat(format!(
        "Invalid Scrivener FileExtension path component: {extension:?}"
    ))
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
        let uuid = "F8F9FDEF-FD9F-4A8C-B33D-3434A1220ADC";

        let rtf_path = get_rtf_path(scriv_path, uuid).unwrap();

        assert_eq!(
            rtf_path,
            Path::new(
                "/path/to/Project.scriv/Files/Data/F8F9FDEF-FD9F-4A8C-B33D-3434A1220ADC/content.rtf"
            )
        );
    }

    #[test]
    fn test_get_rtf_path_rejects_absolute_uuid() {
        let scriv_path = Path::new("/path/to/Project.scriv");
        let result = get_rtf_path(scriv_path, "/tmp/host-file");

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
    }

    #[test]
    fn test_get_rtf_path_rejects_parent_dir_uuid() {
        let scriv_path = Path::new("/path/to/Project.scriv");
        let result = get_rtf_path(scriv_path, "../../host-file");

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
    }

    #[test]
    fn test_get_media_path_rejects_uuid_before_host_path_resolution() {
        let scriv_path = Path::new("/path/to/Project.scriv");
        let result = get_media_path(scriv_path, "/tmp/host-file", "pdf");

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
    }

    #[test]
    fn test_sanitize_file_extension_accepts_common_extensions() {
        for (raw, expected) in [
            ("pdf", "pdf"),
            ("jpg", "jpg"),
            ("jpeg", "jpeg"),
            ("png", "png"),
            ("gif", "gif"),
            ("tiff", "tiff"),
            ("mp3", "mp3"),
            ("mp4", "mp4"),
            ("docx", "docx"),
            ("7z", "7z"),
            (" .PDF ", "PDF"),
        ] {
            assert_eq!(sanitize_file_extension(raw).unwrap(), expected);
        }
    }

    #[test]
    fn test_sanitize_file_extension_rejects_unsafe_values() {
        for raw in [
            "",
            "   ",
            ".",
            "..",
            "../pdf",
            "pdf/../../md",
            "/tmp/pdf",
            r"C:\tmp\pdf",
            r"\\server\share",
            "pdf;rm",
            "pdf $(touch x)",
            "pdf\nmd",
            "tar.gz",
            "pdf:ads",
        ] {
            let result = sanitize_file_extension(raw);
            assert!(
                matches!(result, Err(ChiknError::InvalidFormat(_))),
                "expected {raw:?} to be rejected, got {result:?}"
            );
        }
    }

    #[test]
    fn test_get_media_path_uses_sanitized_file_extension() {
        let scriv_path = Path::new("/path/to/Project.scriv");
        let uuid = "F8F9FDEF-FD9F-4A8C-B33D-3434A1220ADC";

        let media_path = get_media_path(scriv_path, uuid, " .pdf ").unwrap();

        assert_eq!(
            media_path,
            Path::new(
                "/path/to/Project.scriv/Files/Data/F8F9FDEF-FD9F-4A8C-B33D-3434A1220ADC/content.pdf"
            )
        );
    }
}
