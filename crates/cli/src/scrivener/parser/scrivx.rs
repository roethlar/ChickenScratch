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

use chickenscratch_core::utils::error::ChiknError;
use quick_xml::de::from_str;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

    /// Label names by ID from `LabelSettings` (Scrivener 3 stores an item's
    /// label as `<LabelID>`; `-1` is the "No Label" sentinel).
    #[serde(default)]
    pub labels: HashMap<i64, String>,

    /// Status names by ID from `StatusSettings` (`-1` is "No Status").
    #[serde(default)]
    pub statuses: HashMap<i64, String>,

    /// Keyword titles by ID from the project `Keywords` tree, flattened.
    #[serde(default)]
    pub keywords: HashMap<i64, String>,

    /// Custom metadata fields from `CustomMetaDataSettings`, in order:
    /// (field ID, title).
    #[serde(default)]
    pub custom_fields: Vec<(String, String)>,
}

impl ScrivenerProject {
    /// The label an item carries, by name. The `-1` sentinel and the
    /// literal "No Label" are none; the configurable default is a real
    /// label and resolves like any other (review finding plan-3 R1-6).
    pub fn label_name(&self, meta: Option<&BinderMetadata>) -> Option<String> {
        let meta = meta?;
        if let Some(id) = meta.label_id {
            return self.named(&self.labels, id, "No Label");
        }
        meta.label
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty() && *s != "No Label")
            .map(str::to_string)
    }

    /// The status an item carries, by name (see [`Self::label_name`]).
    pub fn status_name(&self, meta: Option<&BinderMetadata>) -> Option<String> {
        let meta = meta?;
        if let Some(id) = meta.status_id {
            return self.named(&self.statuses, id, "No Status");
        }
        meta.status
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty() && *s != "No Status")
            .map(str::to_string)
    }

    /// The keyword titles an item carries: ID references resolved against
    /// the project table, plus any name-form text.
    pub fn keyword_names(&self, meta: Option<&BinderMetadata>) -> Vec<String> {
        let Some(refs) = meta.and_then(|m| m.keywords.as_ref()) else {
            return Vec::new();
        };
        let mut names: Vec<String> = refs
            .ids
            .iter()
            .filter_map(|id| self.keywords.get(id).cloned())
            .collect();
        if let Some(text) = refs.text.as_deref() {
            names.extend(
                text.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
            );
        }
        names
    }

    fn named(&self, table: &HashMap<i64, String>, id: i64, sentinel: &str) -> Option<String> {
        if id < 0 {
            return None;
        }
        table
            .get(&id)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && *s != sentinel)
            .map(str::to_string)
    }
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

    /// Scrivener 3: the label as an ID into `LabelSettings`.
    #[serde(rename = "LabelID", default)]
    pub label_id: Option<i64>,

    /// Scrivener 3: the status as an ID into `StatusSettings`.
    #[serde(rename = "StatusID", default)]
    pub status_id: Option<i64>,

    /// Keywords: Scrivener 3 writes `<KeywordID>` references; older files
    /// wrote names as text.
    #[serde(rename = "Keywords", default)]
    pub keywords: Option<KeywordRefs>,

    /// Custom metadata values, keyed by field ID.
    #[serde(rename = "CustomMetaData", default)]
    pub custom: Option<CustomMetaData>,

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

/// `<Keywords>` on a binder item: `<KeywordID>` children (Scrivener 3) or
/// name text (older files).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeywordRefs {
    #[serde(rename = "KeywordID", default)]
    pub ids: Vec<i64>,
    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

/// `<CustomMetaData>` on a binder item.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CustomMetaData {
    #[serde(rename = "MetaDataItem", default)]
    pub items: Vec<MetaDataItem>,
}

/// One custom metadata value: the field's ID and the item's value.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetaDataItem {
    #[serde(rename = "FieldID", default)]
    pub field_id: String,
    #[serde(rename = "Value", default)]
    pub value: Option<String>,
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
/// use chikn_converter::scrivener::parser::parse_scrivx;
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

    let labels = project
        .label_settings
        .map(|s| s.labels.items.into_iter().map(|l| (l.id, l.name)).collect())
        .unwrap_or_default();
    let statuses = project
        .status_settings
        .map(|s| s.items.items.into_iter().map(|l| (l.id, l.name)).collect())
        .unwrap_or_default();
    let mut keywords = HashMap::new();
    if let Some(tree) = project.keywords {
        flatten_keywords(tree.items, &mut keywords);
    }
    let custom_fields = project
        .custom_settings
        .map(|s| {
            s.fields
                .into_iter()
                .map(|f| (f.id, f.title.unwrap_or_default()))
                .collect()
        })
        .unwrap_or_default();

    Ok(ScrivenerProject {
        name,
        version: project.version.unwrap_or_else(|| "3.0".to_string()),
        binder: project.binder.items,
        labels,
        statuses,
        keywords,
        custom_fields,
    })
}

fn flatten_keywords(items: Vec<KeywordXml>, out: &mut HashMap<i64, String>) {
    for item in items {
        if let Some(title) = item.title {
            out.insert(item.id, title);
        }
        if let Some(children) = item.children {
            flatten_keywords(children.items, out);
        }
    }
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

    #[serde(rename = "LabelSettings", default)]
    label_settings: Option<LabelSettingsXml>,

    #[serde(rename = "StatusSettings", default)]
    status_settings: Option<StatusSettingsXml>,

    #[serde(rename = "Keywords", default)]
    keywords: Option<KeywordsXml>,

    #[serde(rename = "CustomMetaDataSettings", default)]
    custom_settings: Option<CustomMetaDataSettingsXml>,
}

#[derive(Debug, Deserialize)]
struct BinderContainer {
    #[serde(rename = "BinderItem", default)]
    items: Vec<BinderItem>,
}

#[derive(Debug, Deserialize)]
struct LabelSettingsXml {
    #[serde(rename = "Labels", default)]
    labels: LabelsXml,
}

#[derive(Debug, Deserialize, Default)]
struct LabelsXml {
    #[serde(rename = "Label", default)]
    items: Vec<LabelXml>,
}

#[derive(Debug, Deserialize)]
struct StatusSettingsXml {
    #[serde(rename = "StatusItems", default)]
    items: StatusItemsXml,
}

#[derive(Debug, Deserialize, Default)]
struct StatusItemsXml {
    #[serde(rename = "Status", default)]
    items: Vec<StatusXml>,
}

#[derive(Debug, Deserialize)]
struct LabelXml {
    #[serde(rename = "@ID")]
    id: i64,
    #[serde(rename = "$text", default)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct StatusXml {
    #[serde(rename = "@ID")]
    id: i64,
    #[serde(rename = "$text", default)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct KeywordsXml {
    #[serde(rename = "Keyword", default)]
    items: Vec<KeywordXml>,
}

#[derive(Debug, Deserialize)]
struct KeywordXml {
    #[serde(rename = "@ID")]
    id: i64,
    #[serde(rename = "Title", default)]
    title: Option<String>,
    #[serde(rename = "Children", default)]
    children: Option<KeywordsXml>,
}

#[derive(Debug, Deserialize)]
struct CustomMetaDataSettingsXml {
    #[serde(rename = "MetaDataField", default)]
    fields: Vec<MetaDataFieldXml>,
}

#[derive(Debug, Deserialize)]
struct MetaDataFieldXml {
    #[serde(rename = "@ID")]
    id: String,
    #[serde(rename = "Title", default)]
    title: Option<String>,
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

/// A named side file of an item (`synopsis.txt`, `notes.rtf`,
/// `content.comments`) under `Files/Data/<UUID>/`.
pub fn get_data_file_path(
    scriv_path: &Path,
    uuid: &str,
    file_name: &str,
) -> Result<PathBuf, ChiknError> {
    validate_scrivener_uuid(uuid)?;
    Ok(scriv_path
        .join("Files")
        .join("Data")
        .join(uuid)
        .join(file_name))
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
