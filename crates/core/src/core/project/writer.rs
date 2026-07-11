//! # Project Writer
//!
//! Writes Project structs to disk as .chikn format.
//!
//! ## Responsibilities
//! - Serialize Project to project.yaml
//! - Write all document content to .md files
//! - Write document metadata to .meta files
//! - Atomic writes (temp file + rename)
//! - Create required directory structure
//!
//! ## Example
//! ```no_run
//! use std::path::Path;
//! use chickenscratch_core::core::project::fidelity::acquire_write_token;
//! use chickenscratch_core::core::project::writer::{create_project, write_project};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut project = create_project(Path::new("MyNovel.chikn"), "My Novel")?;
//! let token = acquire_write_token(Path::new("MyNovel.chikn"))?;
//! write_project(&mut project, &token)?;
//! println!("Project saved successfully");
//! # Ok(()) }
//! ```

use chrono::Utc;
use std::collections::HashSet;
use std::fs;
use std::io::ErrorKind;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use super::fidelity::WriteToken;
use super::format::{
    get_document_meta_path, get_project_file_path, get_threads_path, FORMAT_VERSION,
    REQUIRED_FOLDERS,
};
use super::reader::{lift_legacy_novelist_keys, DocumentMetadata, ProjectMetadata};
use super::safe_path;
use crate::models::{Project, TreeNode};
use crate::utils::error::ChiknError;

/// Writes a Project to disk as a .chikn project.
///
/// # Arguments
/// * `project` - Mutable reference to project (modified timestamp will be updated)
/// * `token` - Write capability for this project's root (see
///   [`super::fidelity::acquire_write_token`]) — a Degraded project can
///   never obtain one, so it can never be saved over.
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(ChiknError)` on failure
///
/// # Errors
/// - `ReadOnly`: token is for another project or stale
/// - `Io`: File system errors during writing
/// - `Serialization`: YAML serialization errors
///
/// # Example
/// ```no_run
/// # use std::path::Path;
/// # use chickenscratch_core::core::project::fidelity::acquire_write_token;
/// # use chickenscratch_core::core::project::writer::{create_project, write_project};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let mut project = create_project(Path::new("MyNovel.chikn"), "My Novel")?;
/// # let token = acquire_write_token(Path::new("MyNovel.chikn"))?;
/// write_project(&mut project, &token)?;
/// # Ok(()) }
/// ```
pub fn write_project(project: &mut Project, token: &WriteToken) -> Result<(), ChiknError> {
    let project_path = Path::new(&project.path);
    token.ensure_valid_for(project_path)?;

    // Update modified timestamp
    project.modified = Utc::now().to_rfc3339();

    // Create directory structure if it doesn't exist
    create_project_structure(project_path)?;

    // Fail before rewriting project.yaml if any document path would escape
    // the project or traverse a symlink.
    validate_all_document_targets(project)?;

    // Write project.yaml
    write_project_metadata(project)?;

    // Write all documents
    write_all_documents(project)?;

    // Write threads.yaml — only when there are threads to persist; never deletes.
    write_threads_if_any(project)?;

    Ok(())
}

/// Writes `threads.yaml` if the project has any threads. Empty thread vec
/// removes the existing sidecar (if present) so deleting the last thread
/// actually persists — without the cleanup, the file lingered with the
/// pre-deletion threads forever and a reload would resurrect them.
fn write_threads_if_any(project: &Project) -> Result<(), ChiknError> {
    let path = get_threads_path(Path::new(&project.path));
    if project.threads.is_empty() {
        if path.exists() {
            // Surface remove errors. Silently swallowing means a failed
            // unlink (perms, lock, etc.) leaves the OLD threads.yaml on
            // disk; the next reader run resurrects every "deleted"
            // thread. Better to fail the operation so the user knows
            // the delete didn't take.
            fs::remove_file(&path)?;
        }
        return Ok(());
    }
    #[derive(serde::Serialize)]
    struct ThreadsFile<'a> {
        threads: &'a [crate::models::Thread],
    }
    let payload = ThreadsFile {
        threads: &project.threads,
    };
    let yaml = serde_yaml::to_string(&payload)?;
    atomic_write_file(&path, yaml.as_bytes())?;
    Ok(())
}

/// Creates a new .chikn project on disk with required folder structure.
///
/// # Arguments
/// * `path` - Path where project should be created
/// * `name` - Project name
///
/// # Returns
/// * `Ok(Project)` - Newly created project struct
/// * `Err(ChiknError)` on failure
///
/// # Errors
/// - `Io`: File system errors
/// - `InvalidFormat`: Path already exists
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use chickenscratch_core::core::project::writer::create_project;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let project = create_project(Path::new("MyNovel.chikn"), "My Novel")?;
/// # Ok(()) }
/// ```
pub fn create_project(path: &Path, name: &str) -> Result<Project, ChiknError> {
    // Check if path already exists
    if path.exists() {
        return Err(ChiknError::InvalidFormat(format!(
            "Project already exists: {}",
            path.display()
        )));
    }

    // Create directory structure
    create_project_structure(path)?;

    // Create initial project
    let now = Utc::now().to_rfc3339();
    let project = Project {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        path: path.to_string_lossy().to_string(),
        hierarchy: vec![
            TreeNode::Folder {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Manuscript".to_string(),
                children: Vec::new(),
            },
            TreeNode::Folder {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Research".to_string(),
                children: Vec::new(),
            },
            // Trash is structural — without it the binder's "Move to Trash"
            // path falls through to permanent delete on first use, which
            // means new-project sessions lose work. The reader's repair
            // step also adds it on reload, but that's too late for the
            // first session.
            TreeNode::Folder {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Trash".to_string(),
                children: Vec::new(),
            },
        ],
        documents: std::collections::HashMap::new(),
        created: now.clone(),
        modified: now,
        metadata: Default::default(),
        threads: Vec::new(),
    };

    // Write .gitignore
    let gitignore = path.join(".gitignore");
    fs::write(
        &gitignore,
        "# Automatic snapshots (use git history instead)\nrevs/\n\n# OS files\n.DS_Store\nThumbs.db\n\n# Editor temp files\n*.tmp\n*.swp\n*~\n",
    )?;

    // Write initial project.yaml
    write_project_metadata(&project)?;

    // Initialize git repository (no commit — caller decides when content is ready)
    let _ = crate::core::git::init_repo(path);

    Ok(project)
}

/// Creates the required folder structure for a .chikn project
fn create_project_structure(path: &Path) -> Result<(), ChiknError> {
    // Create root directory
    fs::create_dir_all(path)?;

    let project_root = canonical_project_root(path)?;
    for folder in REQUIRED_FOLDERS {
        ensure_safe_directory_path(path, &project_root, Path::new(folder), folder)?;
    }

    Ok(())
}

/// Read the existing project.yaml so unknown top-level keys can be carried
/// into the rewrite (I5). Missing file → Ok(None) (fresh create). A present
/// but unparseable file is an error: overwriting YAML we cannot read would
/// destroy whatever it held, so the save aborts before touching disk —
/// mirrors the corrupt-.meta guard in `read_existing_document_metadata`.
fn read_existing_project_metadata(
    project_file: &Path,
) -> Result<Option<ProjectMetadata>, ChiknError> {
    match fs::read_to_string(project_file) {
        Ok(content) => serde_yaml::from_str::<ProjectMetadata>(&content)
            .map(Some)
            .map_err(|e| {
                ChiknError::InvalidFormat(format!(
                    "Failed to parse existing project metadata at {}: {}",
                    project_file.display(),
                    e
                ))
            }),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Writes project.yaml metadata
fn write_project_metadata(project: &Project) -> Result<(), ChiknError> {
    let project_path = Path::new(&project.path);
    let project_file = get_project_file_path(project_path);

    // Preserve unknown top-level keys from the existing file (I5).
    let existing = read_existing_project_metadata(&project_file)?;

    let metadata = ProjectMetadata {
        // Always stamp the version this writer speaks — a legacy
        // version-less project gains the marker on its first save.
        format_version: Some(FORMAT_VERSION.to_string()),
        id: project.id.clone(),
        name: project.name.clone(),
        hierarchy: project.hierarchy.clone(),
        created: project.created.clone(),
        modified: project.modified.clone(),
        metadata: project.metadata.clone(),
        extra: existing.map(|m| m.extra).unwrap_or_default(),
    };

    let yaml_content = serde_yaml::to_string(&metadata)?;

    atomic_write_file(&project_file, yaml_content.as_bytes())?;

    Ok(())
}

/// Writes all documents to their respective folders
fn write_all_documents(project: &Project) -> Result<(), ChiknError> {
    let project_path = Path::new(&project.path);

    for document in project.documents.values() {
        write_document(project_path, document)?;
    }

    Ok(())
}

fn validate_all_document_targets(project: &Project) -> Result<(), ChiknError> {
    let project_path = Path::new(&project.path);
    let project_root = canonical_project_root(project_path)?;
    let mut document_ids = HashSet::new();
    let mut document_paths = HashSet::new();

    for (map_id, document) in &project.documents {
        if map_id != &document.id {
            return Err(ChiknError::InvalidFormat(format!(
                "Document map key {map_id} does not match document id {}",
                document.id
            )));
        }
        if !document_ids.insert(document.id.clone()) {
            return Err(ChiknError::InvalidFormat(format!(
                "Duplicate document id: {}",
                document.id
            )));
        }
        validate_relative_document_path(&document.path)?;
        let normalized_path = normalized_relative_document_path(&document.path)?;
        if !document_paths.insert(normalized_path) {
            return Err(ChiknError::InvalidFormat(format!(
                "Duplicate document path: {}",
                document.path
            )));
        }

        let doc_path = Path::new(&document.path);
        let full_content_path = project_path.join(doc_path);
        let folder_path = full_content_path.parent().ok_or_else(|| {
            ChiknError::InvalidFormat(format!("Document has no parent: {}", document.path))
        })?;
        let relative_parent = doc_path.parent().ok_or_else(|| {
            ChiknError::InvalidFormat(format!("Document has no parent: {}", document.path))
        })?;
        ensure_existing_ancestors_safe(
            project_path,
            &project_root,
            relative_parent,
            &document.path,
        )?;
        ensure_existing_path_safe(
            &full_content_path,
            &project_root,
            &document.path,
            "document file",
        )?;

        let doc_name = full_content_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                ChiknError::InvalidFormat(format!("Invalid document path: {}", document.path))
            })?;
        let meta_path = get_document_meta_path(folder_path, doc_name);
        ensure_existing_path_safe(
            &meta_path,
            &project_root,
            &document.path,
            "document metadata",
        )?;
        let _ = read_existing_document_metadata(&meta_path)?;
    }

    Ok(())
}

fn validate_relative_document_path(document_path: &str) -> Result<(), ChiknError> {
    let path = Path::new(document_path);
    let mut has_normal_component = false;

    for component in path.components() {
        match component {
            Component::Normal(_) => {
                has_normal_component = true;
            }
            Component::CurDir => {
                return Err(invalid_document_path(
                    document_path,
                    "current-directory components are not allowed",
                ));
            }
            Component::ParentDir => {
                return Err(invalid_document_path(
                    document_path,
                    "parent-directory components are not allowed",
                ));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(invalid_document_path(
                    document_path,
                    "absolute paths are not allowed",
                ));
            }
        }
    }

    if !has_normal_component {
        return Err(invalid_document_path(
            document_path,
            "path must contain a file name",
        ));
    }

    Ok(())
}

fn normalized_relative_document_path(document_path: &str) -> Result<String, ChiknError> {
    let slash_path = document_path.replace('\\', "/");
    validate_relative_document_path(&slash_path)?;
    let components: Vec<String> = Path::new(&slash_path)
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();

    Ok(components.join("/"))
}

fn invalid_document_path(document_path: &str, reason: &str) -> ChiknError {
    ChiknError::InvalidFormat(format!(
        "Document path must be relative and within project ({}): {}",
        reason, document_path
    ))
}

fn canonical_project_root(project_path: &Path) -> Result<PathBuf, ChiknError> {
    let project_root = project_path.canonicalize()?;
    if !project_root.is_dir() {
        return Err(ChiknError::InvalidFormat(format!(
            "Project path is not a directory: {}",
            project_path.display()
        )));
    }
    Ok(project_root)
}

fn ensure_document_parent_directory(
    project_path: &Path,
    document_path: &str,
) -> Result<PathBuf, ChiknError> {
    validate_relative_document_path(document_path)?;

    let doc_path = Path::new(document_path);
    let relative_parent = doc_path.parent().ok_or_else(|| {
        ChiknError::InvalidFormat(format!("Document has no parent: {}", document_path))
    })?;
    let project_root = canonical_project_root(project_path)?;
    ensure_safe_directory_path(project_path, &project_root, relative_parent, document_path)
}

fn ensure_safe_directory_path(
    project_path: &Path,
    project_root: &Path,
    relative_path: &Path,
    document_path: &str,
) -> Result<PathBuf, ChiknError> {
    let mut current = project_path.to_path_buf();

    for component in relative_path.components() {
        let Component::Normal(part) = component else {
            return Err(invalid_document_path(
                document_path,
                "directory path contains unsafe components",
            ));
        };
        current.push(part);

        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                ensure_directory_metadata_safe(&current, &metadata, project_root, document_path)?;
            }
            Err(e) if e.kind() == ErrorKind::NotFound => {
                fs::create_dir(&current)?;
                let metadata = fs::symlink_metadata(&current)?;
                ensure_directory_metadata_safe(&current, &metadata, project_root, document_path)?;
            }
            Err(e) => return Err(e.into()),
        }
    }

    Ok(current)
}

fn ensure_existing_ancestors_safe(
    project_path: &Path,
    project_root: &Path,
    relative_parent: &Path,
    document_path: &str,
) -> Result<(), ChiknError> {
    let mut current = project_path.to_path_buf();

    for component in relative_parent.components() {
        let Component::Normal(part) = component else {
            return Err(invalid_document_path(
                document_path,
                "directory path contains unsafe components",
            ));
        };
        current.push(part);

        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                ensure_directory_metadata_safe(&current, &metadata, project_root, document_path)?;
            }
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}

fn ensure_directory_metadata_safe(
    path: &Path,
    metadata: &fs::Metadata,
    project_root: &Path,
    document_path: &str,
) -> Result<(), ChiknError> {
    if metadata.file_type().is_symlink() {
        return Err(ChiknError::InvalidFormat(format!(
            "Document path traverses a symlink: {} ({})",
            document_path,
            path.display()
        )));
    }
    if !metadata.is_dir() {
        return Err(ChiknError::InvalidFormat(format!(
            "Document path parent is not a directory: {} ({})",
            document_path,
            path.display()
        )));
    }
    ensure_canonical_path_within_project(path, project_root, document_path)
}

fn ensure_existing_path_safe(
    path: &Path,
    project_root: &Path,
    document_path: &str,
    kind: &str,
) -> Result<(), ChiknError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(ChiknError::InvalidFormat(format!(
                    "Document {} is a symlink: {} ({})",
                    kind,
                    document_path,
                    path.display()
                )));
            }
            ensure_canonical_path_within_project(path, project_root, document_path)
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

fn ensure_canonical_path_within_project(
    path: &Path,
    project_root: &Path,
    document_path: &str,
) -> Result<(), ChiknError> {
    let canonical_path = path.canonicalize()?;
    if !canonical_path.starts_with(project_root) {
        return Err(ChiknError::InvalidFormat(format!(
            "Document path escapes project root: {} ({})",
            document_path,
            path.display()
        )));
    }
    Ok(())
}

fn read_existing_document_metadata(
    meta_path: &Path,
) -> Result<Option<DocumentMetadata>, ChiknError> {
    match fs::read_to_string(meta_path) {
        Ok(content) => serde_yaml::from_str::<DocumentMetadata>(&content)
            .map(Some)
            .map_err(|e| {
                ChiknError::InvalidFormat(format!(
                    "Failed to parse document metadata at {}: {}",
                    meta_path.display(),
                    e
                ))
            }),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Writes a single document (content + metadata)
fn write_document(
    project_path: &Path,
    document: &crate::models::Document,
) -> Result<(), ChiknError> {
    // Text content is only ever written to .md files. Anything else a
    // binder can reference (imported PDFs, images, audio) is an opaque
    // asset: its sidecar metadata is ours to maintain, its content bytes
    // are not — they are copied verbatim at import time and never touched
    // again (see `write_document_blobs` for byte-faithful restores).
    let is_md = Path::new(&document.path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case(super::format::DOCUMENT_EXTENSION))
        .unwrap_or(false);

    let (full_content_path, meta_path) = safe_document_write_paths(project_path, &document.path)?;

    // Read existing metadata to preserve fields we don't model in Document
    let existing_meta = read_existing_document_metadata(&meta_path)?;

    let mut metadata = DocumentMetadata {
        id: document.id.clone(),
        name: Some(document.name.clone()),
        created: document.created.clone(),
        // Preserve the doc's own `modified` instead of stamping `now()`.
        // `write_project` writes EVERY doc on each call, so a fresh
        // `now()` here would bump every .meta's timestamp on every
        // unrelated save (rename/move/comment/etc.) and produce noisy
        // git diffs plus inaccurate per-doc modified dates. Callers who
        // genuinely change a doc bump `document.modified` themselves
        // before this runs (see commands/document.rs).
        modified: document.modified.clone(),
        parent_id: document.parent_id.clone(),
        label: document.label.clone(),
        status: document.status.clone(),
        keywords: document.keywords.clone(),
        synopsis: document.synopsis.clone(),
        section_type: existing_meta.as_ref().and_then(|m| m.section_type.clone()),
        include_in_compile: Some(
            if document.include_in_compile {
                "Yes"
            } else {
                "No"
            }
            .to_string(),
        ),
        scrivener_uuid: existing_meta
            .as_ref()
            .and_then(|m| m.scrivener_uuid.clone()),
        links: document.links.clone(),
        word_count_target: document.word_count_target,
        compile_order: document.compile_order,
        comments: document.comments.clone(),
        fields: document.fields.clone(),
        // Unknown top-level sidecar keys survive the rewrite (I5), same
        // re-read pattern as section_type / scrivener_uuid above.
        extra: existing_meta
            .as_ref()
            .map(|m| m.extra.clone())
            .unwrap_or_default(),
    };

    // The existing .meta may still carry legacy top-level novelist keys the
    // reader lifted into `fields` at load time; relocate them here too so
    // the save moves them on disk instead of duplicating them.
    lift_legacy_novelist_keys(&mut metadata);

    let meta_content = serde_yaml::to_string(&metadata)?;

    // Write content (.md file) — never for opaque assets (see above).
    if is_md {
        atomic_write_file(&full_content_path, document.content.as_bytes())?;
    }
    atomic_write_file(&meta_path, meta_content.as_bytes())?;

    Ok(())
}

/// Write restored document bytes through the same path validation, symlink
/// rejection, and atomic-file replacement used by normal project saves.
pub(crate) fn write_document_blobs(
    project_path: &Path,
    document_path: &str,
    content: &[u8],
    metadata: Option<&[u8]>,
) -> Result<(), ChiknError> {
    let (full_content_path, meta_path) = safe_document_write_paths(project_path, document_path)?;

    atomic_write_file(&full_content_path, content)?;
    if let Some(metadata) = metadata {
        atomic_write_file(&meta_path, metadata)?;
    }

    Ok(())
}

fn safe_document_write_paths(
    project_path: &Path,
    document_path: &str,
) -> Result<(PathBuf, PathBuf), ChiknError> {
    let full_content_path = project_path.join(document_path);

    let folder_path = ensure_document_parent_directory(project_path, document_path)?;
    let doc_name = full_content_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            ChiknError::InvalidFormat(format!("Invalid document path: {}", document_path))
        })?;

    let project_root = canonical_project_root(project_path)?;
    ensure_existing_path_safe(
        &full_content_path,
        &project_root,
        document_path,
        "document file",
    )?;

    let meta_path = get_document_meta_path(&folder_path, doc_name);
    ensure_existing_path_safe(
        &meta_path,
        &project_root,
        document_path,
        "document metadata",
    )?;

    Ok((full_content_path, meta_path))
}

fn atomic_write_file(path: &Path, contents: &[u8]) -> Result<(), ChiknError> {
    let parent = path.parent().ok_or_else(|| {
        ChiknError::InvalidFormat(format!(
            "Cannot write path without parent: {}",
            path.display()
        ))
    })?;
    let file_name = path.file_name().ok_or_else(|| {
        ChiknError::InvalidFormat(format!(
            "Cannot write path without file name: {}",
            path.display()
        ))
    })?;
    let existing_permissions = fs::metadata(path)
        .ok()
        .map(|metadata| metadata.permissions());

    let prefix = format!(".{}.tmp-", file_name.to_string_lossy());
    let mut temp_file = tempfile::Builder::new()
        .prefix(&prefix)
        .tempfile_in(parent)?;
    temp_file.write_all(contents)?;
    if let Some(permissions) = existing_permissions {
        temp_file.as_file().set_permissions(permissions)?;
    }
    temp_file.as_file().sync_all()?;
    let _persisted = temp_file.persist(path).map_err(std::io::Error::from)?;
    sync_parent_directory(parent)?;
    Ok(())
}

#[cfg(unix)]
fn sync_parent_directory(parent: &Path) -> Result<(), ChiknError> {
    fs::File::open(parent)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_parent_directory(_parent: &Path) -> Result<(), ChiknError> {
    Ok(())
}

/// Deletes a document from disk using its stored path
///
/// # Arguments
/// * `project_path` - Root path of project
/// * `document_path` - Document's relative path (from Document.path)
/// * `token` - Write capability for this project's root
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(ChiknError)` if file doesn't exist or can't be deleted
pub fn delete_document(
    project_path: &Path,
    document_path: &str,
    token: &WriteToken,
) -> Result<(), ChiknError> {
    token.ensure_valid_for(project_path)?;
    validate_relative_document_path(document_path)?;

    // Resolve full paths
    let full_content_path = project_path.join(document_path);
    let project_root = canonical_project_root(project_path)?;
    let relative_parent = Path::new(document_path).parent().ok_or_else(|| {
        ChiknError::InvalidFormat(format!("Document has no parent: {}", document_path))
    })?;
    ensure_existing_ancestors_safe(project_path, &project_root, relative_parent, document_path)?;
    ensure_existing_path_safe(
        &full_content_path,
        &project_root,
        document_path,
        "document file",
    )?;

    // Delete .md file
    if full_content_path.exists() {
        fs::remove_file(&full_content_path)?;
    }

    // Delete .meta file
    let doc_name = full_content_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            ChiknError::InvalidFormat(format!("Invalid document path: {}", document_path))
        })?;

    let folder_path = full_content_path.parent().ok_or_else(|| {
        ChiknError::InvalidFormat(format!("Document has no parent: {}", document_path))
    })?;

    let meta_path = get_document_meta_path(folder_path, doc_name);
    ensure_existing_path_safe(
        &meta_path,
        &project_root,
        document_path,
        "document metadata",
    )?;
    if meta_path.exists() {
        fs::remove_file(&meta_path)?;
    }

    Ok(())
}

/// Token-gated write of a project-internal application file (for example
/// `settings/writing-history.json`). App-side writes into the project obey
/// the same gate as document writes: without a `Full` probe there is no
/// token, and nothing can be written. Missing parent folders are created
/// through the same symlink-refusing safe-path machinery as document
/// writes, and the file itself is written atomically.
pub fn write_project_app_file(
    token: &WriteToken,
    relative_path: &Path,
    contents: &[u8],
) -> Result<(), ChiknError> {
    token.ensure_fresh()?;
    let project_path = token.root().to_path_buf();

    let file_name = relative_path.file_name().ok_or_else(|| {
        ChiknError::InvalidFormat(format!(
            "App file path must name a file: {}",
            relative_path.display()
        ))
    })?;
    let parent = relative_path.parent().filter(|p| !p.as_os_str().is_empty());

    let dir = match parent {
        Some(parent) => safe_path::ensure_project_subdir_safe(&project_path, parent)?,
        None => project_path.clone(),
    };
    let target = dir.join(file_name);
    let project_root = canonical_project_root(&project_path)?;
    ensure_existing_path_safe(
        &target,
        &project_root,
        &relative_path.to_string_lossy(),
        "app file",
    )?;
    atomic_write_file(&target, contents)
}

/// Token-gated creation of a project subdirectory (for example the
/// `characters/` entity folder). The former public `safe_path` helpers are
/// engine-private now; this is the sanctioned caller-facing surface.
pub fn ensure_project_subdir(
    token: &WriteToken,
    relative_path: &Path,
) -> Result<PathBuf, ChiknError> {
    token.ensure_fresh()?;
    safe_path::ensure_project_subdir_safe(token.root(), relative_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::project::format::{get_manuscript_path, get_research_path};
    use crate::core::project::reader::read_project;
    use crate::models::{Document, TreeNode};
    use tempfile::TempDir;

    /// Acquire a write token for a freshly created (Full) test project.
    /// Tests that exercise the writer's own internal guards acquire the
    /// token BEFORE corrupting the fixture, mirroring a real session that
    /// opened a healthy project and then hit damage mid-flight.
    fn test_token(project_path: &Path) -> WriteToken {
        crate::core::project::fidelity::acquire_write_token(project_path).expect("write token")
    }

    #[test]
    fn test_create_project() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("NewProject.chikn");

        let result = create_project(&project_path, "New Project");
        assert!(result.is_ok());

        let project = result.unwrap();
        assert_eq!(project.name, "New Project");
        assert!(project_path.exists());
        assert!(get_manuscript_path(&project_path).exists());
        assert!(get_research_path(&project_path).exists());
    }

    #[test]
    fn test_create_project_already_exists() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("ExistingProject.chikn");

        // Create once
        create_project(&project_path, "Test").unwrap();

        // Try to create again - should fail
        let result = create_project(&project_path, "Test");
        assert!(result.is_err());
    }

    #[test]
    fn test_write_project() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("TestProject.chikn");

        let mut project = create_project(&project_path, "Test Project").unwrap();
        let token = test_token(&project_path);

        // Add a document
        let doc = Document {
            id: "doc1".to_string(),
            name: "chapter-01".to_string(),
            path: "manuscript/chapter-01.md".to_string(),
            content: "# Chapter 1\n\nTest content".to_string(),
            parent_id: None,
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };

        project.documents.insert(doc.id.clone(), doc.clone());
        project.hierarchy.push(TreeNode::Document {
            id: doc.id.clone(),
            name: doc.name.clone(),
            path: doc.path.clone(),
        });

        // Write project
        let result = write_project(&mut project, &token);
        assert!(result.is_ok());

        // Verify files exist
        let content_path = get_manuscript_path(&project_path).join("chapter-01.md");
        assert!(content_path.exists());

        let meta_path = get_manuscript_path(&project_path).join("chapter-01.meta");
        assert!(meta_path.exists());
    }

    #[test]
    fn test_write_project_rejects_duplicate_document_paths_before_write() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("DuplicatePathProject.chikn");
        let mut project = create_project(&project_path, "Duplicate Path Project").unwrap();
        let token = test_token(&project_path);

        let content_path = get_manuscript_path(&project_path).join("chapter-01.md");
        fs::write(&content_path, "original").unwrap();

        let first = Document {
            id: "doc1".to_string(),
            name: "Chapter 1".to_string(),
            path: "manuscript/chapter-01.md".to_string(),
            content: "first".to_string(),
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };
        let second = Document {
            id: "doc2".to_string(),
            name: "Chapter 1 Copy".to_string(),
            path: "manuscript/chapter-01.md".to_string(),
            content: "second".to_string(),
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };

        project.documents.insert(first.id.clone(), first);
        project.documents.insert(second.id.clone(), second);

        let result = write_project(&mut project, &token);

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
        assert_eq!(fs::read_to_string(content_path).unwrap(), "original");
    }

    #[test]
    fn test_round_trip_write_read() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("RoundTripProject.chikn");

        // Create and write project
        let mut original_project = create_project(&project_path, "Round Trip Test").unwrap();
        let token = test_token(&project_path);

        let doc = Document {
            id: "doc1".to_string(),
            name: "test-document".to_string(),
            path: "manuscript/test-document.md".to_string(),
            content: "Test content for round trip".to_string(),
            parent_id: None,
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };

        original_project
            .documents
            .insert(doc.id.clone(), doc.clone());
        original_project.hierarchy.push(TreeNode::Document {
            id: doc.id.clone(),
            name: doc.name.clone(),
            path: doc.path.clone(),
        });

        write_project(&mut original_project, &token).unwrap();

        // Read project back
        let loaded_project = read_project(&project_path).unwrap();

        // Verify data matches
        assert_eq!(loaded_project.name, original_project.name);
        assert_eq!(loaded_project.id, original_project.id);
        assert_eq!(loaded_project.documents.len(), 1);
        assert!(loaded_project.documents.contains_key("doc1"));

        let loaded_doc = loaded_project.documents.get("doc1").unwrap();
        assert_eq!(loaded_doc.name, "test-document");
        assert_eq!(loaded_doc.content, "Test content for round trip");
    }

    #[test]
    fn test_fields_map_round_trip() {
        // The generic `fields` map is the format's sole UI-extensibility point.
        // Arbitrary keys must survive write → read unchanged, regardless of
        // their shape (string, int, list, nested map). The format has no
        // domain; the novelist UI's convention keys are just three of many
        // possible consumers.
        use serde_yaml::Value;

        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("FieldsMap.chikn");
        let mut project = create_project(&project_path, "Fields Map Test").unwrap();
        let token = test_token(&project_path);

        let mut fields = std::collections::BTreeMap::new();
        // A novelist-convention string.
        fields.insert("pov_character".to_string(), Value::String("sarah".into()));
        // An integer.
        fields.insert("duration_minutes".to_string(), Value::Number(45.into()));
        // A list.
        fields.insert(
            "threads".to_string(),
            Value::Sequence(vec![
                Value::String("main-plot".into()),
                Value::String("romance".into()),
            ]),
        );
        // A nested mapping — the format has no opinion about shape.
        let mut nested = serde_yaml::Mapping::new();
        nested.insert(
            Value::String("scale".into()),
            Value::String("medium".into()),
        );
        nested.insert(Value::String("year".into()), Value::Number(1987.into()));
        fields.insert("world_state".to_string(), Value::Mapping(nested));

        let doc = Document {
            id: "doc1".to_string(),
            name: "opening".to_string(),
            path: "manuscript/opening.md".to_string(),
            content: "The motel was quiet.".to_string(),
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            fields: fields.clone(),
            ..Default::default()
        };

        project.documents.insert(doc.id.clone(), doc.clone());
        project.hierarchy.push(TreeNode::Document {
            id: doc.id.clone(),
            name: doc.name.clone(),
            path: doc.path.clone(),
        });
        write_project(&mut project, &token).unwrap();

        let loaded = read_project(&project_path).unwrap();
        let back = loaded.documents.get("doc1").expect("loaded");
        assert_eq!(back.fields, fields, "fields map must round-trip unchanged");
    }

    #[test]
    fn test_fields_absent_writes_clean_meta() {
        // A document with an empty `fields` map must not emit a `fields:` key
        // at all. That keeps .meta files unchanged for projects that ignore
        // the extensibility point.
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("Clean.chikn");
        let mut project = create_project(&project_path, "Clean").unwrap();
        let token = test_token(&project_path);

        let doc = Document {
            id: "basic".to_string(),
            name: "basic".to_string(),
            path: "manuscript/basic.md".to_string(),
            content: "Plain.".to_string(),
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };
        project.documents.insert(doc.id.clone(), doc.clone());
        project.hierarchy.push(TreeNode::Document {
            id: doc.id.clone(),
            name: doc.name.clone(),
            path: doc.path.clone(),
        });
        write_project(&mut project, &token).unwrap();

        let meta_path = project_path.join("manuscript/basic.meta");
        let meta_text = std::fs::read_to_string(&meta_path).unwrap();
        assert!(
            !meta_text.contains("fields:"),
            "empty fields map must be skipped"
        );
    }

    #[test]
    fn test_unknown_fields_preserved_through_round_trip() {
        // If a reader parses a .meta that has fields it doesn't explicitly
        // understand (written by a newer or differently-configured UI), the
        // writer must emit them back unchanged. "Tolerant readers, preserving
        // writers" — the format-level guarantee. We simulate this by handwriting
        // a .meta with a key we never declare (a hypothetical TTRPG UI field),
        // reading the doc, writing it back, and asserting the key survives.
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("Preserve.chikn");
        let mut project = create_project(&project_path, "Preserve").unwrap();
        let token = test_token(&project_path);

        let doc = Document {
            id: "session-7".to_string(),
            name: "session-seven".to_string(),
            path: "manuscript/session-seven.md".to_string(),
            content: "A dragon showed up.".to_string(),
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };
        project.documents.insert(doc.id.clone(), doc.clone());
        project.hierarchy.push(TreeNode::Document {
            id: doc.id.clone(),
            name: doc.name.clone(),
            path: doc.path.clone(),
        });
        write_project(&mut project, &token).unwrap();

        // Hand-inject a foreign UI field into the .meta on disk.
        let meta_path = project_path.join("manuscript/session-seven.meta");
        let existing = std::fs::read_to_string(&meta_path).unwrap();
        std::fs::write(
            &meta_path,
            format!(
                "{}\nfields:\n  ttrpg_session_date: 2026-04-23\n  ttrpg_encounter_cr: 12\n",
                existing.trim_end()
            ),
        )
        .unwrap();

        // Read, write, read again. The foreign keys must survive.
        let mut reloaded = read_project(&project_path).unwrap();
        write_project(&mut reloaded, &token).unwrap();
        let final_load = read_project(&project_path).unwrap();

        let d = final_load.documents.get("session-7").unwrap();
        assert_eq!(
            d.fields.get("ttrpg_session_date").and_then(|v| v.as_str()),
            Some("2026-04-23"),
            "unknown string field must round-trip"
        );
        assert_eq!(
            d.fields.get("ttrpg_encounter_cr").and_then(|v| v.as_i64()),
            Some(12),
            "unknown numeric field must round-trip"
        );
    }

    #[test]
    fn test_unknown_top_level_meta_keys_survive_round_trip() {
        // I5: unknown TOP-LEVEL sidecar keys (not nested under `fields:`)
        // written by another or newer tool must survive a read→write cycle.
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("TopLevel.chikn");
        let mut project = create_project(&project_path, "TopLevel").unwrap();
        let token = test_token(&project_path);

        let doc = Document {
            id: "doc1".to_string(),
            name: "chapter-01".to_string(),
            path: "manuscript/chapter-01.md".to_string(),
            content: "Text.".to_string(),
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };
        project.documents.insert(doc.id.clone(), doc.clone());
        project.hierarchy.push(TreeNode::Document {
            id: doc.id.clone(),
            name: doc.name.clone(),
            path: doc.path.clone(),
        });
        write_project(&mut project, &token).unwrap();

        let meta_path = project_path.join("manuscript/chapter-01.meta");
        let existing = std::fs::read_to_string(&meta_path).unwrap();
        std::fs::write(
            &meta_path,
            format!(
                "{}\nfuture_format_key: from-the-future\nanother_tool:\n  nested: true\n",
                existing.trim_end()
            ),
        )
        .unwrap();

        let mut reloaded = read_project(&project_path).unwrap();
        write_project(&mut reloaded, &token).unwrap();

        let rewritten = std::fs::read_to_string(&meta_path).unwrap();
        assert!(
            rewritten.contains("future_format_key: from-the-future"),
            "unknown top-level scalar key must survive rewrite:\n{rewritten}"
        );
        assert!(
            rewritten.contains("another_tool:") && rewritten.contains("nested: true"),
            "unknown top-level mapping key must survive rewrite:\n{rewritten}"
        );
    }

    #[test]
    fn test_unknown_project_yaml_keys_survive_round_trip() {
        // I5 for project.yaml: unknown keys at the top level and inside the
        // metadata: block must survive a read→write cycle.
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("ProjExtra.chikn");
        let mut project = create_project(&project_path, "ProjExtra").unwrap();
        let token = test_token(&project_path);
        // Ensure the metadata: block exists as a mapping we can patch below.
        project.metadata.title = Some("ProjExtra".into());
        write_project(&mut project, &token).unwrap();

        let project_file = project_path.join("project.yaml");
        let existing = std::fs::read_to_string(&project_file).unwrap();
        let patched = format!("{}\nfuture_top_level: keep-me\n", existing.trim_end())
            .replace("metadata:\n", "metadata:\n  future_meta_key: keep-me-too\n");
        std::fs::write(&project_file, patched).unwrap();

        let mut reloaded = read_project(&project_path).unwrap();
        assert_eq!(
            reloaded
                .metadata
                .extra
                .get("future_meta_key")
                .and_then(|v| v.as_str()),
            Some("keep-me-too"),
            "metadata-block unknown key must load into the model's extras"
        );
        write_project(&mut reloaded, &token).unwrap();

        let rewritten = std::fs::read_to_string(&project_file).unwrap();
        assert!(
            rewritten.contains("future_top_level: keep-me"),
            "unknown top-level project.yaml key must survive rewrite:\n{rewritten}"
        );
        assert!(
            rewritten.contains("future_meta_key: keep-me-too"),
            "unknown metadata-block key must survive rewrite:\n{rewritten}"
        );
    }

    #[test]
    fn test_unknown_thread_entry_keys_survive_round_trip() {
        // I5 for threads.yaml: unknown keys on a thread entry survive.
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("ThreadExtra.chikn");
        let mut project = create_project(&project_path, "ThreadExtra").unwrap();
        let token = test_token(&project_path);
        project.threads = vec![crate::models::Thread {
            id: "main-plot".into(),
            name: "Main Plot".into(),
            color: None,
            description: None,
            extra: Default::default(),
        }];
        write_project(&mut project, &token).unwrap();

        let threads_path = project_path.join("threads.yaml");
        let existing = std::fs::read_to_string(&threads_path).unwrap();
        std::fs::write(
            &threads_path,
            format!("{}\n  arc_stage: rising-action\n", existing.trim_end()),
        )
        .unwrap();

        let mut reloaded = read_project(&project_path).unwrap();
        write_project(&mut reloaded, &token).unwrap();

        let rewritten = std::fs::read_to_string(&threads_path).unwrap();
        assert!(
            rewritten.contains("arc_stage: rising-action"),
            "unknown thread-entry key must survive rewrite:\n{rewritten}"
        );
    }

    #[test]
    fn test_project_yaml_stamped_with_format_version() {
        // The writer stamps the format version it speaks; new projects carry
        // the marker from their very first project.yaml.
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("Versioned.chikn");
        create_project(&project_path, "Versioned").unwrap();

        let yaml: serde_yaml::Value = serde_yaml::from_str(
            &std::fs::read_to_string(project_path.join("project.yaml")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            yaml.get("format_version").and_then(|v| v.as_str()),
            Some(crate::core::project::format::FORMAT_VERSION),
            "new project.yaml must carry the format version marker"
        );
    }

    #[test]
    fn test_versionless_project_loads_and_gains_version_on_save() {
        // Projects written before the marker existed load fine (never a
        // gate) and pick up the marker on their next save. A file claiming
        // a future version also loads — preservation means an older engine
        // doesn't destroy what it doesn't understand.
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("Legacy.chikn");
        let mut project = create_project(&project_path, "Legacy").unwrap();
        let token = test_token(&project_path);
        write_project(&mut project, &token).unwrap();

        // Simulate a pre-marker project.yaml.
        let project_file = project_path.join("project.yaml");
        let stripped: String = std::fs::read_to_string(&project_file)
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with("format_version:"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&project_file, format!("{stripped}\n")).unwrap();

        let mut reloaded = read_project(&project_path).expect("version-less project must load");
        write_project(&mut reloaded, &token).unwrap();
        let yaml: serde_yaml::Value =
            serde_yaml::from_str(&std::fs::read_to_string(&project_file).unwrap()).unwrap();
        assert_eq!(
            yaml.get("format_version").and_then(|v| v.as_str()),
            Some(crate::core::project::format::FORMAT_VERSION),
            "version-less project must gain the marker on save"
        );

        // Future-version tolerance: replace the marker and reload.
        let future = std::fs::read_to_string(&project_file).unwrap().replace(
            &format!(
                "format_version: '{}'",
                crate::core::project::format::FORMAT_VERSION
            ),
            "format_version: '9.9'",
        );
        std::fs::write(&project_file, future).unwrap();
        let loaded = read_project(&project_path);
        assert!(
            loaded.is_ok(),
            "future format_version must never gate reads: {loaded:?}"
        );
    }

    #[test]
    fn test_fields_serialize_in_canonical_sorted_order() {
        // Sidecars must have one canonical byte form: fields keys serialize
        // sorted regardless of insertion order, so the embedded git history
        // never records spurious reorder diffs.
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("Canonical.chikn");
        let mut project = create_project(&project_path, "Canonical").unwrap();
        let token = test_token(&project_path);

        let mut doc = Document {
            id: "doc1".to_string(),
            name: "canon".to_string(),
            path: "manuscript/canon.md".to_string(),
            content: "Text.".to_string(),
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };
        for key in ["zeta", "alpha", "mike", "bravo", "yankee", "charlie"] {
            doc.fields
                .insert(key.to_string(), serde_yaml::Value::String("v".into()));
        }
        project.documents.insert(doc.id.clone(), doc.clone());
        project.hierarchy.push(TreeNode::Document {
            id: doc.id.clone(),
            name: doc.name.clone(),
            path: doc.path.clone(),
        });
        write_project(&mut project, &token).unwrap();

        let meta = std::fs::read_to_string(project_path.join("manuscript/canon.meta")).unwrap();
        let field_keys: Vec<String> = meta
            .lines()
            .skip_while(|l| !l.starts_with("fields:"))
            .skip(1)
            .take_while(|l| l.starts_with("  "))
            .filter_map(|l| l.trim().split(':').next().map(str::to_string))
            .collect();
        assert_eq!(
            field_keys,
            vec!["alpha", "bravo", "charlie", "mike", "yankee", "zeta"],
            "fields keys must serialize in sorted order:\n{meta}"
        );
    }

    #[test]
    fn test_legacy_novelist_keys_lift_into_fields() {
        // Sidecars written during the 10ec683 window carry novelist keys at
        // the .meta TOP LEVEL. They must load into `fields` (where the UIs
        // look) and relocate under `fields:` on the next save.
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("Legacy.chikn");
        let mut project = create_project(&project_path, "Legacy").unwrap();
        let token = test_token(&project_path);

        let doc = Document {
            id: "scene-1".to_string(),
            name: "scene-one".to_string(),
            path: "manuscript/scene-one.md".to_string(),
            content: "Sarah waited.".to_string(),
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };
        project.documents.insert(doc.id.clone(), doc.clone());
        project.hierarchy.push(TreeNode::Document {
            id: doc.id.clone(),
            name: doc.name.clone(),
            path: doc.path.clone(),
        });
        write_project(&mut project, &token).unwrap();

        let meta_path = project_path.join("manuscript/scene-one.meta");
        let existing = std::fs::read_to_string(&meta_path).unwrap();
        std::fs::write(
            &meta_path,
            format!(
                "{}\npov_character: sarah\nduration_minutes: 90\nthreads:\n- main-plot\n",
                existing.trim_end()
            ),
        )
        .unwrap();

        // Read: values must surface in `fields`.
        let mut reloaded = read_project(&project_path).unwrap();
        let d = reloaded.documents.get("scene-1").unwrap();
        assert_eq!(
            d.fields.get("pov_character").and_then(|v| v.as_str()),
            Some("sarah"),
            "legacy string key must lift into fields on read"
        );
        assert_eq!(
            d.fields.get("duration_minutes").and_then(|v| v.as_i64()),
            Some(90),
            "legacy numeric key must lift into fields on read"
        );

        // Write: keys must relocate under fields:, not duplicate at top level.
        write_project(&mut reloaded, &token).unwrap();
        let rewritten = std::fs::read_to_string(&meta_path).unwrap();
        assert!(
            !rewritten
                .lines()
                .any(|l| l.starts_with("pov_character:") || l.starts_with("duration_minutes:")),
            "legacy keys must not remain at .meta top level:\n{rewritten}"
        );
        let final_load = read_project(&project_path).unwrap();
        let d = final_load.documents.get("scene-1").unwrap();
        assert_eq!(
            d.fields.get("pov_character").and_then(|v| v.as_str()),
            Some("sarah")
        );
        assert_eq!(
            d.fields
                .get("threads")
                .and_then(|v| v.as_sequence())
                .map(|s| s.len()),
            Some(1),
            "legacy list key must survive the relocation"
        );
    }

    #[test]
    fn test_legacy_lift_fields_wins_on_conflict() {
        // When the same key exists both under fields: and at the legacy top
        // level, the fields: value is authoritative; the stale duplicate is
        // dropped rather than resurrected.
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("LegacyConflict.chikn");
        let mut project = create_project(&project_path, "LegacyConflict").unwrap();
        let token = test_token(&project_path);

        let doc = Document {
            id: "scene-1".to_string(),
            name: "scene-one".to_string(),
            path: "manuscript/scene-one.md".to_string(),
            content: "Text.".to_string(),
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };
        project.documents.insert(doc.id.clone(), doc.clone());
        project.hierarchy.push(TreeNode::Document {
            id: doc.id.clone(),
            name: doc.name.clone(),
            path: doc.path.clone(),
        });
        write_project(&mut project, &token).unwrap();

        let meta_path = project_path.join("manuscript/scene-one.meta");
        let existing = std::fs::read_to_string(&meta_path).unwrap();
        std::fs::write(
            &meta_path,
            format!(
                "{}\npov_character: old-sarah\nfields:\n  pov_character: new-sarah\n",
                existing.trim_end()
            ),
        )
        .unwrap();

        let mut reloaded = read_project(&project_path).unwrap();
        let d = reloaded.documents.get("scene-1").unwrap();
        assert_eq!(
            d.fields.get("pov_character").and_then(|v| v.as_str()),
            Some("new-sarah"),
            "fields: value must win over the legacy top-level duplicate"
        );

        write_project(&mut reloaded, &token).unwrap();
        let rewritten = std::fs::read_to_string(&meta_path).unwrap();
        assert!(
            !rewritten.lines().any(|l| l.starts_with("pov_character:")),
            "stale top-level duplicate must be gone after save:\n{rewritten}"
        );
        assert!(
            rewritten.contains("pov_character: new-sarah"),
            "authoritative fields: value must persist:\n{rewritten}"
        );
    }

    #[test]
    fn test_corrupt_existing_project_yaml_aborts_write() {
        // Overwriting a project.yaml we cannot parse would destroy whatever
        // it held (I6): the save must abort before touching the file.
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("CorruptProj.chikn");
        let mut project = create_project(&project_path, "CorruptProj").unwrap();
        let token = test_token(&project_path);
        write_project(&mut project, &token).unwrap();

        let project_file = project_path.join("project.yaml");
        let corrupt = "id: [unclosed\n";
        std::fs::write(&project_file, corrupt).unwrap();

        let result = write_project(&mut project, &token);
        assert!(result.is_err(), "write over corrupt project.yaml must fail");
        assert_eq!(
            std::fs::read_to_string(&project_file).unwrap(),
            corrupt,
            "corrupt project.yaml must be left untouched by the failed save"
        );
    }

    #[test]
    fn test_delete_document() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("DeleteTest.chikn");

        let mut project = create_project(&project_path, "Delete Test").unwrap();
        let token = test_token(&project_path);

        // Create a document
        let doc = Document {
            id: "doc1".to_string(),
            name: "to-delete".to_string(),
            path: "manuscript/to-delete.md".to_string(),
            content: "This will be deleted".to_string(),
            parent_id: None,
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };

        project.documents.insert(doc.id.clone(), doc.clone());
        write_project(&mut project, &token).unwrap();

        // Verify file exists
        let content_path = get_manuscript_path(&project_path).join("to-delete.md");
        assert!(content_path.exists());

        // Delete document using its path
        delete_document(&project_path, "manuscript/to-delete.md", &token).unwrap();

        // Verify files are gone
        assert!(!content_path.exists());
        let meta_path = get_manuscript_path(&project_path).join("to-delete.meta");
        assert!(!meta_path.exists());
    }

    #[test]
    fn test_write_project_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("MetadataTest.chikn");

        let project = create_project(&project_path, "Metadata Test").unwrap();
        let result = write_project_metadata(&project);

        assert!(result.is_ok());

        let project_file = get_project_file_path(&project_path);
        assert!(project_file.exists());

        // Verify YAML is valid
        let content = fs::read_to_string(&project_file).unwrap();
        assert!(content.contains("name:") && content.contains("Metadata Test"));
    }

    #[test]
    fn test_write_nested_document() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("NestedWrite.chikn");

        let mut project = create_project(&project_path, "Nested Write Test").unwrap();
        let token = test_token(&project_path);

        // Create document with nested path
        let doc = Document {
            id: "nested1".to_string(),
            name: "Chapter 1".to_string(),
            path: "manuscript/part-one/chapter-01.md".to_string(),
            content: "# Nested Chapter\n\nContent".to_string(),
            parent_id: None,
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };

        project.documents.insert(doc.id.clone(), doc.clone());
        write_project(&mut project, &token).unwrap();

        // Verify nested file exists
        let nested_path = project_path
            .join("manuscript")
            .join("part-one")
            .join("chapter-01.md");
        assert!(nested_path.exists());

        // Verify content
        let content = fs::read_to_string(&nested_path).unwrap();
        assert_eq!(content, "# Nested Chapter\n\nContent");

        // Verify metadata file
        let meta_path = project_path
            .join("manuscript")
            .join("part-one")
            .join("chapter-01.meta");
        assert!(meta_path.exists());
    }

    #[test]
    fn test_write_research_document() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("ResearchWrite.chikn");

        let mut project = create_project(&project_path, "Research Test").unwrap();
        let token = test_token(&project_path);

        // Create document in research folder
        let doc = Document {
            id: "research1".to_string(),
            name: "Character Notes".to_string(),
            path: "research/characters.md".to_string(),
            content: "# Characters\n\nJohn Doe".to_string(),
            parent_id: None,
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };

        project.documents.insert(doc.id.clone(), doc);
        write_project(&mut project, &token).unwrap();

        // Verify file in research folder
        let research_path = project_path.join("research").join("characters.md");
        assert!(research_path.exists());
    }

    #[test]
    fn test_write_document_rejects_absolute_path() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("SecurityTest.chikn");

        let mut project = create_project(&project_path, "Security Test").unwrap();
        let token = test_token(&project_path);

        // Try to create document with absolute path
        let doc = Document {
            id: "bad1".to_string(),
            name: "Bad Document".to_string(),
            path: "/etc/passwd".to_string(), // Absolute path - security issue!
            content: "Evil content".to_string(),
            parent_id: None,
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };

        project.documents.insert(doc.id.clone(), doc);
        let result = write_project(&mut project, &token);

        // Should fail with InvalidFormat error
        assert!(result.is_err());
    }

    #[test]
    fn test_write_document_rejects_parent_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("TraversalTest.chikn");

        let mut project = create_project(&project_path, "Traversal Test").unwrap();
        let token = test_token(&project_path);

        // Try to escape project directory
        let doc = Document {
            id: "bad2".to_string(),
            name: "Bad Document".to_string(),
            path: "../../../etc/passwd".to_string(), // Directory traversal!
            content: "Evil content".to_string(),
            parent_id: None,
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };

        project.documents.insert(doc.id.clone(), doc);
        let result = write_project(&mut project, &token);

        // Should fail with InvalidFormat error
        assert!(result.is_err());
    }

    #[test]
    fn test_write_document_allows_dotdot_inside_component() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("DotDotName.chikn");

        let mut project = create_project(&project_path, "Dot Dot Name").unwrap();
        let token = test_token(&project_path);

        let doc = Document {
            id: "doc-dotdot".to_string(),
            name: "chapter..01".to_string(),
            path: "manuscript/chapter..01.md".to_string(),
            content: "Dots in names are allowed.".to_string(),
            parent_id: None,
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };

        project.documents.insert(doc.id.clone(), doc);
        write_project(&mut project, &token).unwrap();

        assert!(project_path.join("manuscript/chapter..01.md").exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_write_document_rejects_symlink_parent() {
        use std::os::unix::fs as unix_fs;

        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("SymlinkParent.chikn");
        let outside_path = temp_dir.path().join("outside");
        fs::create_dir(&outside_path).unwrap();

        let mut project = create_project(&project_path, "Symlink Parent").unwrap();
        let token = test_token(&project_path);
        unix_fs::symlink(&outside_path, project_path.join("manuscript/link")).unwrap();

        let doc = Document {
            id: "bad-link-parent".to_string(),
            name: "pwned".to_string(),
            path: "manuscript/link/pwned.md".to_string(),
            content: "must not escape".to_string(),
            parent_id: None,
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };

        project.documents.insert(doc.id.clone(), doc);
        let result = write_project(&mut project, &token);

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
        assert!(!outside_path.join("pwned.md").exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_write_document_rejects_symlink_file_target() {
        use std::os::unix::fs as unix_fs;

        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("SymlinkTarget.chikn");
        let outside_file = temp_dir.path().join("outside.md");
        fs::write(&outside_file, "original").unwrap();

        let mut project = create_project(&project_path, "Symlink Target").unwrap();
        let token = test_token(&project_path);
        unix_fs::symlink(&outside_file, project_path.join("manuscript/linked.md")).unwrap();

        let doc = Document {
            id: "bad-link-file".to_string(),
            name: "linked".to_string(),
            path: "manuscript/linked.md".to_string(),
            content: "must not overwrite outside".to_string(),
            parent_id: None,
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };

        project.documents.insert(doc.id.clone(), doc);
        let result = write_project(&mut project, &token);

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
        assert_eq!(fs::read_to_string(&outside_file).unwrap(), "original");
    }

    #[cfg(unix)]
    #[test]
    fn test_delete_document_rejects_symlink_parent() {
        use std::os::unix::fs as unix_fs;

        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("DeleteSymlinkParent.chikn");
        let outside_path = temp_dir.path().join("outside-delete");
        fs::create_dir(&outside_path).unwrap();
        let outside_file = outside_path.join("victim.md");
        fs::write(&outside_file, "do not delete").unwrap();

        create_project(&project_path, "Delete Symlink Parent").unwrap();
        let token = test_token(&project_path);
        unix_fs::symlink(&outside_path, project_path.join("manuscript/link")).unwrap();

        let result = delete_document(&project_path, "manuscript/link/victim.md", &token);

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
        assert!(outside_file.exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_delete_document_rejects_symlink_file_target() {
        use std::os::unix::fs as unix_fs;

        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("DeleteSymlinkTarget.chikn");
        let outside_file = temp_dir.path().join("outside.md");
        fs::write(&outside_file, "do not delete").unwrap();

        create_project(&project_path, "Delete Symlink Target").unwrap();
        let token = test_token(&project_path);
        unix_fs::symlink(&outside_file, project_path.join("manuscript/linked.md")).unwrap();

        let result = delete_document(&project_path, "manuscript/linked.md", &token);

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
        assert!(outside_file.exists());
        assert!(
            fs::symlink_metadata(project_path.join("manuscript/linked.md"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn test_delete_nested_document() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("DeleteNested.chikn");

        let mut project = create_project(&project_path, "Delete Nested Test").unwrap();
        let token = test_token(&project_path);

        // Create nested document
        let doc = Document {
            id: "nested1".to_string(),
            name: "Nested Chapter".to_string(),
            path: "manuscript/part-one/chapter.md".to_string(),
            content: "Content".to_string(),
            parent_id: None,
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };

        project.documents.insert(doc.id.clone(), doc.clone());
        write_project(&mut project, &token).unwrap();

        // Verify it exists
        let nested_path = project_path
            .join("manuscript")
            .join("part-one")
            .join("chapter.md");
        assert!(nested_path.exists());

        // Delete it
        delete_document(&project_path, "manuscript/part-one/chapter.md", &token).unwrap();

        // Verify it's gone
        assert!(!nested_path.exists());
    }

    #[test]
    fn test_modified_timestamp_updates() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("TimestampTest.chikn");

        let mut project = create_project(&project_path, "Timestamp Test").unwrap();
        let token = test_token(&project_path);
        let original_modified = project.modified.clone();

        // Wait a bit to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Write project
        write_project(&mut project, &token).unwrap();

        // Verify modified timestamp was updated
        assert_ne!(project.modified, original_modified);
    }

    #[test]
    fn test_write_preserves_document_modified() {
        // Regression: `write_project` used to stamp every doc's .meta with
        // `Utc::now()`, so renaming or moving any node bumped the modified
        // timestamp on every other document. Verify that an unrelated
        // write leaves `Document.modified` untouched.
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("ModifiedPreserve.chikn");

        let mut project = create_project(&project_path, "Modified Preserve").unwrap();
        let token = test_token(&project_path);
        let frozen = "2020-01-01T00:00:00Z".to_string();
        let doc = Document {
            id: "doc1".to_string(),
            name: "Stable".to_string(),
            path: "manuscript/stable.md".to_string(),
            content: "frozen".to_string(),
            parent_id: None,
            created: frozen.clone(),
            modified: frozen.clone(),
            ..Default::default()
        };
        project.documents.insert(doc.id.clone(), doc);
        write_project(&mut project, &token).unwrap();

        // Reload and confirm the writer kept the historical timestamp.
        let reloaded = read_project(&project_path).unwrap();
        let stable = reloaded.documents.get("doc1").unwrap();
        assert_eq!(stable.modified, frozen);
    }

    #[test]
    fn test_write_project_rejects_corrupt_existing_document_meta() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("CorruptMeta.chikn");

        let mut project = create_project(&project_path, "Corrupt Meta").unwrap();
        let token = test_token(&project_path);
        let doc = Document {
            id: "doc1".to_string(),
            name: "Stable".to_string(),
            path: "manuscript/stable.md".to_string(),
            content: "initial".to_string(),
            parent_id: None,
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };
        project.documents.insert(doc.id.clone(), doc);
        write_project(&mut project, &token).unwrap();

        let meta_path = get_manuscript_path(&project_path).join("stable.meta");
        let content_path = get_manuscript_path(&project_path).join("stable.md");
        fs::write(&meta_path, "id: [").unwrap();
        let project_yaml_before = fs::read_to_string(project_path.join("project.yaml")).unwrap();
        let content_before = fs::read_to_string(&content_path).unwrap();

        project
            .documents
            .get_mut("doc1")
            .unwrap()
            .content
            .push_str("\nupdated");

        let result = write_project(&mut project, &token);

        assert!(matches!(result, Err(ChiknError::InvalidFormat(_))));
        assert_eq!(fs::read_to_string(&meta_path).unwrap(), "id: [");
        assert_eq!(fs::read_to_string(&content_path).unwrap(), content_before);
        assert_eq!(
            fs::read_to_string(project_path.join("project.yaml")).unwrap(),
            project_yaml_before
        );
    }

    #[test]
    fn test_atomic_write_file_replaces_existing_file_and_removes_temp() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("chapter.md");
        fs::write(&target, "old complete content").unwrap();

        atomic_write_file(&target, b"new complete content").unwrap();

        assert_eq!(fs::read_to_string(&target).unwrap(), "new complete content");
        assert_no_atomic_temp_files(temp_dir.path());
    }

    #[test]
    fn test_atomic_write_file_removes_temp_after_replace_failure() {
        let temp_dir = TempDir::new().unwrap();
        let directory_target = temp_dir.path().join("chapter.md");
        fs::create_dir(&directory_target).unwrap();

        let result = atomic_write_file(&directory_target, b"content");

        assert!(matches!(result, Err(ChiknError::Io(_))));
        assert!(directory_target.is_dir());
        assert_no_atomic_temp_files(temp_dir.path());
    }

    #[test]
    fn test_write_document_leaves_no_atomic_temp_files_after_success() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("AtomicTempCleanup.chikn");

        let mut project = create_project(&project_path, "Atomic Temp Cleanup").unwrap();
        let token = test_token(&project_path);
        let doc = Document {
            id: "doc1".to_string(),
            name: "Stable".to_string(),
            path: "manuscript/stable.md".to_string(),
            content: "initial".to_string(),
            parent_id: None,
            created: Utc::now().to_rfc3339(),
            modified: Utc::now().to_rfc3339(),
            ..Default::default()
        };
        project.documents.insert(doc.id.clone(), doc);
        write_project(&mut project, &token).unwrap();

        project
            .documents
            .get_mut("doc1")
            .unwrap()
            .content
            .push_str("\nupdated");
        write_project(&mut project, &token).unwrap();

        assert_no_atomic_temp_files(&project_path);
        assert_no_atomic_temp_files(&get_manuscript_path(&project_path));
    }

    fn assert_no_atomic_temp_files(directory: &Path) {
        let leftovers: Vec<_> = fs::read_dir(directory)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with('.') && name.contains(".tmp-"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "unexpected atomic temp files in {}: {leftovers:?}",
            directory.display()
        );
    }

    #[test]
    fn test_emptying_threads_removes_file() {
        // Regression: `write_threads_if_any` used to no-op on an empty
        // thread list, leaving stale `threads.yaml` content on disk.
        // Deleting the last thread should actually persist.
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("EmptyThreads.chikn");
        let mut project = create_project(&project_path, "Empty Threads").unwrap();
        let token = test_token(&project_path);

        project.threads = vec![crate::models::Thread {
            id: "main".into(),
            name: "Main Plot".into(),
            color: None,
            description: None,
            extra: Default::default(),
        }];
        write_project(&mut project, &token).unwrap();
        let threads_path = project_path.join("threads.yaml");
        assert!(
            threads_path.exists(),
            "threads.yaml written when threads present"
        );

        project.threads.clear();
        write_project(&mut project, &token).unwrap();
        assert!(
            !threads_path.exists(),
            "threads.yaml removed when project drops to zero threads"
        );
    }
}
